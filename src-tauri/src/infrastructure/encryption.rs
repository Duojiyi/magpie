use base64::Engine;

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
type BOOL = i32;
#[cfg(windows)]
type DWORD = u32;

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct DATA_BLOB {
    cbData: DWORD,
    pbData: *mut u8,
}

#[cfg(windows)]
const CRYPTPROTECT_UI_FORBIDDEN: DWORD = 0x1;

#[cfg(windows)]
#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        p_data_in: *mut DATA_BLOB,
        sz_data_descr: *const u16,
        p_optional_entropy: *mut DATA_BLOB,
        pv_reserved: *mut c_void,
        p_prompt_struct: *mut c_void,
        dw_flags: DWORD,
        p_data_out: *mut DATA_BLOB,
    ) -> BOOL;

    fn CryptUnprotectData(
        p_data_in: *mut DATA_BLOB,
        ppsz_data_descr: *mut *mut u16,
        p_optional_entropy: *mut DATA_BLOB,
        pv_reserved: *mut c_void,
        p_prompt_struct: *mut c_void,
        dw_flags: DWORD,
        p_data_out: *mut DATA_BLOB,
    ) -> BOOL;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(hmem: *mut c_void) -> *mut c_void;
}

pub const ENCRYPT_PREFIX: &str = "dpapi:";

#[cfg(windows)]
pub fn encrypt_value(plain: &str) -> Option<String> {
    let bytes = plain.as_bytes();
    let mut in_blob = DATA_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut out_blob = DATA_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut in_blob,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok != 0 {
        let out = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
        let encoded = base64::engine::general_purpose::STANDARD.encode(out);
        unsafe {
            let _ = LocalFree(out_blob.pbData as _);
        }
        Some(format!("{}{}", ENCRYPT_PREFIX, encoded))
    } else {
        None
    }
}

#[cfg(windows)]
pub fn decrypt_value(cipher: &str) -> Option<String> {
    let payload = cipher.strip_prefix(ENCRYPT_PREFIX)?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()?;
    let mut in_blob = DATA_BLOB {
        cbData: decoded.len() as u32,
        pbData: decoded.as_ptr() as *mut u8,
    };
    let mut out_blob = DATA_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &mut in_blob,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok != 0 {
        let out = unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) };
        let result = String::from_utf8(out.to_vec()).ok();
        unsafe {
            let _ = LocalFree(out_blob.pbData as _);
        }
        result
    } else {
        None
    }
}

/// Prefix for the portable (non-DPAPI) at-rest encryption used on macOS and Linux.
pub const PORTABLE_PREFIX: &str = "mgpk1:";

/// Does this value look like something an `encrypt_value` produced, on any platform?
pub fn is_encrypted_payload(value: &str) -> bool {
    value.starts_with(ENCRYPT_PREFIX) || value.starts_with(PORTABLE_PREFIX)
}

/// Every at-rest ciphertext prefix, for callers that must express "is encrypted" in SQL.
/// Keep in step with [`is_encrypted_payload`]: a scheme missing here makes SQL disagree with
/// the Rust check, and rows silently drop out of encryption-aware queries.
pub const ENCRYPTED_PREFIXES: [&str; 2] = [ENCRYPT_PREFIX, PORTABLE_PREFIX];

#[cfg(test)]
mod prefix_tests {
    use super::*;

    #[test]
    fn every_prefix_is_recognised_by_the_rust_check() {
        // SQL scans build `LIKE '<prefix>%'` clauses from ENCRYPTED_PREFIXES while Rust uses
        // is_encrypted_payload. If the two disagree, encrypted rows silently drop out of the
        // encryption-aware queries on one platform.
        for prefix in ENCRYPTED_PREFIXES {
            assert!(
                is_encrypted_payload(&format!("{}payload", prefix)),
                "{} must be recognised as ciphertext",
                prefix
            );
        }
        assert!(!is_encrypted_payload("plain value"));
        assert!(!is_encrypted_payload(""));
    }

    #[test]
    fn prefixes_are_distinct_so_schemes_cannot_be_confused() {
        assert_ne!(ENCRYPT_PREFIX, PORTABLE_PREFIX);
        assert!(!PORTABLE_PREFIX.starts_with(ENCRYPT_PREFIX));
        assert!(!ENCRYPT_PREFIX.starts_with(PORTABLE_PREFIX));
    }
}

/// Where the portable key file lives. Set once during startup, after the data directory is
/// resolved (the key must sit beside the database so the two travel together).
#[cfg(not(windows))]
static PORTABLE_KEY_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

#[cfg(not(windows))]
pub fn init_portable_key_dir(dir: std::path::PathBuf) {
    let _ = PORTABLE_KEY_DIR.set(dir);
}

#[cfg(windows)]
pub fn init_portable_key_dir(_dir: std::path::PathBuf) {}

/// Load (or create) the 32-byte local key used to encrypt sensitive values at rest.
///
/// macOS and Linux have no DPAPI equivalent, and until now sensitive values — AI provider API
/// keys, MQTT credentials, the end-to-end sync passphrase, entries tagged sensitive — were
/// simply stored in cleartext in the SQLite file.
///
/// Threat model, stated plainly: the key sits next to the database with `0600` permissions.
/// That protects against other users on the machine and against a database file copied
/// somewhere else without its key, which is the same protection DPAPI gives on Windows. It
/// does **not** protect against code already running as this user. Using the OS keychain would
/// raise that bar, at the cost of failing on headless and minimal desktop installs.
#[cfg(not(windows))]
fn portable_key() -> Option<[u8; 32]> {
    use std::io::Read;

    static KEY: std::sync::OnceLock<Option<[u8; 32]>> = std::sync::OnceLock::new();
    *KEY.get_or_init(|| {
        let dir = PORTABLE_KEY_DIR.get()?;
        let path = dir.join("local.key");

        if let Ok(mut file) = std::fs::File::open(&path) {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() && buf.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&buf);
                return Some(key);
            }
            // A short or unreadable key file means existing ciphertext is unrecoverable.
            // Do NOT overwrite it: keep it for manual recovery and report "no key" so callers
            // surface "unreadable" rather than silently re-keying.
            return None;
        }

        // First run on this platform: mint a key. uuid gives us OS randomness without adding
        // a dependency purely for this.
        let mut key = [0u8; 32];
        key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());

        if std::fs::create_dir_all(dir).is_err() {
            return None;
        }

        // Create with the final permissions rather than writing first and tightening after:
        // `fs::write` would briefly leave the key world-readable at 0644, which is exactly the
        // exposure this file is supposed to prevent. `create_new` also means we never clobber
        // a key that appeared concurrently.
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(mut file) => {
                    if file.write_all(&key).is_err() {
                        return None;
                    }
                }
                // Lost a race: another thread/process just created it, so read that one.
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    let existing = std::fs::read(&path).ok()?;
                    if existing.len() != 32 {
                        return None;
                    }
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&existing);
                    return Some(key);
                }
                Err(_) => return None,
            }
        }
        #[cfg(not(unix))]
        {
            if std::fs::write(&path, key).is_err() {
                return None;
            }
        }
        Some(key)
    })
}

#[cfg(not(windows))]
pub fn encrypt_value(plain: &str) -> Option<String> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use chacha20poly1305::{XChaCha20Poly1305, XNonce};

    // No key (never initialised, or the key file is damaged). Callers fall back to storing the
    // value as-is, because refusing to store it would lose user input outright — but that is a
    // silent downgrade of a value the app promised to protect, so say so loudly.
    let Some(key) = portable_key() else {
        crate::error!(
            "[SECURITY] at-rest encryption key unavailable; a sensitive value is being stored \
             unencrypted. Check permissions on local.key in the data folder."
        );
        return None;
    };
    let cipher = XChaCha20Poly1305::new_from_slice(&key).ok()?;

    let mut nonce = [0u8; 24];
    nonce[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    nonce[16..].copy_from_slice(&uuid::Uuid::new_v4().as_bytes()[..8]);

    let sealed = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plain.as_bytes(),
                aad: b"magpie/local-at-rest",
            },
        )
        .ok()?;

    let mut blob = Vec::with_capacity(nonce.len() + sealed.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&sealed);
    Some(format!(
        "{}{}",
        PORTABLE_PREFIX,
        base64::engine::general_purpose::STANDARD.encode(blob)
    ))
}

#[cfg(not(windows))]
pub fn decrypt_value(cipher_text: &str) -> Option<String> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use chacha20poly1305::{XChaCha20Poly1305, XNonce};

    // DPAPI is Windows-only, so a `dpapi:` payload came from a Windows install (a copied data
    // directory, or a synced snapshot) and genuinely cannot be read here. Returning it
    // unchanged used to surface the literal ciphertext to the user as if it were their API
    // key; `None` is the honest answer and callers already treat it as "stored but unreadable".
    if cipher_text.starts_with(ENCRYPT_PREFIX) {
        return None;
    }
    let Some(payload) = cipher_text.strip_prefix(PORTABLE_PREFIX) else {
        return Some(cipher_text.to_string());
    };

    let key = portable_key()?;
    let blob = base64::engine::general_purpose::STANDARD.decode(payload).ok()?;
    if blob.len() < 24 + 16 {
        return None;
    }
    let (nonce, sealed) = blob.split_at(24);
    let plain = XChaCha20Poly1305::new_from_slice(&key)
        .ok()?
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: sealed,
                aad: b"magpie/local-at-rest",
            },
        )
        .ok()?;
    String::from_utf8(plain).ok()
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn windows_ciphertext_is_reported_unreadable_not_echoed() {
        assert_eq!(decrypt_value("dpapi:AQAAANCMnd8BFdERjHoAwE"), None);
    }

    #[test]
    fn plaintext_passes_through_untouched() {
        assert_eq!(decrypt_value("sk-plain-value").as_deref(), Some("sk-plain-value"));
    }

    #[test]
    fn round_trips_through_the_portable_scheme() {
        let dir = std::env::temp_dir().join(format!("magpie-key-{}", uuid::Uuid::new_v4()));
        init_portable_key_dir(dir.clone());

        // Only meaningful when this test owns the OnceLock; skip if another test set it first.
        if PORTABLE_KEY_DIR.get() != Some(&dir) {
            return;
        }

        let sealed = encrypt_value("super secret 秘密").expect("key available");
        assert!(sealed.starts_with(PORTABLE_PREFIX));
        assert!(!sealed.contains("super secret"));
        assert_eq!(
            decrypt_value(&sealed).as_deref(),
            Some("super secret 秘密")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
