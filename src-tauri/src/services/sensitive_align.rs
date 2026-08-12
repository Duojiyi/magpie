use std::thread;
use std::time::Duration;
use rusqlite::params;
use tauri::{AppHandle, Manager};

use crate::database::{DbState, SENSITIVE_TAGS};
use crate::infrastructure::encryption;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

pub fn spawn_sensitive_alignment(app_handle: AppHandle) {
    thread::spawn(move || run_alignment(app_handle));
}

fn run_alignment(app_handle: AppHandle) {
    let db_state = app_handle.state::<DbState>();
    let done = db_state
        .settings_repo
        .get("db.sensitive_alignment_done")
        .unwrap_or(None)
        .unwrap_or_else(|| "false".to_string());
    if done == "true" {
        return;
    }

    let sensitive_tags_sql = {
        let parts: Vec<String> = SENSITIVE_TAGS
            .iter()
            .map(|t| format!("'{}'", t.replace('\'', "''")))
            .collect();
        format!("({})", parts.join(","))
    };

    // 加密前缀用于在 SQL 内直接判断各字段是否已加密，避免把完整内容读进内存。
    // 两种方案都要匹配：Windows 用 DPAPI，其它平台用可移植方案；只认其中一种会让另一平台上
    // 已加密的行被判定为「未加密」，从而被反复重写。
    let enc_like = format!("{}%", encryption::ENCRYPTED_PREFIXES[0]);
    let enc_like_alt = format!("{}%", encryption::ENCRYPTED_PREFIXES[1]);

    let mut cursor_ts = i64::MAX;
    let mut cursor_id = i64::MAX;
    let batch_size = 200;

    loop {
        let conn_guard = match db_state.conn.lock() {
            Ok(c) => c,
            Err(_) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        };

        // 仅取对齐决策所需的标量（id/时间戳/各加密布尔标志），不再加载 content/preview/html_content 大字符串，
        // 使消费队列的内存占用与条目内容体积无关、严格有界，避免大文本条目导致的内存激增（需求 22.2）。
        let sql = format!(
            "SELECT ch.id, ch.timestamp,
                    COALESCE(ch.content LIKE ?4 OR ch.content LIKE ?5, 0) AS content_encrypted,
                    COALESCE(ch.preview LIKE ?4 OR ch.preview LIKE ?5, 0) AS preview_encrypted,
                    (ch.html_content IS NOT NULL) AS has_html,
                    COALESCE(ch.html_content LIKE ?4 OR ch.html_content LIKE ?5, 0) AS html_encrypted,
                    EXISTS (
                        SELECT 1 FROM entry_tags se
                        WHERE se.entry_id = ch.id
                          AND se.tag COLLATE NOCASE IN {}
                    ) AS is_sensitive
             FROM clipboard_history ch
             WHERE (ch.timestamp < ?1) OR (ch.timestamp = ?1 AND ch.id < ?2)
             ORDER BY ch.timestamp DESC, ch.id DESC
             LIMIT ?3",
            sensitive_tags_sql
        );

        // 每批仅保留定长标量元组，单批内存上限固定为 batch_size 条 × 几个整型，处理后即被释放
        let mut batch: Vec<(i64, i64, bool, bool, bool, bool, bool)> = Vec::new();
        {
            let mut stmt = match conn_guard.prepare(&sql) {
                Ok(s) => s,
                Err(_) => break,
            };

            let rows = match stmt.query_map(
                params![cursor_ts, cursor_id, batch_size, enc_like, enc_like_alt],
                |row| {
                    let id: i64 = row.get(0)?;
                    let ts: i64 = row.get(1)?;
                    let content_encrypted: bool = row.get(2)?;
                    let preview_encrypted: bool = row.get(3)?;
                    let has_html: bool = row.get(4)?;
                    let html_encrypted: bool = row.get(5)?;
                    let is_sensitive: i32 = row.get(6)?;
                    Ok((
                        id,
                        ts,
                        content_encrypted,
                        preview_encrypted,
                        has_html,
                        html_encrypted,
                        is_sensitive == 1,
                    ))
                },
            ) {
                Ok(r) => r,
                Err(_) => break,
            };

            for row in rows {
                if let Ok(item) = row {
                    batch.push(item);
                }
            }
        }

        if batch.is_empty() {
            break;
        }

        for (id, _ts, content_encrypted, preview_encrypted, has_html, html_encrypted, is_sensitive) in
            batch.iter()
        {
            if *is_sensitive
                && (!content_encrypted || !preview_encrypted || (*has_html && !html_encrypted))
            {
                // 敏感但未完全加密：补加密
                let _ = db_state.repo.encrypt_entry_with_conn(&conn_guard, *id);
            } else if !*is_sensitive
                && (*content_encrypted || *preview_encrypted || *html_encrypted)
            {
                // 非敏感但残留加密：解密还原
                let _ = db_state.repo.decrypt_entry_with_conn(&conn_guard, *id);
            }
        }

        if let Some((id, ts, ..)) = batch.last() {
            cursor_ts = *ts;
            cursor_id = *id;
        } else {
            break;
        }

        // 处理完即释放本批数据与连接锁，确保内存随批次回收、队列不会无界增长
        drop(batch);
        drop(conn_guard);
        thread::sleep(Duration::from_millis(50));
    }

    let _ = db_state
        .settings_repo
        .set("db.sensitive_alignment_done", "true");
}
