use crate::database::is_sensitive_key;
use crate::infrastructure::encryption;
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const LEGACY_PLAIN_PREFIX: &str = "plain:";

/// Outcome of reading a value that may be encrypted at rest.
///
/// Exists because collapsing `Unreadable` into "empty" is dangerous for secrets: a caller
/// then tells the user nothing is configured, the user supplies a *new* value, and whatever
/// the old one protected becomes unrecoverable. That is exactly how a still-valid E2E
/// passphrase gets replaced after the DPAPI master key changes (OS reinstall, moved profile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretState {
    /// No row, or the stored value is empty.
    Missing,
    /// Ciphertext is present but this machine can no longer decrypt it.
    Unreadable,
    Value(String),
}

pub trait SettingsRepository {
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn get(&self, key: &str) -> Result<Option<String>>;
    /// Like [`SettingsRepository::get`], but distinguishes "not set" from "stored but
    /// undecryptable on this machine". Use for secrets whose loss is unrecoverable.
    fn get_secret(&self, key: &str) -> Result<SecretState>;
    fn get_all(&self) -> Result<HashMap<String, String>>;
    fn clear(&self) -> Result<()>;
}

pub struct SqliteSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSettingsRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn strip_plain_prefixes<'a>(mut value: &'a str) -> &'a str {
        while let Some(stripped) = value.strip_prefix(LEGACY_PLAIN_PREFIX) {
            value = stripped;
        }
        value
    }

    fn encrypted_payload<'a>(value: &'a str) -> Option<&'a str> {
        let normalized = Self::strip_plain_prefixes(value);
        // Recognise both schemes: DPAPI on Windows and the portable one used elsewhere.
        // Matching only DPAPI would make a portable payload look like plaintext and get
        // handed to the UI verbatim.
        if encryption::is_encrypted_payload(normalized) {
            Some(normalized)
        } else {
            None
        }
    }

    fn should_try_decrypt(key: &str, value: &str) -> bool {
        Self::encrypted_payload(value).is_some()
            && (is_sensitive_key(key) || key.eq_ignore_ascii_case("mqtt_username"))
    }

    fn try_decrypt_legacy_or_sensitive(key: &str, value: &str) -> Option<String> {
        if !Self::should_try_decrypt(key, value) {
            return None;
        }

        let mut current = value.to_string();
        let mut changed = false;

        for _ in 0..4 {
            let stripped = Self::strip_plain_prefixes(&current).to_string();
            if stripped != current {
                current = stripped;
                changed = true;
            }

            if !encryption::is_encrypted_payload(&current) {
                break;
            }

            let decrypted = encryption::decrypt_value(&current)?;
            current = decrypted;
            changed = true;
        }

        let final_value = Self::strip_plain_prefixes(&current).to_string();
        if final_value != current {
            current = final_value;
            changed = true;
        }

        if changed && !encryption::is_encrypted_payload(&current) {
            Some(current)
        } else {
            None
        }
    }

    pub fn get_raw(conn: &Connection, key: &str) -> Result<Option<String>> {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            if let Some(decrypted) = Self::try_decrypt_legacy_or_sensitive(key, &value) {
                return Ok(Some(decrypted));
            }
            if Self::should_try_decrypt(key, &value) {
                return Ok(Some(String::new()));
            }
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn maybe_encrypt(&self, key: &str, value: &str) -> String {
        #[cfg(feature = "portable")]
        let _ = key;
        #[cfg(not(feature = "portable"))]
        {
            if is_sensitive_key(key) && !encryption::is_encrypted_payload(value) {
                return encryption::encrypt_value(value).unwrap_or_else(|| value.to_string());
            }
        }
        value.to_string()
    }

    fn maybe_decrypt(&self, key: &str, value: &str) -> String {
        if let Some(decrypted) = Self::try_decrypt_legacy_or_sensitive(key, value) {
            return decrypted;
        }
        if Self::should_try_decrypt(key, value) {
            return String::new();
        }
        value.to_string()
    }
}

impl SettingsRepository for SqliteSettingsRepository {
    fn set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let final_value = self.maybe_encrypt(key, value);

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![key, final_value],
        )?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            let decrypted = self.maybe_decrypt(key, &value);

            // Auto-migrate to encrypted if it was plaintext and is sensitive.
            // Also migrate legacy encrypted mqtt_username back to plaintext.
            #[cfg(not(feature = "portable"))]
            {
                if is_sensitive_key(key) && !encryption::is_encrypted_payload(&value) {
                    let _ = conn.execute(
                        "UPDATE settings SET value = ? WHERE key = ?",
                        params![self.maybe_encrypt(key, &decrypted), key],
                    );
                } else if key.eq_ignore_ascii_case("mqtt_username")
                    && Self::encrypted_payload(&value).is_some()
                    && !decrypted.is_empty()
                {
                    let _ = conn.execute(
                        "UPDATE settings SET value = ? WHERE key = ?",
                        params![&decrypted, key],
                    );
                }
            }

            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    fn get_secret(&self, key: &str) -> Result<SecretState> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;

        let Some(row) = rows.next()? else {
            return Ok(SecretState::Missing);
        };
        let value: String = row.get(0)?;

        if let Some(decrypted) = Self::try_decrypt_legacy_or_sensitive(key, &value) {
            return Ok(if decrypted.is_empty() {
                SecretState::Missing
            } else {
                SecretState::Value(decrypted)
            });
        }
        // Ciphertext we could not open. Deliberately NOT reported as Missing: the stored
        // bytes are still on disk and may become readable again (e.g. restoring the original
        // Windows account), so the caller must ask for the original secret, not a new one.
        if Self::should_try_decrypt(key, &value) {
            return Ok(SecretState::Unreadable);
        }
        Ok(if value.is_empty() {
            SecretState::Missing
        } else {
            SecretState::Value(value)
        })
    }

    fn get_all(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut settings = HashMap::new();
        for row in rows {
            let (key, value) = row?;
            let decrypted = self.maybe_decrypt(&key, &value);

            // Auto-migrate to encrypted if it was plaintext and is sensitive.
            // Also migrate legacy encrypted mqtt_username back to plaintext.
            #[cfg(not(feature = "portable"))]
            {
                if is_sensitive_key(&key) && !encryption::is_encrypted_payload(&value) {
                    let _ = conn.execute(
                        "UPDATE settings SET value = ? WHERE key = ?",
                        params![self.maybe_encrypt(&key, &decrypted), &key],
                    );
                } else if key.eq_ignore_ascii_case("mqtt_username")
                    && Self::encrypted_payload(&value).is_some()
                    && !decrypted.is_empty()
                {
                    let _ = conn.execute(
                        "UPDATE settings SET value = ? WHERE key = ?",
                        params![&decrypted, &key],
                    );
                }
            }

            settings.insert(key, decrypted);
        }
        Ok(settings)
    }

    fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM settings", [])?;
        // Note: seed_defaults should probably be called by the caller or we move it here
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with(rows: &[(&str, &str)]) -> SqliteSettingsRepository {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .expect("create table");
        for (key, value) in rows {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?, ?)",
                params![key, value],
            )
            .expect("insert");
        }
        SqliteSettingsRepository::new(Arc::new(Mutex::new(conn)))
    }

    const SECRET_KEY: &str = "cloud_sync_e2e_passphrase";

    #[test]
    fn absent_secret_reads_as_missing() {
        let repo = repo_with(&[]);
        assert_eq!(repo.get_secret(SECRET_KEY).unwrap(), SecretState::Missing);
        assert_eq!(repo.get_secret("cloud_sync_e2e_salt").unwrap(), SecretState::Missing);
    }

    #[test]
    fn empty_secret_reads_as_missing() {
        let repo = repo_with(&[(SECRET_KEY, "")]);
        assert_eq!(repo.get_secret(SECRET_KEY).unwrap(), SecretState::Missing);
    }

    #[test]
    fn undecryptable_secret_is_not_reported_as_missing() {
        // Regression guard: a stored-but-unopenable secret must never look "unset". If it
        // did, the caller would prompt for a new E2E passphrase and silently strand every
        // ciphertext already uploaded under the old one.
        let repo = repo_with(&[(SECRET_KEY, "dpapi:!!!not-valid-base64!!!")]);
        let state = repo.get_secret(SECRET_KEY).unwrap();
        assert_eq!(state, SecretState::Unreadable);
        assert_ne!(state, SecretState::Missing);
    }

    #[test]
    fn plaintext_non_sensitive_value_reads_through() {
        let repo = repo_with(&[("app.language", "zh")]);
        assert_eq!(
            repo.get_secret("app.language").unwrap(),
            SecretState::Value("zh".to_string())
        );
    }
}
