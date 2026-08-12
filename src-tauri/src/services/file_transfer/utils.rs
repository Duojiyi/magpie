use crate::services::file_transfer::models::ServerActivityState;
use base64::{engine::general_purpose, Engine as _};
use local_ip_address::list_afinet_netifas;
use std::time::SystemTime;
use tauri::{AppHandle, Manager};

pub fn update_activity(app_handle: &AppHandle) {
    if let Ok(mut guard) = app_handle
        .state::<ServerActivityState>()
        .last_activity
        .lock()
    {
        *guard = Some(SystemTime::now());
    }
}

pub fn get_app_logo_base64(_app: &AppHandle) -> String {
    // Embed the icon directly to ensure it works in Dev and Prod without path issues
    const ICON_BYTES: &[u8] = include_bytes!("../../../icons/icon.png");
    format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(ICON_BYTES)
    )
}

pub fn score_interface(name: &str, ip: &str) -> i32 {
    let mut score = 0;
    if name.contains("wi-fi") || name.contains("wlan") {
        score += 10;
    }
    if name.contains("ethernet") {
        score += 5;
    }
    if ip.starts_with("192.168.") {
        score += 3;
    }
    if ip.starts_with("10.") {
        score += 2;
    }
    score
}

#[tauri::command]
pub fn get_available_ips() -> Vec<String> {
    if let Ok(ifas) = list_afinet_netifas() {
        let mut candidates = Vec::new();
        for (name, ip) in ifas {
            let ip_str = ip.to_string();
            let name_lower = name.to_lowercase();
            if ip.is_loopback() || !ip.is_ipv4() {
                continue;
            }

            // 过滤掉明显的虚拟网卡
            let is_virtual = name_lower.contains("vnet")
                || name_lower.contains("vbox")
                || name_lower.contains("virtual")
                || name_lower.contains("vmnet")
                || name_lower.contains("tailscale")
                || name_lower.contains("zerotier")
                || name_lower.contains("pseudo")
                || name_lower.contains("clash")
                || name_lower.contains("wsl")
                || name_lower.contains("vethernet")
                || name_lower.contains("docker")
                || name_lower.contains("hyper-v")
                || name_lower.contains("radmin");

            if is_virtual {
                continue;
            }

            if ip_str.starts_with("192.168.")
                || ip_str.starts_with("10.")
                || ip_str.starts_with("172.")
            {
                candidates.push((name_lower, ip_str));
            }
        }

        candidates.sort_by(|(name_a, ip_a), (name_b, ip_b)| {
            let score_a = score_interface(name_a, ip_a);
            let score_b = score_interface(name_b, ip_b);
            score_b.cmp(&score_a)
        });

        return candidates.into_iter().map(|(_, ip)| ip).collect();
    }
    vec![]
}

/// Reduce a client-supplied name (multipart `filename`, chunked-upload `file_name`/
/// `upload_id`, ...) to a single safe path component before it is joined onto a
/// filesystem save directory. These values are fully attacker-controlled — anyone on
/// the LAN can hit `/upload` or `/upload_chunk` with e.g. `..\..\..\Startup\evil.bat` —
/// and were previously interpolated into the save path unmodified (arbitrary file
/// write outside the intended directory). `Path::file_name()` already discards any
/// leading directory / `..` / `.` components; we additionally blank out characters
/// that are illegal on Windows/macOS/Linux and trim trailing dots/spaces (invalid on
/// Windows) so the result can never escape `save_dir` or target a different file than
/// the one just written.
pub fn sanitize_upload_filename(raw: &str) -> String {
    let base = std::path::Path::new(raw)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let cleaned: String = base
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();

    let trimmed = cleaned.trim().trim_matches('.').trim();

    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.chars().take(200).collect()
    }
}

static POLL_TOKEN_SEED: std::sync::OnceLock<std::collections::hash_map::RandomState> =
    std::sync::OnceLock::new();

/// Derive a download token for a chat message id that is stable across repeated
/// `/poll` calls (so `SharedFileState` doesn't grow a fresh entry every poll) but not
/// predictable by a client. `RandomState` is seeded once with process-random keys, so
/// this replaces the previous `format!("temp_{}", m.id)`, which let anyone on the LAN
/// enumerate small integers and download every image ever shared through chat on an
/// endpoint (`/download/{token}`) that has no auth of its own.
pub fn stable_poll_token(id: u64) -> String {
    use std::hash::{BuildHasher, Hasher};
    let seed = POLL_TOKEN_SEED.get_or_init(std::collections::hash_map::RandomState::new);
    let mut hasher = seed.build_hasher();
    hasher.write_u64(id);
    format!("temp_{:016x}", hasher.finish())
}

/// Stable, collision-resistant temp-file name component for a chunked upload id.
///
/// The session map is keyed by the *raw* `upload_id`, but the on-disk temp file must not
/// be named via `sanitize_upload_filename`: that function is many-to-one (`"a/b"`→`"b"`,
/// `""`/`"..."`→`"file"`, 200-char truncation), so two different sessions could collide on
/// one `.tmp_*` file and interleave their appended bytes — a LAN peer could even craft an
/// `upload_id` that sanitizes onto a victim's temp file. Hashing the raw id makes a
/// collision between distinct sessions negligibly unlikely (64-bit, random-seeded) and
/// keeps the name free of any path separators. Seeded per process so it is also not
/// externally predictable.
pub fn stable_upload_temp_name(upload_id: &str) -> String {
    use std::hash::{BuildHasher, Hasher};
    let seed = POLL_TOKEN_SEED.get_or_init(std::collections::hash_map::RandomState::new);
    let mut hasher = seed.build_hasher();
    hasher.write(upload_id.as_bytes());
    format!("{:016x}", hasher.finish())
}

pub async fn bind_listener(start_port: u16) -> (tokio::net::TcpListener, u16) {
    let mut port = start_port;
    loop {
        let addr = format!("0.0.0.0:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => return (listener, port),
            Err(_) => {
                // u16::MAX == 65535，再 +1 会溢出；走系统随机端口兜底
                if port == u16::MAX {
                    if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:0").await {
                        let p = listener.local_addr().map(|a| a.port()).unwrap_or(0);
                        return (listener, p);
                    }
                    break;
                }
                port += 1;
            }
        }
    }
    (tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap(), 0)
}

#[cfg(test)]
mod tests {
    use super::{sanitize_upload_filename, stable_poll_token, stable_upload_temp_name};
    use std::path::Path;

    /// The single security invariant every sanitized name must satisfy regardless of
    /// platform: it is one path component that can never escape the directory it is
    /// joined onto. `\` is a separator on Windows but a literal char on Unix, so we
    /// assert the platform-independent properties instead of an exact string.
    fn assert_is_safe_component(sanitized: &str) {
        assert!(!sanitized.is_empty(), "sanitized name must never be empty");
        assert!(
            !sanitized.contains('/') && !sanitized.contains('\\'),
            "sanitized name must not contain any path separator: {sanitized:?}"
        );
        assert_ne!(sanitized, "..", "sanitized name must not be the parent marker");
        // A bare filename joined onto a base dir must keep that base dir as its parent.
        let joined = Path::new("base_dir").join(sanitized);
        assert_eq!(
            joined.parent(),
            Some(Path::new("base_dir")),
            "sanitized name escaped its base directory: {sanitized:?} -> {joined:?}"
        );
    }

    #[test]
    fn strips_forward_slash_traversal_to_final_component() {
        // Forward slash is a separator on every platform, so this is deterministic.
        assert_eq!(sanitize_upload_filename("../../../etc/passwd"), "passwd");
        assert_eq!(sanitize_upload_filename("/etc/cron.d/evil"), "evil");
        assert_eq!(sanitize_upload_filename("a/b/c/report.pdf"), "report.pdf");
    }

    #[test]
    fn neutralizes_windows_startup_folder_traversal() {
        // The concrete attack the fix targets: an attacker on the LAN uploading a name
        // crafted to drop a payload into the Windows Startup folder. On any platform the
        // result must stay a single, in-directory component.
        let evil = r"..\..\..\Users\admin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup\evil.bat";
        let safe = sanitize_upload_filename(evil);
        assert_is_safe_component(&safe);
        assert!(!safe.contains(".."), "traversal markers must not survive: {safe:?}");
    }

    #[test]
    fn keeps_plain_filenames_untouched() {
        assert_eq!(sanitize_upload_filename("normal.png"), "normal.png");
        assert_eq!(sanitize_upload_filename("my document 2024.txt"), "my document 2024.txt");
    }

    #[test]
    fn replaces_characters_illegal_on_windows() {
        // None of these are path separators on any platform, so file_name() keeps the
        // whole string and every reserved char must be blanked to '_'.
        assert_eq!(sanitize_upload_filename(r#"a<b>c:d"e|f?g*h.txt"#), "a_b_c_d_e_f_g_h.txt");
    }

    #[test]
    fn falls_back_to_placeholder_for_empty_or_dot_only_names() {
        for input in ["", ".", "..", "...", "   ", "/", "//"] {
            assert_eq!(
                sanitize_upload_filename(input),
                "file",
                "expected placeholder for input {input:?}"
            );
        }
    }

    #[test]
    fn caps_length_to_200_chars() {
        let long = "a".repeat(500);
        assert!(sanitize_upload_filename(&long).chars().count() <= 200);
    }

    #[test]
    fn poll_token_is_stable_per_id_within_process() {
        assert_eq!(stable_poll_token(42), stable_poll_token(42));
        assert_eq!(stable_poll_token(0), stable_poll_token(0));
    }

    #[test]
    fn poll_token_differs_between_ids() {
        assert_ne!(stable_poll_token(1), stable_poll_token(2));
        assert_ne!(stable_poll_token(100), stable_poll_token(101));
    }

    #[test]
    fn poll_token_is_not_the_enumerable_plain_id() {
        // Regression guard for the fix: the download endpoint token must no longer be the
        // predictable `temp_{id}` that let anyone on the LAN enumerate small integers.
        for id in [0u64, 1, 2, 5, 42, 1000] {
            let token = stable_poll_token(id);
            assert_ne!(token, format!("temp_{id}"), "token must not expose the raw id");
            assert!(token.starts_with("temp_"), "unexpected token shape: {token:?}");
            assert_eq!(token.len(), "temp_".len() + 16, "expected 16 hex digits: {token:?}");
            assert!(
                token["temp_".len()..].chars().all(|c| c.is_ascii_hexdigit()),
                "token tail must be hex: {token:?}"
            );
        }
    }

    #[test]
    fn upload_temp_name_is_stable_and_separator_free() {
        assert_eq!(stable_upload_temp_name("abc"), stable_upload_temp_name("abc"));
        let name = stable_upload_temp_name("a/b\\c");
        assert!(!name.contains('/') && !name.contains('\\'), "temp name has a separator: {name:?}");
        assert_eq!(name.len(), 16);
        assert!(name.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn upload_temp_name_distinct_where_sanitize_collides() {
        // The whole reason for hashing: ids that sanitize_upload_filename maps onto the
        // SAME component must still get DISTINCT temp files so their bytes never interleave.
        assert_eq!(sanitize_upload_filename("a/b"), sanitize_upload_filename("x/b"));
        assert_ne!(
            stable_upload_temp_name("a/b"),
            stable_upload_temp_name("x/b"),
            "distinct upload_ids must not share a temp file"
        );
        assert_ne!(stable_upload_temp_name(""), stable_upload_temp_name("..."));
    }
}
