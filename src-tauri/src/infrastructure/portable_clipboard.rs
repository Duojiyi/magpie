//! Cross-platform clipboard backend for the non-Windows builds.
//!
//! On Windows the app talks to the clipboard through `windows_api::win_clipboard` (raw
//! Win32). On macOS and Linux those entry points used to be no-op stubs that returned
//! `Ok(())`/`None`, which made the app *look* healthy while silently neither capturing nor
//! writing images, files or rich text. This module is the real implementation behind those
//! entry points, built on `clipboard-rs` (NSPasteboard on macOS, X11 selections on Linux).
//!
//! Deliberately compiled on every platform — including Windows, where it is never called —
//! so that the one platform we can build locally still type-checks all of this code and the
//! pure helpers stay unit-testable. That is the direct lesson of two release breakages caused
//! by `#[cfg]`-hidden stub drift.
//!
//! Windows continues to use `win_clipboard` for everything; nothing here changes its paths.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use clipboard_rs::common::RustImage;
use clipboard_rs::{
    Clipboard, ClipboardContent, ClipboardContext, ClipboardHandler, ClipboardWatcher,
    ClipboardWatcherContext,
};

/// Monotonic clipboard-change counter, the non-Windows analogue of Win32's
/// `GetClipboardSequenceNumber`. Bumped by the change watcher (or the polling fallback)
/// exactly once per observed clipboard change. Readers must see a *stable* value between
/// changes — the previous stub incremented on every read, which made the "same sequence, skip
/// re-capture" debounce in the capture pipeline permanently false.
static CHANGE_SEQ: AtomicU32 = AtomicU32::new(1);

pub fn change_sequence() -> u32 {
    CHANGE_SEQ.load(Ordering::Relaxed)
}

pub fn bump_change_sequence() {
    CHANGE_SEQ.fetch_add(1, Ordering::Relaxed);
}

/// Process-wide clipboard connection.
///
/// Must be a singleton. clipboard-rs's X11 backend opens **two** X connections and spawns a
/// selection-owner thread per context, and never closes any of them (no `Drop`). Creating one
/// per call — as this first did — leaks roughly 100 X connections and 50 threads per copy,
/// because the capture pipeline probes many formats; a handful of copies exhausts the X
/// server's client limit and starts breaking *other* applications on the session. The write
/// side additionally needs its connection to stay alive to retain selection ownership, so a
/// long-lived context is also the semantically correct choice.
static CLIPBOARD: OnceLock<Mutex<ClipboardContext>> = OnceLock::new();

fn ctx() -> Result<MutexGuard<'static, ClipboardContext>, String> {
    let cell = match CLIPBOARD.get() {
        Some(cell) => cell,
        None => {
            let created = ClipboardContext::new()
                .map_err(|e| format!("clipboard context unavailable: {}", e))?;
            // Racing initialisers are fine: the loser's context is dropped immediately, and a
            // context that was never used owns no selection.
            let _ = CLIPBOARD.set(Mutex::new(created));
            CLIPBOARD
                .get()
                .ok_or_else(|| "clipboard context unavailable".to_string())?
        }
    };
    // A panicking clipboard call must not permanently disable the clipboard.
    Ok(cell.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Can this process talk to the platform clipboard at all?
pub fn is_available() -> bool {
    ctx().is_ok()
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested on every platform)
// ---------------------------------------------------------------------------

const CF_FRAGMENT_START: &str = "<!--StartFragment-->";
const CF_FRAGMENT_END: &str = "<!--EndFragment-->";

/// Recover the raw HTML fragment from a CF_HTML payload.
///
/// The write path hands us CF_HTML because that is what the (Windows-first) callers build;
/// macOS/Linux pasteboards want the bare HTML. Marker-based first, with a "first tag" fallback
/// so a payload without markers still yields usable HTML instead of leaking header lines.
pub fn extract_fragment_from_cf_html(cf_html: &str) -> String {
    if let Some(start) = cf_html.find(CF_FRAGMENT_START) {
        let after = start + CF_FRAGMENT_START.len();
        if let Some(end_rel) = cf_html[after..].find(CF_FRAGMENT_END) {
            return cf_html[after..after + end_rel].trim().to_string();
        }
    }
    // No markers: strip the Version/StartHTML/... header lines by starting at the first tag.
    match cf_html.find('<') {
        Some(idx) => cf_html[idx..].trim().to_string(),
        None => cf_html.trim().to_string(),
    }
}

/// Wrap a bare HTML fragment in a CF_HTML envelope the existing capture pipeline can parse.
///
/// The offsets are deliberately zeroed: our `parse_cf_html` is documented (and tested) to fall
/// back to the `<!--StartFragment-->` markers whenever the offsets are unusable, and emitting
/// real byte offsets here would just be another thing to get subtly wrong.
pub fn wrap_fragment_as_cf_html(fragment: &str) -> Vec<u8> {
    format!(
        "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n<html><body>{}{}{}</body></html>",
        CF_FRAGMENT_START, fragment, CF_FRAGMENT_END
    )
    .into_bytes()
}

/// Normalize one entry of a file list to a plain filesystem path.
///
/// X11 hands us `text/uri-list` entries (`file:///home/a%20b.png`), macOS plain paths; the
/// rest of the app (and the Windows implementation) deals in plain paths only.
pub fn file_uri_to_path(entry: &str) -> String {
    let trimmed = entry.trim();
    let Some(rest) = trimmed.strip_prefix("file://") else {
        return trimmed.to_string();
    };
    // file://host/path is legal in uri-lists; we only handle localhost forms.
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(idx) if rest[..idx].eq_ignore_ascii_case("localhost") => &rest[idx..],
        _ => rest,
    };
    match urlencoding::decode(path) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => path.to_string(),
    }
}

/// Does this logical (Windows-named) clipboard format ask for HTML?
fn is_html_format_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "html format" || lower == "text/html" || lower == "html"
}

fn is_png_format_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "png" || lower == "image/png"
}

fn is_rtf_format_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "rich text format" || lower == "text/rtf" || lower == "rtf"
}

fn is_gif_format_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "gif" || lower == "animated gif" || lower == "image/gif"
}

/// Platform-native identifier for GIF data: a UTI on macOS, a MIME-typed selection target on
/// X11. Needed because the pipeline asks for GIF by its Windows name.
fn native_gif_format() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "com.compuserve.gif"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "image/gif"
    }
}

/// Formats we already read and write through dedicated paths. Re-preserving them as opaque
/// "named formats" would write the same payload twice and, on paste, fight the typed writes.
fn is_natively_handled_format(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    is_html_format_name(&lower)
        || is_rtf_format_name(&lower)
        || is_png_format_name(&lower)
        || is_gif_format_name(&lower)
        || lower.starts_with("public.")
        || lower.starts_with("text/")
        || lower.starts_with("image/")
        || lower.contains("utf8_string")
        || lower.contains("string")
        // X11 plain-text targets. Without these they look "private", get stored, and then get
        // written back a second time alongside the real text.
        || lower == "text"
        || lower.contains("compound_text")
        || lower.contains("targets")
        || lower.contains("timestamp")
        || lower.contains("multiple")
        || lower.contains("save_targets")
        || lower.contains("uri-list")
        || lower.contains("filenames")
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// RGBA image currently on the clipboard, as `(width, height, rgba_bytes)`.
pub fn get_image_rgba() -> Option<(usize, usize, Vec<u8>)> {
    let ctx = ctx().ok()?;
    let image = ctx.get_image().ok()?;
    if image.is_empty() {
        return None;
    }
    let rgba = image.to_rgba8().ok()?;
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);
    Some((width, height, rgba.into_raw()))
}

pub fn get_files() -> Option<Vec<String>> {
    let ctx = ctx().ok()?;
    let files = ctx.get_files().ok()?;
    if files.is_empty() {
        return None;
    }
    Some(files.iter().map(|f| file_uri_to_path(f)).collect())
}

/// Formats currently offered by the clipboard, cached per clipboard change.
///
/// Reading a selection on X11 costs a round trip with a polling wait, and the capture pipeline
/// probes on the order of thirty format names per copy — the vast majority of which are absent.
/// Asking once which formats exist, and answering the rest from that list, turns those probes
/// into memory lookups. Keyed on the change sequence so it can never serve a stale answer.
static FORMAT_CACHE: Mutex<Option<(u32, Vec<String>)>> = Mutex::new(None);

fn clipboard_offers(format: &str) -> bool {
    let seq = change_sequence();
    let mut cache = FORMAT_CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    if cache.as_ref().map(|(cached, _)| *cached) != Some(seq) {
        let Ok(ctx) = ctx() else {
            return false;
        };
        let formats = ctx.available_formats().unwrap_or_default();
        *cache = Some((seq, formats));
    }

    cache
        .as_ref()
        .map(|(_, formats)| formats.iter().any(|f| f.eq_ignore_ascii_case(format)))
        .unwrap_or(false)
}

/// Read a clipboard format by its *Windows* name, translating to what this platform offers.
/// Formats without a portable equivalent (private OLE containers) return `None`, which callers
/// already treat as "format not present".
pub fn get_raw_format(name: &str) -> Option<Vec<u8>> {
    // Match the name *before* touching the clipboard. The capture pipeline probes ~10 format
    // names per copy and we recognise a few of them; acquiring the connection first would make
    // the majority of calls pure overhead.
    if !is_html_format_name(name)
        && !is_rtf_format_name(name)
        && !is_png_format_name(name)
        && !is_gif_format_name(name)
    {
        return None;
    }
    if is_gif_format_name(name) {
        // Serve the original GIF bytes so animation survives. Without this the pipeline falls
        // through to the still-image path, which re-encodes to PNG and freezes the first frame.
        if !clipboard_offers(native_gif_format()) {
            return None;
        }
        let ctx = ctx().ok()?;
        let raw = ctx.get_buffer(native_gif_format()).ok()?;
        if raw.len() > 6 && (raw.starts_with(b"GIF87a") || raw.starts_with(b"GIF89a")) {
            return Some(raw);
        }
        return None;
    }
    let ctx = ctx().ok()?;
    if is_html_format_name(name) {
        let html = ctx.get_html().ok()?;
        if html.trim().is_empty() {
            return None;
        }
        return Some(wrap_fragment_as_cf_html(&html));
    }
    if is_rtf_format_name(name) {
        let rtf = ctx.get_rich_text().ok()?;
        if rtf.trim().is_empty() {
            return None;
        }
        return Some(rtf.into_bytes());
    }
    if is_png_format_name(name) {
        let image = ctx.get_image().ok()?;
        if image.is_empty() {
            return None;
        }
        let png = image.to_png().ok()?;
        return Some(png.get_bytes().to_vec());
    }
    None
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

pub fn clear() -> Result<(), String> {
    ctx()?.clear().map_err(|e| e.to_string())
}

pub fn set_files(paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() {
        return Err("file list is empty".to_string());
    }
    // clipboard-rs adds the platform's own URI/path framing; plain paths in.
    ctx()?.set_files(paths).map_err(|e| e.to_string())
}

fn rgba_to_rust_image(width: usize, height: usize, rgba: &[u8]) -> Result<clipboard_rs::RustImageData, String> {
    let buffer = image::RgbaImage::from_raw(width as u32, height as u32, rgba.to_vec())
        .ok_or_else(|| "image dimensions do not match buffer".to_string())?;
    Ok(clipboard_rs::RustImageData::from_dynamic_image(
        image::DynamicImage::ImageRgba8(buffer),
    ))
}

pub fn set_image_rgba(width: usize, height: usize, rgba: &[u8]) -> Result<(), String> {
    let image = rgba_to_rust_image(width, height, rgba)?;
    ctx()?.set_image(image).map_err(|e| e.to_string())
}

/// Write text + HTML (+ optionally an image) as one clipboard transaction, mirroring the
/// Windows `set_clipboard_rich_content` contract: the clipboard ends up either fully written
/// or untouched by us, never text-only-when-html-was-requested.
pub fn set_rich_content(
    text: &str,
    cf_html: &str,
    image: Option<(usize, usize, &[u8])>,
    named_formats: &[(String, Vec<u8>)],
) -> Result<(), String> {
    let fragment = extract_fragment_from_cf_html(cf_html);
    let mut contents: Vec<ClipboardContent> = Vec::with_capacity(3 + named_formats.len());
    if let Some((width, height, rgba)) = image {
        contents.push(ClipboardContent::Image(rgba_to_rust_image(
            width, height, rgba,
        )?));
    }
    if !fragment.trim().is_empty() {
        contents.push(ClipboardContent::Html(fragment));
    }
    if !text.is_empty() {
        contents.push(ClipboardContent::Text(text.to_string()));
    }
    // Application-private formats (an Office table's native representation, say) ride along in
    // the *same* write. Writing them afterwards would not work: a second write replaces the
    // selection rather than adding to it, so it would drop the text and HTML we just set.
    for (name, data) in named_formats {
        if !is_natively_handled_format(name) && !data.is_empty() {
            contents.push(ClipboardContent::Other(name.clone(), data.clone()));
        }
    }
    if contents.is_empty() {
        return Err("nothing to write".to_string());
    }
    ctx()?.set(contents).map_err(|e| e.to_string())
}

/// Enumerate application-private clipboard formats worth preserving for a round trip.
///
/// `keep` receives each platform format identifier (a UTI on macOS, a selection target name on
/// X11). Formats we already handle through typed paths are excluded so they are not stored and
/// replayed twice.
pub fn get_named_formats(
    max_formats: usize,
    max_format_bytes: usize,
    max_total_bytes: usize,
    keep: &dyn Fn(&str) -> bool,
) -> Vec<(String, Vec<u8>)> {
    // Reuse the per-change format list rather than asking again.
    let seq = change_sequence();
    let formats = {
        let mut cache = FORMAT_CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if cache.as_ref().map(|(cached, _)| *cached) != Some(seq) {
            let Ok(ctx) = ctx() else {
                return Vec::new();
            };
            *cache = Some((seq, ctx.available_formats().unwrap_or_default()));
        }
        match cache.as_ref() {
            Some((_, formats)) => formats.clone(),
            None => return Vec::new(),
        }
    };

    let Ok(ctx) = ctx() else {
        return Vec::new();
    };
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total = 0usize;
    for name in formats {
        if out.len() >= max_formats {
            break;
        }
        if is_natively_handled_format(&name) || !keep(&name) {
            continue;
        }
        let Ok(data) = ctx.get_buffer(&name) else {
            continue;
        };
        if data.is_empty() || data.len() > max_format_bytes {
            continue;
        }
        if total.saturating_add(data.len()) > max_total_bytes {
            continue;
        }
        total = total.saturating_add(data.len());
        out.push((name, data));
    }
    out
}

// ---------------------------------------------------------------------------
// Change watching
// ---------------------------------------------------------------------------

struct WatcherHandler {
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl ClipboardHandler for WatcherHandler {
    fn on_clipboard_change(&mut self) {
        bump_change_sequence();
        (self.callback)();
    }
}

/// Run the platform's clipboard change watcher on the current thread (blocking).
///
/// Event-driven where the platform allows it: NSPasteboard changeCount on macOS, XFixes
/// selection-owner events on X11 — both of which observe image/file/HTML changes that the old
/// "poll `get_text` every 500 ms" loop was blind to. Returns `Err` when no watcher could be
/// established (e.g. no display server) so the caller can fall back to polling.
/// Linux clipboard change watcher, implemented directly on XFixes.
///
/// Deliberately not clipboard-rs's watcher: that one `.expect()`s its way through connecting,
/// negotiating XFixes, and every `poll_for_event`. Under `panic = "abort"` each of those turns
/// a recoverable condition into "the whole app dies" — including the entirely routine case of
/// the X server closing the connection when the user logs out, where every other application
/// exits cleanly. Owning the loop lets every failure become an ordinary `Err`.
#[cfg(target_os = "linux")]
fn run_x11_change_watcher(callback: &Arc<dyn Fn() + Send + Sync + 'static>) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xfixes::{self, ConnectionExt as XfixesConnectionExt, SelectionEventMask};
    use x11rb::protocol::xproto::ConnectionExt as XprotoConnectionExt;
    use x11rb::protocol::Event;

    let (conn, screen_num) = x11rb::connect(None).map_err(|e| format!("x11 connect: {}", e))?;
    let root = conn
        .setup()
        .roots
        .get(screen_num)
        .ok_or_else(|| "x11: no such screen".to_string())?
        .root;

    // XFixes must be version-negotiated before any of its requests may be used.
    xfixes::query_version(&conn, 5, 0)
        .map_err(|e| format!("x11 xfixes query: {}", e))?
        .reply()
        .map_err(|e| format!("x11 xfixes unavailable: {}", e))?;

    let clipboard = conn
        .intern_atom(false, b"CLIPBOARD")
        .map_err(|e| format!("x11 intern CLIPBOARD: {}", e))?
        .reply()
        .map_err(|e| format!("x11 intern CLIPBOARD: {}", e))?
        .atom;

    xfixes::select_selection_input(
        &conn,
        root,
        clipboard,
        SelectionEventMask::SET_SELECTION_OWNER,
    )
    .map_err(|e| format!("x11 select selection input: {}", e))?
    .check()
    .map_err(|e| format!("x11 select selection input: {}", e))?;

    conn.flush().map_err(|e| format!("x11 flush: {}", e))?;

    loop {
        match conn.wait_for_event() {
            Ok(Event::XfixesSelectionNotify(_)) => {
                bump_change_sequence();
                callback();
            }
            Ok(_) => {}
            // Connection lost (X server shutdown, logout, network X). Report it; the caller
            // retries, and if the server is really gone the process is exiting anyway.
            Err(err) => return Err(format!("x11 connection lost: {}", err)),
        }
    }
}

pub fn run_change_watcher(callback: Arc<dyn Fn() + Send + Sync + 'static>) -> Result<(), String> {
    // Probe first. `ClipboardWatcherContext::new()` only allocates a channel and always
    // succeeds; the X11 backend does its real connecting *inside* `start_watch()` — with
    // `.expect()` on every step. Under `panic = "abort"` that turns "no display server" into
    // "the whole app dies at startup", and it would also make the polling fallback below
    // unreachable. Establishing a normal connection first tells us whether start_watch can
    // safely run.
    // Drop the guard immediately: the watcher's callback re-enters this module to read the
    // clipboard, so holding the connection lock across the watch loop would deadlock on the
    // first change event.
    drop(ctx().map_err(|e| format!("clipboard watcher precondition failed: {}", e))?);

    #[cfg(target_os = "linux")]
    {
        return run_x11_change_watcher(&callback);
    }

    // macOS: clipboard-rs polls NSPasteboard's changeCount and contains no panicking paths,
    // so its watcher is safe to use as-is.
    #[cfg(not(target_os = "linux"))]
    {
        let mut watcher: ClipboardWatcherContext<WatcherHandler> = ClipboardWatcherContext::new()
            .map_err(|e| format!("clipboard watcher unavailable: {}", e))?;
        watcher.add_handler(WatcherHandler { callback });
        // Blocking; only returns if the platform loop stops on its own.
        watcher.start_watch();
        Ok(())
    }
}

/// Fallback change detector for environments where the watcher cannot start. Text-hash based;
/// bumps the sequence number itself so debouncing still works.
///
/// Returns after roughly `budget`, so the caller can retry the (much better) event-driven
/// watcher. The display server often simply is not up yet when an autostarted app launches,
/// and staying in text-only polling for the rest of the session would permanently lose
/// image/file/HTML capture.
pub fn run_polling_watcher_for(
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
    budget: std::time::Duration,
) {
    use std::hash::{Hash, Hasher};

    let deadline = std::time::Instant::now() + budget;
    let mut clipboard = None;
    let mut last_hash = 0u64;
    while std::time::Instant::now() < deadline {
        if clipboard.is_none() {
            // Never unwrap here: with `panic = "abort"` a missing display server would take
            // the whole app down. Keep retrying instead; clipboard capture simply stays off
            // until the environment provides one.
            clipboard = arboard::Clipboard::new().ok();
            if clipboard.is_none() {
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        }
        if let Some(cb) = clipboard.as_mut() {
            if let Ok(text) = cb.get_text() {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                text.hash(&mut hasher);
                let current = hasher.finish();
                if current != last_hash {
                    last_hash = current;
                    bump_change_sequence();
                    callback();
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_html_fragment_round_trips() {
        let fragment = "<p>Hello <b>世界</b></p>";
        let wrapped = wrap_fragment_as_cf_html(fragment);
        let wrapped_str = String::from_utf8(wrapped).expect("cf_html is utf-8");
        assert!(wrapped_str.starts_with("Version:0.9"));
        assert_eq!(extract_fragment_from_cf_html(&wrapped_str), fragment);
    }

    #[test]
    fn fragment_extraction_survives_missing_markers() {
        assert_eq!(
            extract_fragment_from_cf_html(
                "Version:0.9\r\nStartHTML:0000000105\r\n<p>plain</p>"
            ),
            "<p>plain</p>"
        );
        assert_eq!(extract_fragment_from_cf_html("no markup at all"), "no markup at all");
    }

    #[test]
    fn file_uris_become_plain_paths() {
        assert_eq!(
            file_uri_to_path("file:///home/user/a%20b.png"),
            "/home/user/a b.png"
        );
        assert_eq!(
            file_uri_to_path("file://localhost/tmp/x.txt"),
            "/tmp/x.txt"
        );
        // Already-plain paths (macOS) pass through untouched.
        assert_eq!(
            file_uri_to_path("/Users/me/图片.png"),
            "/Users/me/图片.png"
        );
    }

    #[test]
    fn format_name_mapping_matches_the_names_the_pipeline_asks_for() {
        // These are the literal names the capture pipeline probes with; if they stop being
        // recognised the corresponding capture path silently turns off.
        assert!(is_html_format_name("HTML Format"));
        assert!(is_rtf_format_name("Rich Text Format"));
        assert!(is_png_format_name("PNG"));
        assert!(is_png_format_name("image/png"));
        assert!(!is_html_format_name("Rich Text Format"));
        assert!(!is_png_format_name("GIF"));
    }

    #[test]
    fn change_sequence_is_stable_between_bumps() {
        // The whole point vs. the old stub: reading must not advance the counter.
        let before = change_sequence();
        assert_eq!(change_sequence(), before);
        bump_change_sequence();
        assert_eq!(change_sequence(), before + 1);
        assert_eq!(change_sequence(), before + 1);
    }

    #[test]
    fn rgba_size_mismatch_is_rejected_not_panicking() {
        assert!(rgba_to_rust_image(10, 10, &[0u8; 4]).is_err());
    }
}
