use crate::database::{
    calc_image_hash, calc_text_hash, has_sensitive_tag, is_text_type, save_image_to_file,
};
use crate::domain::models::ClipboardEntry;
use crate::infrastructure::encryption;
use crate::infrastructure::repository::settings_repo::SqliteSettingsRepository;
use rusqlite::params;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use urlencoding::decode;

const RICH_IMAGE_FALLBACK_PREFIX: &str = "<!--TIEZ_RICH_IMAGE:";
const RICH_IMAGE_FALLBACK_SUFFIX: &str = "-->";

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn is_syncable_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "text" | "code" | "url" | "rich_text" | "image" | "file" | "video" | "emoji_sync"
    )
}

pub trait ClipboardRepository {
    fn save(
        &self,
        entry: &ClipboardEntry,
        data_dir: Option<&std::path::Path>,
    ) -> Result<i64, String>;
    fn get_history(
        &self,
        limit: i32,
        offset: i32,
        content_type: Option<&str>,
    ) -> Result<Vec<ClipboardEntry>, String>;
    fn search(&self, query: &str, limit: i32, tag_only: bool) -> Result<Vec<ClipboardEntry>, String>;
    fn delete(&self, id: i64, data_dir: Option<&std::path::Path>) -> Result<(), String>;
    fn clear(&self, data_dir: Option<&std::path::Path>) -> Result<(), String>;
    fn get_count(&self) -> Result<i64, String>;
    fn increment_use_count(&self, id: i64) -> Result<(), String>;
    fn touch_entry(&self, id: i64, timestamp: i64) -> Result<(), String>;
    fn toggle_pin(&self, id: i64, is_pinned: bool) -> Result<(), String>;
    fn update_pinned_order(&self, orders: Vec<(i64, i64)>) -> Result<(), String>;
    fn get_entry_by_id(&self, id: i64) -> Result<Option<ClipboardEntry>, String>;
    fn get_entry_by_content(
        &self,
        content: &str,
        content_type: Option<&str>,
    ) -> Result<Option<i64>, String>;
    fn update_entry_content(&self, id: i64, content: &str, preview: &str) -> Result<(), String>;
    fn get_entry_content(&self, id: i64) -> Result<Option<String>, String>;
    fn get_entry_content_full(&self, id: i64) -> Result<Option<(String, String)>, String>;
    fn get_entry_content_with_html(
        &self,
        id: i64,
    ) -> Result<Option<(String, String, Option<String>)>, String>;
}

pub struct SqliteClipboardRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteClipboardRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn encrypt_entry_with_conn(&self, conn: &Connection, id: i64) -> Result<(), String> {
        let (content_raw, preview_raw, html_raw, content_type, content_hash): (String, String, Option<String>, String, i64) =
            conn.query_row(
                "SELECT content, preview, html_content, content_type, content_hash FROM clipboard_history WHERE id = ?",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2).ok(), row.get(3)?, row.get(4)?)),
            ).map_err(|e| e.to_string())?;

        let already_encrypted = encryption::is_encrypted_payload(&content_raw)
            && encryption::is_encrypted_payload(&preview_raw)
            && html_raw
                .as_ref()
                .map(|h| encryption::is_encrypted_payload(h))
                .unwrap_or(true);
        if already_encrypted {
            return Ok(());
        }

        let content_plain = self.maybe_decrypt_text(&content_raw);
        let preview_plain = self.maybe_decrypt_text(&preview_raw);
        let html_plain = html_raw.map(|h| self.maybe_decrypt_text(&h));

        let content_enc = self.maybe_encrypt_text(&content_plain);
        let preview_enc = self.maybe_encrypt_text(&preview_plain);
        let html_enc = html_plain.as_ref().map(|h| self.maybe_encrypt_text(h));
        let new_hash = if is_text_type(&content_type) {
            calc_text_hash(&content_plain) as i64
        } else {
            content_hash
        };

        conn.execute(
            "UPDATE clipboard_history SET content = ?, preview = ?, html_content = ?, content_hash = ? WHERE id = ?",
            params![content_enc, preview_enc, html_enc, new_hash, id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn decrypt_entry_with_conn(&self, conn: &Connection, id: i64) -> Result<(), String> {
        let (content_raw, preview_raw, html_raw, content_type, content_hash): (String, String, Option<String>, String, i64) =
            conn.query_row(
                "SELECT content, preview, html_content, content_type, content_hash FROM clipboard_history WHERE id = ?",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2).ok(), row.get(3)?, row.get(4)?)),
            ).map_err(|e| e.to_string())?;

        let any_encrypted = encryption::is_encrypted_payload(&content_raw)
            || encryption::is_encrypted_payload(&preview_raw)
            || html_raw
                .as_ref()
                .map(|h| encryption::is_encrypted_payload(h))
                .unwrap_or(false);
        if !any_encrypted {
            return Ok(());
        }

        let content_plain = self.maybe_decrypt_text(&content_raw);
        let preview_plain = self.maybe_decrypt_text(&preview_raw);
        let html_plain = html_raw.map(|h| self.maybe_decrypt_text(&h));
        let new_hash = if is_text_type(&content_type) {
            calc_text_hash(&content_plain) as i64
        } else {
            content_hash
        };

        conn.execute(
            "UPDATE clipboard_history SET content = ?, preview = ?, html_content = ?, content_hash = ? WHERE id = ?",
            params![content_plain, preview_plain, html_plain, new_hash, id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn sync_entry_tags_with_conn(
        &self,
        conn: &Connection,
        entry_id: i64,
        tags: &[String],
    ) -> Result<(), String> {
        conn.execute(
            "DELETE FROM entry_tags WHERE entry_id = ?",
            params![entry_id],
        )
        .map_err(|e| e.to_string())?;
        for tag in tags {
            let clean = tag.trim();
            if clean.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO entry_tags (entry_id, tag) VALUES (?1, ?2)",
                params![entry_id, clean],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn upsert_tombstone_with_conn(
        &self,
        conn: &Connection,
        content_type: &str,
        content_hash: i64,
        deleted_at: i64,
    ) -> Result<(), String> {
        if !is_syncable_content_type(content_type) || content_hash == 0 {
            return Ok(());
        }

        conn.execute(
            "INSERT INTO cloud_sync_tombstones (content_type, content_hash, deleted_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(content_type, content_hash)
             DO UPDATE SET deleted_at = MAX(cloud_sync_tombstones.deleted_at, excluded.deleted_at)",
            params![content_type, content_hash, deleted_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn clear_tombstone_with_conn(
        &self,
        conn: &Connection,
        content_type: &str,
        content_hash: i64,
    ) -> Result<(), String> {
        if !is_syncable_content_type(content_type) || content_hash == 0 {
            return Ok(());
        }

        conn.execute(
            "DELETE FROM cloud_sync_tombstones WHERE content_type = ?1 AND content_hash = ?2",
            params![content_type, content_hash],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn maybe_encrypt_text(&self, value: &str) -> String {
        #[cfg(not(feature = "portable"))]
        {
            if encryption::is_encrypted_payload(value) {
                return value.to_string();
            }
            encryption::encrypt_value(value).unwrap_or_else(|| value.to_string())
        }
        #[cfg(feature = "portable")]
        {
            value.to_string()
        }
    }

    fn maybe_decrypt_text(&self, value: &str) -> String {
        if encryption::is_encrypted_payload(value) {
            encryption::decrypt_value(value).unwrap_or_else(|| value.to_string())
        } else {
            value.to_string()
        }
    }

    fn extract_rich_image_fallback_payload(html: &str) -> Option<String> {
        if let Some(start) = html.rfind(RICH_IMAGE_FALLBACK_PREFIX) {
            let marker_start = start + RICH_IMAGE_FALLBACK_PREFIX.len();
            if let Some(end_rel) = html[marker_start..].find(RICH_IMAGE_FALLBACK_SUFFIX) {
                let marker_end = marker_start + end_rel;
                let payload = html[marker_start..marker_end].trim();
                if !payload.is_empty() {
                    return Some(payload.to_string());
                }
            }
        }
        None
    }

    fn fallback_payload_to_path(payload: &str) -> Option<PathBuf> {
        let value = payload.trim();
        if value.is_empty() || value.starts_with("data:image/") {
            return None;
        }

        let path_raw = if value.starts_with("file://") {
            value.trim_start_matches("file://")
        } else {
            value
        };

        let path_without_drive_prefix =
            if path_raw.starts_with('/') && path_raw.chars().nth(2) == Some(':') {
                &path_raw[1..]
            } else {
                path_raw
            };

        let decoded_path = decode(path_without_drive_prefix)
            .map(|p| p.into_owned())
            .unwrap_or_else(|_| path_without_drive_prefix.to_string());

        if decoded_path.is_empty() {
            None
        } else {
            Some(PathBuf::from(decoded_path))
        }
    }

    fn collect_attachment_paths_for_cleanup(
        &self,
        content_raw: &str,
        html_raw: Option<&str>,
        is_external: bool,
        attachments_dir: &std::path::Path,
    ) -> Vec<PathBuf> {
        let mut paths = HashSet::new();

        if is_external {
            let content_path = PathBuf::from(self.maybe_decrypt_text(content_raw));
            if content_path.starts_with(attachments_dir) {
                paths.insert(content_path);
            }
        }

        if let Some(html_raw_value) = html_raw {
            let html = self.maybe_decrypt_text(html_raw_value);
            if let Some(payload) = Self::extract_rich_image_fallback_payload(&html) {
                if let Some(path) = Self::fallback_payload_to_path(&payload) {
                    if path.starts_with(attachments_dir) {
                        paths.insert(path);
                    }
                }
            }
        }

        paths.into_iter().collect()
    }

    pub fn save_with_conn(
        &self,
        conn: &Connection,
        entry: &ClipboardEntry,
        data_dir: Option<&std::path::Path>,
    ) -> Result<i64, String> {
        // Encrypt only when explicitly marked as sensitive
        let should_encrypt = has_sensitive_tag(&entry.tags);

        let mut final_content = entry.content.clone();
        let mut final_is_external = entry.is_external;

        // Externalize image if possible
        if entry.content_type == "image" && entry.content.starts_with("data:image/") {
            if let Some(dir) = data_dir {
                if let Some(path) = save_image_to_file(&entry.content, dir) {
                    final_content = path;
                    final_is_external = true;
                }
            }
        }

        let calculated_hash = if entry.content_type == "image" {
            if entry.content.starts_with("data:") {
                calc_image_hash(&entry.content).unwrap_or(0)
            } else {
                if let Ok(img) = image::open(&entry.content) {
                    let thumb = img.resize_exact(32, 32, image::imageops::FilterType::Nearest);
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    thumb.as_bytes().hash(&mut hasher);
                    hasher.finish() as i64
                } else {
                    0
                }
            }
        } else {
            calc_text_hash(&final_content) as i64
        };

        // Re-adding an item should clear an older delete tombstone for the same fingerprint.
        let _ = self.clear_tombstone_with_conn(conn, &entry.content_type, calculated_hash);

        let (content, preview, content_hash, html_content) = if should_encrypt {
            let encrypted_content = self.maybe_encrypt_text(&final_content);
            let encrypted_preview = self.maybe_encrypt_text(&entry.preview);
            let encrypted_html = entry
                .html_content
                .as_ref()
                .map(|html| self.maybe_encrypt_text(html));
            (
                encrypted_content,
                encrypted_preview,
                calculated_hash,
                encrypted_html,
            )
        } else {
            (
                final_content,
                entry.preview.clone(),
                calculated_hash,
                entry.html_content.clone(),
            )
        };

        let mut seen: HashSet<String> = HashSet::new();
        let mut cleaned_tags: Vec<String> = Vec::new();
        for tag in &entry.tags {
            let t = tag.trim();
            if t.is_empty() {
                continue;
            }
            let t_owned = t.to_string();
            if seen.insert(t_owned.clone()) {
                cleaned_tags.push(t_owned);
            }
        }

        if entry.id > 0 {
            // 去重命中已有条目的合并 UPDATE（"移到顶部"逻辑），保证重复合并时数据无损：
            // - 保留已有标签：本次传入标签为空时沿用 DB 已有标签，绝不以空集覆盖（U2.1）
            // - use_count 求和：旧使用次数 + 新捕获使用次数（U2.6）
            // - 保留置顶：旧条目或新捕获条目任一置顶则结果置顶；已置顶则保留其 pinned_order（U2.7）
            let final_tags: Vec<String> = if cleaned_tags.is_empty() {
                let existing_json: String = conn
                    .query_row(
                        "SELECT tags FROM clipboard_history WHERE id = ?1",
                        params![entry.id],
                        |row| row.get(0),
                    )
                    .unwrap_or_else(|_| "[]".to_string());
                serde_json::from_str(&existing_json).unwrap_or_default()
            } else {
                cleaned_tags
            };

            conn.execute(
                "UPDATE clipboard_history SET 
                    content_type = ?1, 
                    content = ?2, 
                    html_content = ?3, 
                    source_app = ?4, 
                    timestamp = ?5, 
                    preview = ?6, 
                    content_hash = ?7, 
                    tags = ?8, 
                    is_external = ?9,
                    source_app_path = ?10,
                    use_count = use_count + ?11,
                    pinned_order = CASE WHEN is_pinned = 1 THEN pinned_order ELSE ?12 END,
                    is_pinned = MAX(is_pinned, ?13)
                 WHERE id = ?14",
                params![
                    entry.content_type,
                    content,
                    html_content,
                    entry.source_app,
                    entry.timestamp,
                    preview,
                    content_hash,
                    serde_json::to_string(&final_tags).unwrap_or_else(|_| "[]".to_string()),
                    if final_is_external { 1 } else { 0 },
                    entry.source_app_path.as_deref(),
                    entry.use_count,
                    entry.pinned_order,
                    if entry.is_pinned { 1 } else { 0 },
                    entry.id
                ],
            )
            .map_err(|e| e.to_string())?;
            self.sync_entry_tags_with_conn(conn, entry.id, &final_tags)?;
            Ok(entry.id)
        } else {
            // Insert new entry
            conn.execute(
                "INSERT INTO clipboard_history (content_type, content, html_content, source_app, timestamp, preview, is_pinned, content_hash, tags, is_external, pinned_order, source_app_path) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    entry.content_type,
                    content,
                    html_content,
                    entry.source_app,
                    entry.timestamp,
                    preview,
                    if entry.is_pinned { 1 } else { 0 },
                    content_hash,
                    serde_json::to_string(&cleaned_tags).unwrap_or_else(|_| "[]".to_string()),
                    if final_is_external { 1 } else { 0 },
                    entry.pinned_order,
                    entry.source_app_path.as_deref()
                ],
            ).map_err(|e| e.to_string())?;

            let new_id = conn.last_insert_rowid();
            self.sync_entry_tags_with_conn(conn, new_id, &cleaned_tags)?;
            Ok(new_id)
        }
    }

    pub fn delete_with_conn(
        &self,
        conn: &Connection,
        id: i64,
        data_dir: Option<&std::path::Path>,
    ) -> Result<(), String> {
        let mut tombstone: Option<(String, i64)> = None;
        // Check for external files to delete
        if let Some(dir) = data_dir {
            let attachments_dir = dir.join("attachments");
            let mut stmt = conn
                 .prepare("SELECT content, html_content, is_external, content_type, content_hash FROM clipboard_history WHERE id = ?")
                 .map_err(|e| e.to_string())?;

            if let Ok(entry) = stmt.query_row([id], |row| {
                let content_raw: String = row.get(0)?;
                let html_raw: Option<String> = row.get(1).ok();
                let is_ext: i32 = row.get(2)?;
                let content_type: String = row.get(3)?;
                let content_hash: i64 = row.get(4)?;
                Ok((
                    content_raw,
                    html_raw,
                    is_ext == 1,
                    content_type,
                    content_hash,
                ))
            }) {
                let files_to_remove = self.collect_attachment_paths_for_cleanup(
                    &entry.0,
                    entry.1.as_deref(),
                    entry.2,
                    &attachments_dir,
                );
                for path in files_to_remove {
                    if path.exists() {
                        let _ = std::fs::remove_file(path);
                    }
                }
                tombstone = Some((entry.3, entry.4));
            }
        } else {
            let mut stmt = conn
                .prepare("SELECT content_type, content_hash FROM clipboard_history WHERE id = ?")
                .map_err(|e| e.to_string())?;
            if let Ok(entry) = stmt.query_row([id], |row| {
                let content_type: String = row.get(0)?;
                let content_hash: i64 = row.get(1)?;
                Ok((content_type, content_hash))
            }) {
                tombstone = Some(entry);
            }
        }

        if let Some((content_type, content_hash)) = tombstone {
            let _ = self.upsert_tombstone_with_conn(conn, &content_type, content_hash, now_ms());
        }

        conn.execute("DELETE FROM clipboard_history WHERE id = ?", [id])
            .map_err(|e| e.to_string())?;
        let _ = conn.execute("DELETE FROM entry_tags WHERE entry_id = ?", params![id]);
        Ok(())
    }

    pub fn delete_metadata_with_conn(&self, conn: &Connection, id: i64) -> Result<(), String> {
        conn.execute("DELETE FROM clipboard_history WHERE id = ?", params![id])
            .map_err(|e| e.to_string())?;
        let _ = conn.execute("DELETE FROM entry_tags WHERE entry_id = ?", params![id]);
        Ok(())
    }

    pub fn find_by_content_with_conn(
        &self,
        conn: &Connection,
        content: &str,
        content_type: Option<&str>,
    ) -> Result<Option<i64>, String> {
        if content_type == Some("image") {
            if let Some(hash) = calc_image_hash(content) {
                let mut stmt = conn
                    .prepare(
                        "SELECT id FROM clipboard_history \
                     WHERE (content_type = 'image' AND content_hash = ?) OR content = ?",
                    )
                    .map_err(|e| e.to_string())?;
                let mut rows = stmt
                    .query(params![hash, content])
                    .map_err(|e| e.to_string())?;
                if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    return Ok(Some(row.get(0).map_err(|e| e.to_string())?));
                }
                return Ok(None);
            }
        }

        let hash = calc_text_hash(content) as i64;

        if let Some(ct) = content_type {
            let mut stmt = conn.prepare(
                "SELECT id FROM clipboard_history \
                 WHERE (content_type = ? AND content_hash = ?) OR (content_type = ? AND content = ?)",
            ).map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query(params![ct, hash, ct, content])
                .map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(Some(row.get(0).map_err(|e| e.to_string())?))
            } else {
                Ok(None)
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id FROM clipboard_history \
                 WHERE ((content_type IN ('text', 'rich_text', 'code', 'url')) AND content_hash = ?) OR content = ?",
            ).map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query(params![hash, content])
                .map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                Ok(Some(row.get(0).map_err(|e| e.to_string())?))
            } else {
                Ok(None)
            }
        }
    }

    pub fn enforce_limit_with_conn(
        &self,
        conn: &Connection,
        data_dir: Option<&std::path::Path>,
    ) -> Result<Vec<i64>, String> {
        // Check if storage limit is enabled
        if let Ok(Some(limit_enabled_str)) =
            SqliteSettingsRepository::get_raw(conn, "app.persistent_limit_enabled")
        {
            if limit_enabled_str == "false" {
                return Ok(Vec::new());
            }
        }

        // Get the storage limit
        if let Ok(Some(limit_str)) = SqliteSettingsRepository::get_raw(conn, "app.persistent_limit")
        {
            if let Ok(limit) = limit_str.parse::<i32>() {
                // Count non-pinned entries
                let count: i32 = conn.query_row(
                    "SELECT COUNT(*) FROM clipboard_history WHERE is_pinned = 0 AND (tags = '[]' OR tags IS NULL)",
                    [],
                    |row| row.get(0)
                ).map_err(|e| e.to_string())?;

                if count > limit {
                    // First, get the IDs that will be deleted
                    let to_delete = count - limit;
                    let deleted_ids: Vec<i64> = {
                        let mut stmt = conn
                            .prepare(
                                "SELECT id FROM clipboard_history 
                             WHERE is_pinned = 0 AND (tags = '[]' OR tags IS NULL)
                             ORDER BY timestamp ASC 
                             LIMIT ?",
                            )
                            .map_err(|e| e.to_string())?;

                        let rows = stmt
                            .query_map([to_delete], |row| row.get(0))
                            .map_err(|e| e.to_string())?;
                        rows.filter_map(|r| r.ok()).collect()
                    };
                    // Actually delete records (and files if needed)
                    for id in &deleted_ids {
                        let _ = self.delete_with_conn(conn, *id, data_dir);
                    }
                    return Ok(deleted_ids);
                }
            }
        }

        Ok(Vec::new())
    }
    pub fn toggle_pin_with_conn(
        &self,
        conn: &Connection,
        id: i64,
        is_pinned: bool,
    ) -> Result<(), String> {
        if is_pinned {
            // Set pinned_order to max + 1 so it appears at top
            conn.execute(
                "UPDATE clipboard_history 
                 SET is_pinned = 1, 
                     pinned_order = (SELECT COALESCE(MAX(pinned_order), 0) + 1 FROM clipboard_history WHERE is_pinned = 1) 
                 WHERE id = ?",
                params![id],
            ).map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "UPDATE clipboard_history SET is_pinned = 0, pinned_order = 0 WHERE id = ?",
                params![id],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn update_pinned_order_with_conn(
        &self,
        conn: &Connection,
        orders: Vec<(i64, i64)>,
    ) -> Result<(), String> {
        for (id, order) in orders {
            conn.execute(
                "UPDATE clipboard_history SET pinned_order = ? WHERE id = ?",
                params![order, id],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_entry_by_id_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> Result<Option<ClipboardEntry>, String> {
        let mut stmt = conn.prepare(
            "SELECT id, content_type, content, html_content, source_app, timestamp, preview, is_pinned, tags, use_count, is_external, pinned_order, source_app_path 
             FROM clipboard_history 
             WHERE id = ? 
             LIMIT 1",
        ).map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let tags_str: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

            let content_raw: String = row.get(2).map_err(|e| e.to_string())?;
            let html_raw: Option<String> = row.get(3).map_err(|e| e.to_string()).unwrap_or(None);
            let preview_raw: String = row.get(6).map_err(|e| e.to_string())?;
            let content = self.maybe_decrypt_text(&content_raw);
            let preview = self.maybe_decrypt_text(&preview_raw);
            let html_content = html_raw.map(|v| self.maybe_decrypt_text(&v));

            Ok(Some(ClipboardEntry {
                id: row.get(0).map_err(|e| e.to_string())?,
                content_type: row.get(1).map_err(|e| e.to_string())?,
                content,
                html_content,
                source_app: row.get(4).map_err(|e| e.to_string())?,
                timestamp: row.get(5).map_err(|e| e.to_string())?,
                preview,
                is_pinned: row.get::<_, i32>(7).map_err(|e| e.to_string())? == 1,
                tags,
                use_count: row.get(9).unwrap_or(0),
                is_external: row.get::<_, i32>(10).unwrap_or(0) == 1,
                pinned_order: row.get(11).unwrap_or(0),
                source_app_path: row.get(12).unwrap_or(None),
                file_preview_exists: true,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_entry_content_with_conn(
        &self,
        conn: &Connection,
        id: i64,
        content: &str,
        preview: &str,
    ) -> Result<(), String> {
        let (old_content_raw, content_type, tags_json, has_html) = conn
            .query_row(
                "SELECT content, content_type, tags, (html_content IS NOT NULL) FROM clipboard_history WHERE id = ?",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        let old_content = self.maybe_decrypt_text(&old_content_raw);
        // Procceed if content changed, OR if content is same but we need to transition away from rich text/clear HTML
        if old_content == content && content_type != "rich_text" && !has_html {
            return Ok(());
        }

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let should_encrypt = has_sensitive_tag(&tags);

        if is_text_type(&content_type) {
            let hash = calc_text_hash(content) as i64;
            let new_type = if content_type == "rich_text" {
                "text"
            } else {
                &content_type
            };
            if should_encrypt {
                let encrypted_content = self.maybe_encrypt_text(content);
                let encrypted_preview = self.maybe_encrypt_text(preview);
                conn.execute(
                    "UPDATE clipboard_history SET content = ?, preview = ?, content_hash = ?, html_content = NULL, content_type = ? WHERE id = ?",
                    params![encrypted_content, encrypted_preview, hash, new_type, id],
                ).map_err(|e| e.to_string())?;
            } else {
                conn.execute(
                    "UPDATE clipboard_history SET content = ?, preview = ?, content_hash = ?, html_content = NULL, content_type = ? WHERE id = ?",
                    params![content, preview, hash, new_type, id],
                ).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        if should_encrypt {
            let encrypted_content = self.maybe_encrypt_text(content);
            let encrypted_preview = self.maybe_encrypt_text(preview);
            conn.execute(
                "UPDATE clipboard_history SET content = ?, preview = ?, html_content = NULL WHERE id = ?",
                params![encrypted_content, encrypted_preview, id],
            ).map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "UPDATE clipboard_history SET content = ?, preview = ?, html_content = NULL WHERE id = ?",
                params![content, preview, id],
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_entry_content_full_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> Result<Option<(String, String)>, String> {
        let mut stmt = conn
            .prepare("SELECT content, content_type FROM clipboard_history WHERE id = ?")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let content: String = row.get(0).map_err(|e| e.to_string())?;
            let content_type: String = row.get(1).map_err(|e| e.to_string())?;
            Ok(Some((self.maybe_decrypt_text(&content), content_type)))
        } else {
            Ok(None)
        }
    }

    pub fn get_entry_content_with_html_with_conn(
        &self,
        conn: &Connection,
        id: i64,
    ) -> Result<Option<(String, String, Option<String>)>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT content, content_type, html_content FROM clipboard_history WHERE id = ?",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let content: String = row.get(0).map_err(|e| e.to_string())?;
            let content_type: String = row.get(1).map_err(|e| e.to_string())?;
            let html_raw: Option<String> = row.get(2).map_err(|e| e.to_string()).unwrap_or(None);
            let html_content = html_raw.map(|v| self.maybe_decrypt_text(&v));
            Ok(Some((
                self.maybe_decrypt_text(&content),
                content_type,
                html_content,
            )))
        } else {
            Ok(None)
        }
    }
}

impl ClipboardRepository for SqliteClipboardRepository {
    fn save(
        &self,
        entry: &ClipboardEntry,
        data_dir: Option<&std::path::Path>,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.save_with_conn(&conn, entry, data_dir)
    }

    fn get_history(
        &self,
        limit: i32,
        offset: i32,
        content_type: Option<&str>,
    ) -> Result<Vec<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let map_row = |row: &rusqlite::Row| {
            let tags_str: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let content_type: String = row.get(1)?;
            let content_raw: String = row.get(2)?;
            let html_raw: Option<String> = row.get(3).ok();
            let preview_raw: String = row.get(6)?;
            let content = self.maybe_decrypt_text(&content_raw);
            let preview = self.maybe_decrypt_text(&preview_raw);
            let html_content = html_raw.as_ref().map(|v| self.maybe_decrypt_text(v));

            Ok((
                ClipboardEntry {
                    id: row.get(0)?,
                    content_type,
                    content,
                    html_content,
                    source_app: row.get(4)?,
                    timestamp: row.get(5)?,
                    preview,
                    is_pinned: row.get::<_, i32>(7)? == 1,
                    tags,
                    use_count: row.get(9).unwrap_or(0),
                    is_external: row.get::<_, i32>(10)? == 1,
                    pinned_order: row.get(11).unwrap_or(0),
                    source_app_path: row.get(12).unwrap_or(None),
                    // Avoid synchronous filesystem existence checks in history query.
                    // Missing files are still handled by frontend image/file preview error fallback.
                    file_preview_exists: true,
                },
                content_raw,
                preview_raw,
                html_raw,
            ))
        };

        let mut mapped_rows = Vec::new();
        if let Some(ct) = content_type {
            let mut stmt = conn.prepare(
                "SELECT id, content_type, content, html_content, source_app, timestamp, preview, is_pinned, tags, use_count, is_external, pinned_order, source_app_path 
                 FROM clipboard_history 
                 WHERE content_type = ? 
                 ORDER BY is_pinned DESC, pinned_order DESC, timestamp DESC, id DESC 
                 LIMIT ? OFFSET ?",
            ).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![ct, limit, offset], map_row)
                .map_err(|e| e.to_string())?;
            for row in rows {
                mapped_rows.push(row.map_err(|e| e.to_string())?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, content_type, content, html_content, source_app, timestamp, preview, is_pinned, tags, use_count, is_external, pinned_order, source_app_path 
                 FROM clipboard_history 
                 ORDER BY is_pinned DESC, pinned_order DESC, timestamp DESC, id DESC 
                 LIMIT ? OFFSET ?",
            ).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([limit, offset], map_row)
                .map_err(|e| e.to_string())?;
            for row in rows {
                mapped_rows.push(row.map_err(|e| e.to_string())?);
            }
        }

        let mut history = Vec::new();
        for (entry, content_raw, preview_raw, html_raw) in mapped_rows {
            #[cfg(not(feature = "portable"))]
            {
                let is_sensitive = has_sensitive_tag(&entry.tags);
                let content_encrypted = encryption::is_encrypted_payload(&content_raw);
                let preview_encrypted = encryption::is_encrypted_payload(&preview_raw);
                let html_encrypted = html_raw
                    .as_ref()
                    .map(|h| encryption::is_encrypted_payload(h))
                    .unwrap_or(false);
                let html_needs_encrypt = html_raw
                    .as_ref()
                    .map(|h| !encryption::is_encrypted_payload(h))
                    .unwrap_or(false);

                if is_sensitive && (!content_encrypted || !preview_encrypted || html_needs_encrypt)
                {
                    let _ = self.encrypt_entry_with_conn(&conn, entry.id);
                } else if !is_sensitive
                    && (content_encrypted || preview_encrypted || html_encrypted)
                {
                    let _ = self.decrypt_entry_with_conn(&conn, entry.id);
                }
            }

            history.push(entry);
        }
        Ok(history)
    }

    fn search(&self, query: &str, limit: i32, tag_only: bool) -> Result<Vec<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let term = query.trim().to_lowercase();
        if term.is_empty() {
            return Ok(Vec::new());
        }

        #[cfg(feature = "portable")]
        {
            // Portable version: Data is NOT encrypted, use conventional SQL LIKE search (fastest)
            let sql = if tag_only {
                "SELECT DISTINCT ch.id, ch.content_type, ch.content, ch.html_content, ch.source_app, ch.timestamp, ch.preview, ch.is_pinned, ch.tags, ch.use_count, ch.is_external, ch.pinned_order, ch.source_app_path
                 FROM clipboard_history ch
                 INNER JOIN entry_tags et ON ch.id = et.entry_id
                 WHERE et.tag LIKE '%' || ?1 || '%'
                 ORDER BY ch.timestamp DESC
                 LIMIT ?2"
            } else {
                "SELECT DISTINCT ch.id, ch.content_type, ch.content, ch.html_content, ch.source_app, ch.timestamp, ch.preview, ch.is_pinned, ch.tags, ch.use_count, ch.is_external, ch.pinned_order, ch.source_app_path
                 FROM clipboard_history ch
                 LEFT JOIN entry_tags et ON ch.id = et.entry_id
                 WHERE ch.content LIKE '%' || ?1 || '%'
                    OR ch.source_app LIKE '%' || ?1 || '%'
                    OR et.tag LIKE '%' || ?1 || '%'
                 ORDER BY ch.timestamp DESC
                 LIMIT ?2"
            };

            let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map(params![term, limit], |row| {
                    let tags_str: String =
                        row.get::<_, String>(8).unwrap_or_else(|_| "[]".to_string());
                    Ok(ClipboardEntry {
                        id: row.get(0)?,
                        content_type: row.get(1)?,
                        content: row.get(2)?,
                        html_content: row.get(3).ok(),
                        source_app: row.get(4)?,
                        timestamp: row.get(5)?,
                        preview: row.get(6)?,
                        is_pinned: row.get::<_, i32>(7)? == 1,
                        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                        use_count: row.get(9).unwrap_or(0),
                        is_external: row.get::<_, i32>(10)? == 1,
                        pinned_order: row.get(11).unwrap_or(0),
                        source_app_path: row.get(12).unwrap_or(None),
                        file_preview_exists: true, // Simplified for search
                    })
                })
                .map_err(|e| e.to_string())?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| e.to_string())?);
            }
            Ok(results)
        }

        #[cfg(not(feature = "portable"))]
        {
            let mut results: Vec<ClipboardEntry> = Vec::new();
            let mut seen: HashSet<i64> = HashSet::new();

            let sensitive_tags_sql = {
                let tags = crate::database::SENSITIVE_TAGS;
                let parts: Vec<String> = tags
                    .iter()
                    .map(|t| format!("'{}'", t.replace('\'', "''")))
                    .collect();
                format!("({})", parts.join(","))
            };

            // 1) SQL search for non-sensitive (plaintext) entries
            let sql_non_sensitive = if tag_only {
                format!(
                    "SELECT DISTINCT ch.id, ch.content_type, ch.content, ch.html_content, ch.source_app, ch.timestamp, ch.preview, ch.is_pinned, ch.tags, ch.use_count, ch.is_external, ch.pinned_order, ch.source_app_path
                     FROM clipboard_history ch
                     INNER JOIN entry_tags et ON ch.id = et.entry_id
                     WHERE NOT EXISTS (
                         SELECT 1 FROM entry_tags se
                         WHERE se.entry_id = ch.id
                           AND se.tag COLLATE NOCASE IN {}
                     )
                       AND et.tag LIKE '%' || ?1 || '%'
                     ORDER BY ch.timestamp DESC, ch.id DESC
                     LIMIT ?2",
                    sensitive_tags_sql
                )
            } else {
                format!(
                    "SELECT DISTINCT ch.id, ch.content_type, ch.content, ch.html_content, ch.source_app, ch.timestamp, ch.preview, ch.is_pinned, ch.tags, ch.use_count, ch.is_external, ch.pinned_order, ch.source_app_path
                     FROM clipboard_history ch
                     LEFT JOIN entry_tags et ON ch.id = et.entry_id
                     WHERE NOT EXISTS (
                         SELECT 1 FROM entry_tags se
                         WHERE se.entry_id = ch.id
                           AND se.tag COLLATE NOCASE IN {}
                     )
                       AND (
                         ch.content LIKE '%' || ?1 || '%'
                         OR ch.source_app LIKE '%' || ?1 || '%'
                         OR et.tag LIKE '%' || ?1 || '%'
                       )
                     ORDER BY ch.timestamp DESC, ch.id DESC
                     LIMIT ?2",
                    sensitive_tags_sql
                )
            };

            let mut stmt = conn
                .prepare(&sql_non_sensitive)
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![term, limit], |row| {
                    let tags_str: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
                    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                    let content_raw: String = row.get(2)?;
                    let preview_raw: String = row.get(6)?;
                    let html_raw: Option<String> = row.get(3).ok();
                    let content = self.maybe_decrypt_text(&content_raw);
                    let preview = self.maybe_decrypt_text(&preview_raw);
                    let html_content = html_raw.map(|v| self.maybe_decrypt_text(&v));

                    Ok(ClipboardEntry {
                        id: row.get(0)?,
                        content_type: row.get(1)?,
                        content,
                        html_content,
                        source_app: row.get(4)?,
                        timestamp: row.get(5)?,
                        preview,
                        is_pinned: row.get::<_, i32>(7)? == 1,
                        tags,
                        use_count: row.get(9).unwrap_or(0),
                        is_external: row.get::<_, i32>(10)? == 1,
                        pinned_order: row.get(11).unwrap_or(0),
                        source_app_path: row.get(12).unwrap_or(None),
                        file_preview_exists: true,
                    })
                })
                .map_err(|e| e.to_string())?;

            for row in rows {
                if let Ok(entry) = row {
                    if seen.insert(entry.id) {
                        results.push(entry);
                    }
                }
            }

            // 2) Decrypt-scan sensitive or encrypted entries (only if needed)
            if results.len() < limit as usize {
                let mut cursor_ts = i64::MAX;
                let mut cursor_id = i64::MAX;
                let batch_size = 500;
                // Both at-rest schemes: DPAPI on Windows, the portable one elsewhere. Matching
                // only one would hide encrypted rows from this scan on the other platform.
                let enc_like = format!("{}%", encryption::ENCRYPTED_PREFIXES[0]);
                let enc_like_alt = format!("{}%", encryption::ENCRYPTED_PREFIXES[1]);
                let sql_sensitive = format!(
                    "SELECT ch.id, ch.content_type, ch.content, ch.html_content, ch.source_app, ch.timestamp, ch.preview, ch.is_pinned, ch.tags, ch.use_count, ch.is_external, ch.pinned_order, ch.source_app_path 
                     FROM clipboard_history ch
                     WHERE (
                         EXISTS (
                             SELECT 1 FROM entry_tags se 
                             WHERE se.entry_id = ch.id 
                               AND se.tag COLLATE NOCASE IN {}
                         )
                         OR ch.content LIKE ?1 
                         OR ch.preview LIKE ?1 
                         OR ch.html_content LIKE ?1
                         OR ch.content LIKE ?5
                         OR ch.preview LIKE ?5
                         OR ch.html_content LIKE ?5
                     )
                       AND ((ch.timestamp < ?2) OR (ch.timestamp = ?2 AND ch.id < ?3))
                     ORDER BY ch.timestamp DESC, ch.id DESC
                     LIMIT ?4",
                    sensitive_tags_sql
                );

                loop {
                    let mut stmt = conn.prepare(&sql_sensitive).map_err(|e| e.to_string())?;
                    let rows = stmt
                        .query_map(
                            params![enc_like, cursor_ts, cursor_id, batch_size, enc_like_alt],
                            |row| {
                            let tags_str: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
                            Ok(ClipboardEntry {
                                id: row.get(0)?,
                                content_type: row.get(1)?,
                                content: row.get(2)?, // Encrypted
                                html_content: row.get(3).ok(),
                                source_app: row.get(4)?,
                                timestamp: row.get(5)?,
                                preview: row.get(6)?, // Encrypted
                                is_pinned: row.get::<_, i32>(7)? == 1,
                                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                                use_count: row.get(9).unwrap_or(0),
                                is_external: row.get::<_, i32>(10)? == 1,
                                pinned_order: row.get(11).unwrap_or(0),
                                source_app_path: row.get(12).unwrap_or(None),
                                file_preview_exists: true,
                            })
                            },
                        )
                        .map_err(|e| e.to_string())?;

                    let mut batch: Vec<ClipboardEntry> = Vec::new();
                    for row in rows {
                        if let Ok(mut entry) = row {
                            entry.content = self.maybe_decrypt_text(&entry.content);
                            entry.preview = self.maybe_decrypt_text(&entry.preview);
                            if let Some(html) = entry.html_content.take() {
                                entry.html_content = Some(self.maybe_decrypt_text(&html));
                            }
                            batch.push(entry);
                        }
                    }

                    if batch.is_empty() {
                        break;
                    }

                    for entry in batch.iter() {
                        let matches = if tag_only {
                            entry.tags.iter().any(|t| t.to_lowercase().contains(&term))
                        } else {
                            entry.content.to_lowercase().contains(&term)
                                || entry.source_app.to_lowercase().contains(&term)
                                || entry.tags.iter().any(|t| t.to_lowercase().contains(&term))
                        };

                        if matches && seen.insert(entry.id) {
                            results.push(entry.clone());
                            if results.len() >= limit as usize {
                                break;
                            }
                        }
                    }

                    if results.len() >= limit as usize {
                        break;
                    }

                    if let Some(last) = batch.last() {
                        cursor_ts = last.timestamp;
                        cursor_id = last.id;
                    } else {
                        break;
                    }
                }
            }

            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(b.id.cmp(&a.id)));
            if results.len() > limit as usize {
                results.truncate(limit as usize);
            }
            Ok(results)
        }
    }

    fn delete(&self, id: i64, data_dir: Option<&std::path::Path>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.delete_with_conn(&conn, id, data_dir)
    }

    fn clear(&self, data_dir: Option<&std::path::Path>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Get IDs of unpinned items without tags.
        let mut stmt = conn
            .prepare(
                "SELECT id FROM clipboard_history 
             WHERE is_pinned = 0 
               AND NOT EXISTS (SELECT 1 FROM entry_tags WHERE entry_id = clipboard_history.id)",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        let ids: Vec<i64> = rows.filter_map(Result::ok).collect();

        // Delete one-by-one so tombstones are recorded for cloud deletion sync.
        for id in &ids {
            self.delete_with_conn(&conn, *id, data_dir)?;
        }

        // VACUUM to reclaim space
        let _ = conn.execute_batch("VACUUM;");
        Ok(())
    }

    fn get_count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM clipboard_history")
            .map_err(|e| e.to_string())?;
        let count: i64 = stmt
            .query_row([], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        Ok(count)
    }

    fn increment_use_count(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE clipboard_history SET use_count = use_count + 1 WHERE id = ?",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn touch_entry(&self, id: i64, timestamp: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE clipboard_history SET timestamp = ? WHERE id = ?",
            params![timestamp, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn toggle_pin(&self, id: i64, is_pinned: bool) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.toggle_pin_with_conn(&conn, id, is_pinned)
    }

    fn update_pinned_order(&self, orders: Vec<(i64, i64)>) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        self.update_pinned_order_with_conn(&tx, orders)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_entry_by_id(&self, id: i64) -> Result<Option<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.get_entry_by_id_with_conn(&conn, id)
    }

    fn get_entry_by_content(
        &self,
        content: &str,
        content_type: Option<&str>,
    ) -> Result<Option<i64>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.find_by_content_with_conn(&conn, content, content_type)
    }

    fn update_entry_content(&self, id: i64, content: &str, preview: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.update_entry_content_with_conn(&conn, id, content, preview)
    }

    fn get_entry_content(&self, id: i64) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT content FROM clipboard_history WHERE id = ?")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let content: String = row.get(0).map_err(|e| e.to_string())?;
            Ok(Some(self.maybe_decrypt_text(&content)))
        } else {
            Ok(None)
        }
    }

    fn get_entry_content_full(&self, id: i64) -> Result<Option<(String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.get_entry_content_full_with_conn(&conn, id)
    }

    fn get_entry_content_with_html(
        &self,
        id: i64,
    ) -> Result<Option<(String, String, Option<String>)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        self.get_entry_content_with_html_with_conn(&conn, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// 创建内存测试数据库，建表结构与生产 schema 关键列对齐
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE clipboard_history (
                id INTEGER PRIMARY KEY,
                content_type TEXT NOT NULL,
                content TEXT NOT NULL,
                html_content TEXT,
                source_app TEXT NOT NULL,
                source_app_path TEXT,
                timestamp INTEGER NOT NULL,
                preview TEXT NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                content_hash INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                use_count INTEGER NOT NULL DEFAULT 0,
                is_external INTEGER NOT NULL DEFAULT 0,
                pinned_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE entry_tags (
                entry_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag)
            );
            CREATE TABLE cloud_sync_tombstones (
                content_type TEXT NOT NULL,
                content_hash INTEGER NOT NULL,
                deleted_at INTEGER NOT NULL,
                PRIMARY KEY (content_type, content_hash)
            );
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();
        conn
    }

    /// 构造一条文本类型 ClipboardEntry
    fn make_text_entry(id: i64, content: &str, tags: Vec<String>) -> ClipboardEntry {
        ClipboardEntry {
            id,
            content_type: "text".to_string(),
            content: content.to_string(),
            html_content: None,
            source_app: "test".to_string(),
            source_app_path: None,
            timestamp: 1_000 + id,
            preview: content.chars().take(20).collect(),
            is_pinned: false,
            tags,
            use_count: 0,
            is_external: false,
            pinned_order: 0,
            file_preview_exists: true,
        }
    }

    /// 创建一个仅用于持有连接句柄的仓储；save_with_conn 实际操作传入的 conn，
    /// 因此内部连接句柄不参与本测试
    fn make_repo() -> SqliteClipboardRepository {
        SqliteClipboardRepository::new(Arc::new(Mutex::new(Connection::open_in_memory().unwrap())))
    }

    /// 读取某条目当前的 tags JSON 列并反序列化为 Vec
    fn read_tags_json(conn: &Connection, id: i64) -> Vec<String> {
        let json: String = conn
            .query_row(
                "SELECT tags FROM clipboard_history WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    /// 读取 entry_tags 关联表中某条目的全部标签
    fn read_entry_tag_associations(conn: &Connection, id: i64) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT tag FROM entry_tags WHERE entry_id = ?1")
            .unwrap();
        let rows = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    // Feature: magpie-v0-4-1, Property 4: 标签合并并集往返一致性
    //
    // 对任意「单条标签数 0~50、合并序列长度 1~100」的重复内容合并序列，合并结果条目的
    // 标签集合应恰好等于各参与条目标签集合的并集——既不丢失任一参与标签，也不产生重复
    // 标签关联（两个标签当且仅当标签名逐字节完全相同方视为同一标签）。
    //
    // merge_tags_union 在 pipeline.rs 为私有，无法直接调用；此处以等价方式验证并集语义：
    // 完整复现生产去重合并链路——每次"重复命中"读出已有标签、与新捕获标签拼接后交由真实
    // 的 save_with_conn 去重落库，最终校验存储结果（tags JSON 列与 entry_tags 关联表）
    // 恰好等于全部参与标签的集合并集。
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]
        #[test]
        fn prop_tag_merge_union_round_trip(
            // 序列长度 1~12（落在需求的 1~100 区间内），每条 0~8 个标签（落在 0~50 内）
            // 标签取自小字母表以制造大量重叠，充分检验去重并集语义
            seq in proptest::collection::vec(
                proptest::collection::vec("[a-z]{1,4}", 0..=8),
                1..=12,
            )
        ) {
            let conn = setup_test_db();
            let repo = make_repo();

            // 独立"预言"：全部参与条目标签的逐字节集合并集
            let mut oracle: std::collections::HashSet<String> = std::collections::HashSet::new();
            for item in &seq {
                for tag in item {
                    oracle.insert(tag.clone());
                }
            }

            // 复现去重合并序列：首条 INSERT，其余每条读出已有标签后并集 UPDATE
            let mut id: i64 = 0;
            for (i, item_tags) in seq.iter().enumerate() {
                if i == 0 {
                    let entry = make_text_entry(0, "dup-content", item_tags.clone());
                    id = repo.save_with_conn(&conn, &entry, None).unwrap();
                } else {
                    let existing = read_tags_json(&conn, id);
                    let mut combined = existing;
                    combined.extend(item_tags.clone());
                    let entry = make_text_entry(id, "dup-content", combined);
                    repo.save_with_conn(&conn, &entry, None).unwrap();
                }
            }

            // 校验 tags JSON 列：集合等于并集，且无重复（向量长度等于集合大小）
            let final_tags = read_tags_json(&conn, id);
            let final_set: std::collections::HashSet<String> =
                final_tags.iter().cloned().collect();
            prop_assert_eq!(
                final_set.clone(),
                oracle.clone(),
                "tags JSON 列应等于全部参与标签的并集"
            );
            prop_assert_eq!(
                final_tags.len(),
                oracle.len(),
                "tags JSON 列不应包含重复标签"
            );

            // 校验 entry_tags 关联表：每个不同标签恰好关联一次（集合等于并集）
            let assoc = read_entry_tag_associations(&conn, id);
            let assoc_set: std::collections::HashSet<String> = assoc.iter().cloned().collect();
            prop_assert_eq!(
                assoc_set,
                oracle,
                "entry_tags 关联应等于全部参与标签的并集"
            );
            prop_assert_eq!(
                assoc.len(),
                final_tags.len(),
                "每个不同标签在结果条目上应恰好关联一次"
            );
        }
    }

    // Feature: magpie-v0-4-1, Property 5: 合并数值不变量（计数求和与置顶保留）
    //
    // 对任意参与重复内容合并的条目集合，合并结果条目的使用次数（use_count）应等于各参与
    // 条目使用次数之和；且当且仅当至少一个参与条目处于置顶（pinned）状态时，合并结果条目
    // 处于置顶状态。同时验证：已有条目置顶时其 pinned_order 在后续合并中得到保留。
    //
    // 直接驱动 save_with_conn 的 id>0 UPDATE 分支：先以原始 SQL 播种一条带初始 use_count /
    // is_pinned / pinned_order 的已有条目，再依次以"新捕获"条目合并，最终校验数值不变量。
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]
        #[test]
        fn prop_merge_numeric_invariants(
            init_use_count in 0i32..=1000,
            init_pinned in any::<bool>(),
            init_pinned_order in 0i64..=100,
            // 每次合并的新捕获条目：使用次数、是否置顶、置顶序
            incoming in proptest::collection::vec(
                (0i32..=1000, any::<bool>(), 0i64..=100),
                1..=12,
            )
        ) {
            let conn = setup_test_db();
            let repo = make_repo();

            // 播种已有条目（携带初始数值），模拟去重命中的"保留条目"
            conn.execute(
                "INSERT INTO clipboard_history
                    (content_type, content, source_app, timestamp, preview,
                     is_pinned, content_hash, tags, use_count, is_external, pinned_order)
                 VALUES ('text', 'dup-content', 'seed', 100, 'dup', ?1, 0, '[]', ?2, 0, ?3)",
                params![init_pinned as i32, init_use_count, init_pinned_order],
            )
            .unwrap();
            let id = conn.last_insert_rowid();

            // 依次合并新捕获条目
            for (idx, (uc, pin, po)) in incoming.iter().enumerate() {
                let mut entry = make_text_entry(id, "dup-content", vec![]);
                entry.timestamp = 200 + idx as i64;
                entry.use_count = *uc;
                entry.is_pinned = *pin;
                entry.pinned_order = *po;
                repo.save_with_conn(&conn, &entry, None).unwrap();
            }

            // 读回合并结果数值
            let (final_uc, final_pinned, final_po): (i64, i32, i64) = conn
                .query_row(
                    "SELECT use_count, is_pinned, pinned_order FROM clipboard_history WHERE id = ?1",
                    params![id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();

            // 不变量 1：use_count 等于各参与条目（初始 + 全部新捕获）使用次数之和
            let expected_sum: i64 =
                init_use_count as i64 + incoming.iter().map(|(uc, _, _)| *uc as i64).sum::<i64>();
            prop_assert_eq!(
                final_uc,
                expected_sum,
                "合并结果 use_count 应等于各参与条目使用次数之和"
            );

            // 不变量 2：当且仅当任一参与条目置顶时，结果置顶
            let expected_pinned = init_pinned || incoming.iter().any(|(_, pin, _)| *pin);
            prop_assert_eq!(
                final_pinned == 1,
                expected_pinned,
                "当且仅当存在置顶参与条目时合并结果才置顶"
            );

            // 不变量 3：已有条目初始即置顶时，其 pinned_order 在合并中得到保留
            if init_pinned {
                prop_assert_eq!(
                    final_po,
                    init_pinned_order,
                    "已置顶条目的 pinned_order 应在合并中保留"
                );
            }
        }
    }

    // 任务 2.4（需求 11.5）：去重命中时更新已有条目，不新建独立重复条目
    //
    // 验证：当新捕获内容命中一条已有条目时，以该已有条目为保留条目执行 UPDATE
    // （更新最近使用时间 / 提升排序位置），条目总数不增加，不产生重复条目。
    #[test]
    fn dedup_hit_updates_existing_entry_without_creating_duplicate() {
        let conn = setup_test_db();
        let repo = SqliteClipboardRepository::new(Arc::new(Mutex::new(conn)));

        // 先写入两条不同内容：A（较旧）、B（较新，位于列表顶部）
        let mut entry_a = make_text_entry(0, "content-A", vec![]);
        entry_a.timestamp = 100;
        let id_a = repo.save(&entry_a, None).unwrap();

        let mut entry_b = make_text_entry(0, "content-B", vec![]);
        entry_b.timestamp = 200;
        let _id_b = repo.save(&entry_b, None).unwrap();

        assert_eq!(repo.get_count().unwrap(), 2, "初始应有两条条目");
        let history = repo.get_history(10, 0, None).unwrap();
        assert_eq!(history[0].content, "content-B", "较新的 B 应位于列表顶部");

        // 模拟再次复制 A：去重命中已有条目 id_a（pipeline 会将 entry.id 设为命中 id）
        let hit_id = repo
            .get_entry_by_content("content-A", Some("text"))
            .unwrap()
            .expect("应命中已有条目 A");
        assert_eq!(hit_id, id_a, "去重应命中已有的 A 条目");

        let mut recapture = make_text_entry(hit_id, "content-A", vec![]);
        recapture.timestamp = 300; // 更新的最近使用时间
        repo.save(&recapture, None).unwrap();

        // 不应新建条目：总数仍为 2
        assert_eq!(
            repo.get_count().unwrap(),
            2,
            "去重命中应更新已有条目，不新建重复条目"
        );

        // 已有条目的最近使用时间应被更新，并因更新而提升到列表顶部
        let updated = repo
            .get_entry_by_id(id_a)
            .unwrap()
            .expect("A 条目应仍存在");
        assert_eq!(updated.timestamp, 300, "应更新已有条目的最近使用时间");

        let history_after = repo.get_history(10, 0, None).unwrap();
        assert_eq!(
            history_after[0].id, id_a,
            "更新后的 A 应提升到列表顶部"
        );
    }
}
