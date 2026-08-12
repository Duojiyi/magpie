//! Opt-in end-to-end encryption for cloud-synced clipboard content.
//!
//! # Model
//! A user passphrase plus a per-account random salt (shared across devices via a WebDAV
//! `e2e.json` file) is stretched with **Argon2id** into a 32-byte master key, from which
//! **HKDF-SHA256** derives three subkeys: content encryption, nonce derivation, and blob
//! naming. Individual string fields (`content` / `html_content` / `preview`) are sealed
//! with **XChaCha20-Poly1305** into a self-describing envelope:
//!
//! ```text
//! mgpe2e:1:<base64url_nopad( nonce[24] || ciphertext || tag[16] )>
//! ```
//!
//! # Deterministic nonces (SIV-style)
//! The nonce is `HMAC-SHA256(k_nonce, aad_len || aad || plaintext)[..24]`,
//! i.e. a function of **both** the plaintext and the AAD. Identical (plaintext, AAD) pairs
//! therefore re-encrypt to an identical envelope, which preserves the content-addressed blob
//! dedup/cache the sync layer relies on (a random nonce would force re-uploading everything
//! every sync). This only leaks *equality* of plaintexts — which the cleartext `content_hash`
//! field already exposes — so it is not a new disclosure.
//!
//! The AAD **must** be part of the nonce derivation. Poly1305's one-time key is derived from
//! `(key, nonce)` alone, so encrypting the same plaintext under the same nonce with two
//! different AADs would produce two tags under one one-time key — from which `(r, s)` can be
//! recovered and arbitrary ciphertext forged for that nonce. That situation is reachable
//! whenever identical text is sealed under different AADs: the same string as two items'
//! `preview` (different `content_hash`), or as one item's `content` and its `preview`
//! (different field tag). Binding the AAD into the nonce makes `(plaintext, AAD)` uniquely
//! determine the nonce, so a `(key, nonce)` pair never protects two distinct messages.
//!
//! # AAD
//! The associated data binds a *field role* (`content`, `<type>/html`, `<type>/preview`) and
//! the item's `content_hash`, so a server can neither move a ciphertext onto a different item
//! nor swap an item's `preview` ciphertext into its `content` slot. Both inputs are values the
//! receiver already holds in cleartext, so it can rebuild the AAD exactly.
//!
//! `timestamp` / `deleted_at` are deliberately **not** in the AAD. They are mutable metadata
//! (re-copying the same clipboard entry bumps the timestamp), and because the AAD feeds the
//! nonce, including them would give the same plaintext a fresh nonce on every re-sync —
//! breaking content-addressed blob dedup and leaving orphaned blobs on the server. Keeping
//! the AAD a pure function of the plaintext's identity preserves dedup while still preventing
//! cross-item and cross-field ciphertext transplants.
//!
//! The honest cost: `timestamp` / `deleted_at` travel unauthenticated. A server that retains
//! an old operation can replay that still-valid ciphertext with a raised timestamp to defeat
//! the local tombstone check and resurrect a deleted item. That is an integrity/hygiene
//! weakness, not a confidentiality one — it cannot reveal plaintext or key material.

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Envelope prefix + version. Bump the version if the wire format ever changes.
pub const ENVELOPE_PREFIX: &str = "mgpe2e:1:";
const NONCE_LEN: usize = 24; // XChaCha20 nonce
const TAG_LEN: usize = 16; // Poly1305 tag

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
}

/// Derived key material. Intentionally not `Clone`/`Debug`, and zeroized on drop, to keep
/// key bytes from being copied around or logged.
pub struct CloudCryptoKey {
    content: [u8; 32],
    nonce: [u8; 32],
    name: [u8; 32],
}

impl Drop for CloudCryptoKey {
    fn drop(&mut self) {
        // Best-effort zeroization (no `zeroize` dep); std::ptr::write_volatile per byte
        // so the compiler can't optimize the wipe away.
        for b in self
            .content
            .iter_mut()
            .chain(self.nonce.iter_mut())
            .chain(self.name.iter_mut())
        {
            unsafe { std::ptr::write_volatile(b, 0) };
        }
    }
}

/// `Argon2id(passphrase, salt)` → master key → `HKDF-SHA256` → three domain-separated
/// subkeys. `salt` must be at least 8 bytes (Argon2 requirement); use [`generate_salt_b64`].
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<CloudCryptoKey, String> {
    if passphrase.is_empty() {
        return Err("empty passphrase".to_string());
    }
    if salt.len() < 8 {
        return Err("salt too short".to_string());
    }
    // m = 64 MiB, t = 3, p = 1, 32-byte output — OWASP Argon2id guidance.
    let params = Params::new(64 * 1024, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut master = [0u8; 32];
    argon
        .hash_password_into(passphrase.as_bytes(), salt, &mut master)
        .map_err(|e| e.to_string())?;

    let hk = Hkdf::<Sha256>::new(None, &master);
    let mut key = CloudCryptoKey {
        content: [0u8; 32],
        nonce: [0u8; 32],
        name: [0u8; 32],
    };
    let expanded = (|| {
        hk.expand(b"magpie/e2e/content", &mut key.content)
            .map_err(|e| e.to_string())?;
        hk.expand(b"magpie/e2e/nonce", &mut key.nonce)
            .map_err(|e| e.to_string())?;
        hk.expand(b"magpie/e2e/name", &mut key.name)
            .map_err(|e| e.to_string())
    })();

    // Wipe the master key on every path, including the (unreachable) expand failure.
    for b in master.iter_mut() {
        unsafe { std::ptr::write_volatile(b, 0) };
    }
    expanded?;
    Ok(key)
}

fn deterministic_nonce(key: &CloudCryptoKey, plaintext: &str, aad: &[u8]) -> [u8; NONCE_LEN] {
    // HMAC key length is never invalid for HMAC, so this cannot fail.
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(&key.nonce).expect("HMAC accepts any key length");
    // Length-prefix the AAD so (aad, plaintext) is unambiguous, and bind the AAD into the
    // nonce — see the module docs: without this, the same plaintext under a different AAD
    // would reuse a Poly1305 one-time key.
    mac.update(&(aad.len() as u64).to_le_bytes());
    mac.update(aad);
    mac.update(plaintext.as_bytes());
    let out = mac.finalize().into_bytes();
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&out[..NONCE_LEN]);
    nonce
}

/// Build the AAD binding a sealed field to its field role and its item's identity.
///
/// `field_tag` distinguishes the fields of one item (`"text"`, `"text/html"`,
/// `"text/preview"`), preventing a server from swapping one field's ciphertext into another
/// slot. `content_hash` ties the ciphertext to that specific item. Both are length-prefixed
/// so the encoding is unambiguous.
///
/// Note the item binding is only as strong as `content_hash`, a 64-bit non-cryptographic
/// hash: an attacker who can force a collision could swap ciphertexts between two colliding
/// items of the same type. Treat it as a transplant deterrent, not a strong identity.
pub fn build_aad(field_tag: &str, content_hash: i64) -> Vec<u8> {
    let mut aad = Vec::with_capacity(field_tag.len() + 16);
    aad.extend_from_slice(&(field_tag.len() as u64).to_le_bytes());
    aad.extend_from_slice(field_tag.as_bytes());
    aad.extend_from_slice(&content_hash.to_le_bytes());
    aad
}

/// Seal one cleartext field into an [`ENVELOPE_PREFIX`] envelope.
pub fn encrypt_field(key: &CloudCryptoKey, plaintext: &str, aad: &[u8]) -> Result<String, String> {
    let nonce_bytes = deterministic_nonce(key, plaintext, aad);
    let cipher = XChaCha20Poly1305::new_from_slice(&key.content).map_err(|e| e.to_string())?;
    let ct = cipher
        .encrypt(
            XNonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext.as_bytes(),
                aad,
            },
        )
        .map_err(|_| "encrypt failed".to_string())?;
    let mut buf = Vec::with_capacity(NONCE_LEN + ct.len());
    buf.extend_from_slice(&nonce_bytes);
    buf.extend_from_slice(&ct);
    Ok(format!("{}{}", ENVELOPE_PREFIX, b64().encode(buf)))
}

/// True if `s` looks like one of our envelopes. Callers still handle a decrypt failure as
/// "not really ours / wrong key" (see [`decrypt_field`]), so this is only a fast pre-check.
pub fn is_envelope(s: &str) -> bool {
    // Require a plausibly well-formed payload rather than just the prefix. Genuine user
    // content that happens to start with `mgpe2e:1:` would otherwise be classified as
    // ciphertext by receiving devices, silently dropped and reported as a passphrase
    // mismatch. Checked in O(1): a valid body is at least a 24-byte nonce + 16-byte tag,
    // i.e. 54 base64url characters.
    let Some(body) = s.strip_prefix(ENVELOPE_PREFIX) else {
        return false;
    };
    const MIN_B64_LEN: usize = 54;
    body.len() >= MIN_B64_LEN
        && body.as_bytes()[..MIN_B64_LEN]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
}

/// Open an envelope produced by [`encrypt_field`]. Returns `Err` if `s` is not an envelope,
/// is malformed, or fails authentication (wrong passphrase / tampered ciphertext or AAD).
pub fn decrypt_field(key: &CloudCryptoKey, envelope: &str, aad: &[u8]) -> Result<String, String> {
    let b64part = envelope
        .strip_prefix(ENVELOPE_PREFIX)
        .ok_or_else(|| "not an e2e envelope".to_string())?;
    let raw = b64()
        .decode(b64part.as_bytes())
        .map_err(|_| "malformed envelope base64".to_string())?;
    if raw.len() < NONCE_LEN + TAG_LEN {
        return Err("envelope too short".to_string());
    }
    let (nonce_bytes, ct) = raw.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new_from_slice(&key.content).map_err(|e| e.to_string())?;
    let pt = cipher
        .decrypt(XNonce::from_slice(nonce_bytes), Payload { msg: ct, aad })
        .map_err(|_| "decrypt failed (wrong passphrase or tampered data)".to_string())?;
    String::from_utf8(pt).map_err(|_| "decrypted content is not valid UTF-8".to_string())
}

/// Per-account salt (16 bytes from a v4 UUID, ~122 bits of entropy), base64url-encoded.
/// A salt is not secret; it only needs to be unique per account so the same passphrase
/// yields the same key on every device.
pub fn generate_salt_b64() -> String {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    b64().encode(bytes)
}

/// Decode a salt previously produced by [`generate_salt_b64`].
pub fn decode_salt_b64(s: &str) -> Result<Vec<u8>, String> {
    b64().decode(s.as_bytes())
        .map_err(|_| "malformed salt base64".to_string())
}

/// A verifier confirms a passphrase derives the expected key (so other devices can detect a
/// wrong passphrase, and we can tell "no passphrase set" from "stored passphrase unreadable").
/// It reveals nothing about the passphrase: it's an HMAC of a fixed label under a subkey.
pub fn compute_verifier(key: &CloudCryptoKey) -> String {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(&key.name).expect("HMAC accepts any key length");
    mac.update(b"magpie/e2e/verifier/v1");
    b64().encode(mac.finalize().into_bytes())
}

/// Constant-time-ish check that `key` matches a stored verifier.
pub fn verify(key: &CloudCryptoKey, expected_verifier_b64: &str) -> bool {
    let actual = compute_verifier(key);
    // Length-independent byte compare to avoid early-exit timing on the common-prefix.
    let a = actual.as_bytes();
    let b = expected_verifier_b64.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// NOTE: blob file names are not derived here. The sync layer seals item fields *before*
// hashing them, so a blob's name is `sha256(ciphertext)` — which already denies a server the
// ability to confirm known plaintext by hashing it, while keeping dedup deterministic.

#[cfg(test)]
mod tests {
    use super::*;

    const SALT: &[u8] = b"0123456789abcdef";

    fn key(pass: &str) -> CloudCryptoKey {
        derive_key(pass, SALT).expect("derive")
    }

    #[test]
    fn round_trips_a_field() {
        let k = key("correct horse battery staple");
        let aad = build_aad("text", 42);
        let env = encrypt_field(&k, "hello 世界", &aad).unwrap();
        assert!(is_envelope(&env));
        assert_eq!(decrypt_field(&k, &env, &aad).unwrap(), "hello 世界");
    }

    #[test]
    fn wrong_passphrase_fails() {
        let aad = build_aad("text", 1);
        let env = encrypt_field(&key("right"), "secret", &aad).unwrap();
        assert!(decrypt_field(&key("wrong"), &env, &aad).is_err());
    }

    #[test]
    fn tampered_aad_fails() {
        let k = key("p");
        let aad = build_aad("text", 1);
        let env = encrypt_field(&k, "secret", &aad).unwrap();
        // Same key/envelope but a different item's identity must not authenticate.
        assert!(decrypt_field(&k, &env, &build_aad("text", 2)).is_err());
        // Nor may one field's ciphertext be replayed into another field's slot.
        assert!(decrypt_field(&k, &env, &build_aad("text/preview", 1)).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let k = key("p");
        let aad = build_aad("text", 1);
        let env = encrypt_field(&k, "secret", &aad).unwrap();
        let mut bytes = env.into_bytes();
        *bytes.last_mut().unwrap() ^= 0x01; // flip a bit in the base64 tail
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(decrypt_field(&k, &tampered, &aad).is_err());
    }

    #[test]
    fn deterministic_same_input_same_envelope() {
        let k = key("p");
        let aad = build_aad("text", 1);
        let a = encrypt_field(&k, "same", &aad).unwrap();
        let b = encrypt_field(&k, "same", &aad).unwrap();
        assert_eq!(a, b, "deterministic nonce must yield identical envelopes for dedup");
    }

    #[test]
    fn distinct_plaintext_distinct_envelope() {
        let k = key("p");
        let aad = build_aad("text", 1);
        let a = encrypt_field(&k, "one", &aad).unwrap();
        let b = encrypt_field(&k, "two", &aad).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn envelope_detection_requires_a_well_formed_payload() {
        // A real envelope is recognised...
        let env = encrypt_field(&key("p"), "x", &build_aad("text", 1)).unwrap();
        assert!(is_envelope(&env));
        // ...but user text that merely starts with the prefix is not, so it is sealed and
        // delivered normally instead of being dropped as "undecryptable".
        assert!(!is_envelope("mgpe2e:1:"));
        assert!(!is_envelope("mgpe2e:1:hello world, this is not base64!!!"));
        assert!(!is_envelope("mgpe2e:1:短，且不是 base64"));
    }

    #[test]
    fn non_envelope_is_detected_and_not_decryptable() {
        let k = key("p");
        assert!(!is_envelope("data:image/png;base64,AAAA"));
        assert!(!is_envelope("plain clipboard text"));
        // A plaintext value must be reported as "not an envelope" so callers pass it through.
        assert!(decrypt_field(&k, "plain clipboard text", &[]).is_err());
    }

    #[test]
    fn verifier_matches_same_passphrase_only() {
        let v = compute_verifier(&key("p"));
        assert!(verify(&key("p"), &v));
        assert!(!verify(&key("different"), &v));
    }

    #[test]
    fn same_plaintext_different_aad_uses_a_different_nonce() {
        // Regression guard against Poly1305 one-time-key reuse: identical plaintext sealed
        // under two different AADs (here, the same text as an item's `content` and as another
        // item's field) must not share a nonce. If it did, the two tags would share one
        // one-time key and (r, s) could be recovered to forge authenticated ciphertext.
        let k = key("p");
        let a = encrypt_field(&k, "same", &build_aad("text", 1)).unwrap();
        let b = encrypt_field(&k, "same", &build_aad("text", 2)).unwrap();
        assert_ne!(a, b, "AAD must be bound into the nonce");
        // Same item, different field role: also must not share a nonce.
        let c = encrypt_field(&k, "same", &build_aad("text/preview", 1)).unwrap();
        assert_ne!(a, c, "field role must be bound into the nonce");

        // The nonce is the first 24 bytes of the envelope payload; assert it actually differs.
        let raw_a = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(a.strip_prefix(ENVELOPE_PREFIX).unwrap())
            .unwrap();
        let raw_b = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(b.strip_prefix(ENVELOPE_PREFIX).unwrap())
            .unwrap();
        assert_ne!(raw_a[..24], raw_b[..24], "nonces must differ across AADs");

        // Each still decrypts under its own AAD, and not under the other's.
        assert_eq!(decrypt_field(&k, &a, &build_aad("text", 1)).unwrap(), "same");
        assert!(decrypt_field(&k, &a, &build_aad("text", 2)).is_err());
    }

    #[test]
    fn empty_passphrase_and_short_salt_rejected() {
        assert!(derive_key("", SALT).is_err());
        assert!(derive_key("p", b"short").is_err());
    }

    #[test]
    fn salt_roundtrips() {
        let s = generate_salt_b64();
        assert_eq!(decode_salt_b64(&s).unwrap().len(), 16);
    }
}
