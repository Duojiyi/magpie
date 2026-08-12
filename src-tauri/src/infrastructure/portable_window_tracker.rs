//! Foreground-application detection for macOS and Linux.
//!
//! Windows resolves the app that produced a clipboard entry through `GetClipboardOwner` /
//! `GetForegroundWindow`. Off Windows this was a stub returning the literal `"FallbackApp"`
//! for every entry, which silently disabled everything keyed on provenance: the source icon,
//! per-application cleanup rules, and the "is this Office / a spreadsheet / a screenshot tool"
//! heuristics that decide how aggressively to probe for rich text.
//!
//! Neither platform has a "who owns the clipboard" API, so the frontmost application at
//! capture time is the best available proxy — the same fallback Windows uses when
//! `GetClipboardOwner` returns nothing.

/// Application name plus executable path, mirroring the Windows `ActiveAppInfo` shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForegroundApp {
    pub app_name: String,
    pub process_path: Option<String>,
}

/// Name shown when the platform cannot tell us who is in front.
pub const UNKNOWN_APP: &str = "FallbackApp";

impl ForegroundApp {
    fn unknown() -> Self {
        Self {
            app_name: UNKNOWN_APP.to_string(),
            process_path: None,
        }
    }
}

/// Derive a display name from an executable path when the platform gives us no better one.
/// `/usr/bin/gnome-text-editor` becomes `gnome-text-editor`.
pub fn app_name_from_path(path: &str) -> Option<String> {
    let name = path.rsplit(['/', '\\']).next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn frontmost_app() -> ForegroundApp {
    use objc2_app_kit::NSWorkspace;

    // `frontmostApplication` is a plain property read on a shared singleton and is safe to
    // call from the clipboard thread; it does not require the main thread.
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let Some(app) = (unsafe { workspace.frontmostApplication() }) else {
        return ForegroundApp::unknown();
    };

    let process_path = unsafe { app.bundleURL() }
        .and_then(|url| unsafe { url.path() })
        .map(|path| path.to_string());
    let app_name = unsafe { app.localizedName() }
        .map(|name| name.to_string())
        .or_else(|| process_path.as_deref().and_then(app_name_from_path))
        .unwrap_or_else(|| UNKNOWN_APP.to_string());

    ForegroundApp {
        app_name,
        process_path,
    }
}

#[cfg(target_os = "linux")]
pub fn frontmost_app() -> ForegroundApp {
    linux_frontmost_app().unwrap_or_else(ForegroundApp::unknown)
}

/// Read `_NET_ACTIVE_WINDOW` from the root window, then that window's `_NET_WM_PID`, then
/// resolve the process through `/proc`. Every step is optional: window managers are not
/// obliged to set either property, and the window may belong to a remote client with no
/// local pid, in which case we fall back to "unknown" rather than guessing.
#[cfg(target_os = "linux")]
fn linux_frontmost_app() -> Option<ForegroundApp> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen_num)?.root;

    let active_atom = conn.intern_atom(true, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
    if active_atom == 0 {
        return None;
    }
    let active = conn
        .get_property(false, root, active_atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    if active == 0 {
        return None;
    }

    let pid_atom = conn.intern_atom(true, b"_NET_WM_PID").ok()?.reply().ok()?.atom;
    if pid_atom == 0 {
        return None;
    }
    let pid = conn
        .get_property(false, active, pid_atom, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    if pid == 0 {
        return None;
    }

    let process_path = std::fs::read_link(format!("/proc/{}/exe", pid))
        .ok()
        .map(|p| p.to_string_lossy().into_owned());
    let app_name = std::fs::read_to_string(format!("/proc/{}/comm", pid))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| process_path.as_deref().and_then(app_name_from_path))?;

    Some(ForegroundApp {
        app_name,
        process_path,
    })
}

/// Windows builds never call this; the real implementation lives in `windows_api`.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn frontmost_app() -> ForegroundApp {
    ForegroundApp::unknown()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_a_name_from_an_executable_path() {
        assert_eq!(
            app_name_from_path("/usr/bin/gnome-text-editor").as_deref(),
            Some("gnome-text-editor")
        );
        assert_eq!(
            app_name_from_path("/Applications/Safari.app").as_deref(),
            Some("Safari.app")
        );
        assert_eq!(app_name_from_path(""), None);
        assert_eq!(app_name_from_path("/"), None);
    }

    #[test]
    fn unknown_app_matches_the_name_the_pipeline_treats_as_absent() {
        // Downstream heuristics compare against this literal; drifting from it would make
        // them treat "unknown source" as a real application name.
        assert_eq!(ForegroundApp::unknown().app_name, "FallbackApp");
        assert_eq!(ForegroundApp::unknown().process_path, None);
    }
}
