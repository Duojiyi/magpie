use axum::{
    body::Body,
    extract::{
        ws::{Message as WsMessage, WebSocket},
        Multipart, Path, Query, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::app_state::{SessionHistory, SettingsState};
use crate::database::ClipboardEntry;
use crate::database::DbState;
use crate::infrastructure::repository::clipboard_repo::ClipboardRepository;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

use super::models::*;
use super::utils::*;
use super::web_ui::render_index;
use super::{append_message, register_received_file};

pub async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let app_handle = &state.app_handle;
    let settings = app_handle.state::<SettingsState>();
    let db_state = app_handle.state::<DbState>();
    let logo_base64 = get_app_logo_base64(app_handle);
    let theme = {
        let guard = settings.theme.lock().unwrap();
        guard.clone()
    };
    let color_mode = db_state
        .settings_repo
        .get("app.color_mode")
        .ok()
        .flatten()
        .unwrap_or_else(|| "system".to_string());

    Html(render_index(&theme, &color_mode, &logo_base64))
}

pub async fn poll_messages(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<Message>> {
    let last_id = params
        .get("last_id")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let chat_state = state.app_handle.state::<ChatState>();

    let msgs_result = {
        match chat_state.0.lock() {
            Ok(msgs) => msgs
                .iter()
                .filter(|m| m.id > last_id)
                .map(|m| {
                    let mut m_clone = m.clone();
                    if m.msg_type == "image"
                        && !m.content.starts_with("data:")
                        && !m.content.starts_with("/download/")
                    {
                        let token = stable_poll_token(m.id);
                        let mut filename = "image.png".to_string();
                        let path = std::path::Path::new(&m.content);
                        if let Some(name) = path.file_name() {
                            filename = name.to_string_lossy().to_string();
                        }

                        let shared_files = state.app_handle.state::<SharedFileState>();
                        if let Ok(mut map) = shared_files.0.lock() {
                            map.insert(token.clone(), m.content.clone());
                        }
                        m_clone.content = format!(
                            "/download/{}?name={}",
                            token,
                            urlencoding::encode(&filename)
                        );
                    }
                    m_clone
                })
                .collect::<Vec<Message>>(),
            Err(_) => vec![],
        }
    };

    Json(msgs_result)
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum WsIncoming {
    #[serde(rename = "identity")]
    Identity {
        device_id: String,
        device_name: String,
    },
}

pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();
    // Every identity this socket announced. Cleaned up after the select below: doing it at
    // the end of recv_task skipped it whenever send_task finished first (recv_task is then
    // aborted), leaving the device listed as online.
    let announced: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();

    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(WsMessage::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                // A slow client missed some messages; keep the socket instead of dropping it.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let state_inner = state.clone();
    let announced_inner = announced.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                    match incoming {
                        WsIncoming::Identity {
                            device_id,
                            device_name,
                        } => {
                            let online_devices = state_inner.app_handle.state::<OnlineDevices>();
                            {
                                let mut guard = online_devices.0.lock().unwrap();
                                guard.insert(
                                    device_id.clone(),
                                    DeviceInfo {
                                        id: device_id.clone(),
                                        name: device_name,
                                        last_seen: chrono::Utc::now().timestamp_millis(),
                                    },
                                );
                                let mut ids = announced_inner.lock().unwrap();
                                if !ids.contains(&device_id) {
                                    ids.push(device_id);
                                }
                                drop(ids);

                                let devices: Vec<DeviceInfo> = guard.values().cloned().collect();
                                let update = serde_json::json!({
                                    "type": "devices_update",
                                    "devices": devices
                                });
                                let _ = state_inner.ws_tx.send(update.to_string());
                                let _ = state_inner
                                    .app_handle
                                    .emit("online-devices-updated", devices);
                            }
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
        // Server switched off: close this socket too (an upgraded WebSocket outlives the
        // aborted accept loop).
        _ = state.shutdown.cancelled() => {
            send_task.abort();
            recv_task.abort();
        }
    }

    let ids = std::mem::take(&mut *announced.lock().unwrap());
    if !ids.is_empty() {
        let online_devices = state.app_handle.state::<OnlineDevices>();
        let mut guard = online_devices.0.lock().unwrap();
        for id in &ids {
            guard.remove(id);
        }
        let devices: Vec<DeviceInfo> = guard.values().cloned().collect();
        drop(guard);
        let update = serde_json::json!({
            "type": "devices_update",
            "devices": devices
        });
        let _ = state.ws_tx.send(update.to_string());
        let _ = state.app_handle.emit("online-devices-updated", devices);
    }
}

pub async fn handle_text(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReceiveText>,
) -> axum::response::Response {
    let db_state = state.app_handle.state::<DbState>();
    update_activity(&state.app_handle);

    let sender_id = if payload.sender_id.is_empty() {
        "mobile"
    } else {
        &payload.sender_id
    };
    let sender_name = if payload.sender_name.is_empty() {
        "手机"
    } else {
        &payload.sender_name
    };

    append_message(
        &state.app_handle,
        "in",
        "text",
        &payload.content,
        sender_id,
        sender_name,
        None,
    );

    let settings = state.app_handle.state::<SettingsState>();
    let session_hist = state.app_handle.state::<SessionHistory>();

    let mut preview = payload.content.clone();
    if preview.chars().count() > 100 {
        preview = preview.chars().take(100).collect();
        preview.push_str("...");
    }

    let id_result = if settings.auto_copy_file.load(Ordering::Relaxed) {
        if settings.persistent.load(Ordering::Relaxed) {
            let entry = ClipboardEntry {
                id: 0,
                content_type: "text".to_string(),
                content: payload.content.clone(),
                html_content: None,
                source_app: sender_name.to_string(),
                source_app_path: None,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
                preview: preview.clone(),
                is_pinned: false,
                tags: Vec::new(),
                use_count: 0,
                is_external: false,
                pinned_order: 0,
                file_preview_exists: true,
            };
            db_state.repo.save(&entry, None).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })
        } else {
            let id = -(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as i64
                / 1000);
            let entry = ClipboardEntry {
                id,
                content_type: "text".to_string(),
                content: payload.content.clone(),
                html_content: None,
                source_app: "File Transfer".to_string(),
                source_app_path: None,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
                preview: preview.clone(),
                is_pinned: false,
                tags: Vec::new(),
                use_count: 0,
                is_external: false,
                pinned_order: 0,
                file_preview_exists: true,
            };

            if let Ok(mut session) = session_hist.0.lock() {
                session.push_back(entry);
                for removed in crate::app_state::trim_session_history(&mut session) {
                    let _ = state.app_handle.emit("clipboard-removed", removed);
                }
            }
            Ok(id)
        }
    } else {
        // Auto-copy off: the message was delivered to the chat above; nothing else to save.
        return (StatusCode::OK, "Text received").into_response();
    };

    if let Ok(id) = id_result {
        if id != 0 {
            let _ = state.app_handle.emit("clipboard-changed", id);
            return (StatusCode::OK, "Text received").into_response();
        }
    }
    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save text").into_response()
}

/// Creates `<dir>/<timestamp>_<name>` (or `<timestamp>_<n>_<name>` if taken) exclusively.
/// A per-second name alone let same-named uploads finishing in the same second (iOS names
/// every picked photo `image.jpg`) overwrite each other.
async fn create_unique_upload_file(
    dir: &std::path::Path,
    file_name: &str,
) -> std::io::Result<(std::path::PathBuf, File)> {
    let stamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let name = sanitize_upload_filename(file_name);
    for n in 0..1000u32 {
        let candidate = if n == 0 {
            dir.join(format!("{}_{}", stamp, name))
        } else {
            dir.join(format!("{}_{}_{}", stamp, n, name))
        };
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
            .await
        {
            Ok(file) => return Ok((candidate, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no free upload file name",
    ))
}

pub async fn upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    let mut success = false;
    let db_state = state.app_handle.state::<DbState>();

    let mut current_sender_id = "mobile".to_string();
    let mut current_sender_name = "手机".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "sender_id" {
            if let Ok(val) = field.text().await {
                current_sender_id = val;
            }
            continue;
        }
        if name == "sender_name" {
            if let Ok(val) = field.text().await {
                current_sender_name = val;
            }
            continue;
        }

        if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown.txt").to_string();
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let mut save_dir = state
                .app_handle
                .path()
                .download_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            if let Ok(Some(custom)) = db_state.settings_repo.get("file_transfer_path") {
                if !custom.trim().is_empty() {
                    save_dir = std::path::PathBuf::from(custom);
                }
            }
            if !save_dir.exists() {
                let _ = std::fs::create_dir_all(&save_dir);
            }

            if let Ok((target_path, mut file)) =
                create_unique_upload_file(&save_dir, &file_name).await
            {
                let mut stream = field;
                let mut write_success = true;
                // `while let Some(Ok(..))` treated a stream error (client disconnect, body
                // limit) like EOF and registered the truncated file as received.
                while let Some(next) = stream.next().await {
                    let chunk = match next {
                        Ok(chunk) => chunk,
                        Err(e) => {
                            eprintln!("Upload stream aborted: {}", e);
                            write_success = false;
                            break;
                        }
                    };
                    if let Err(e) = file.write_all(&chunk).await {
                        eprintln!("Error writing: {}", e);
                        write_success = false;
                        break;
                    }
                }
                if write_success {
                    write_success = file.flush().await.is_ok();
                }
                if !write_success {
                    drop(file);
                    let _ = tokio::fs::remove_file(&target_path).await;
                }

                if write_success {
                    register_received_file(
                        &state.app_handle,
                        target_path,
                        file_name,
                        content_type,
                        current_sender_id.clone(),
                        current_sender_name.clone(),
                    )
                    .await;
                    success = true;
                }
            }
        }
    }

    if success {
        (StatusCode::OK, "Upload successful").into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Upload failed").into_response()
    }
}

pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    let mut metadata: Option<ChunkMetadata> = None;
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "metadata" {
            if let Ok(bytes) = field.bytes().await {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(m) = serde_json::from_str(&text) {
                        metadata = Some(m);
                    }
                }
            }
        } else if name == "data" || name == "file" {
            if let Ok(bytes) = field.bytes().await {
                chunk_data = Some(bytes.to_vec());
            }
        }
    }

    let meta = match metadata {
        Some(m) => m,
        None => return (StatusCode::BAD_REQUEST, "Missing metadata").into_response(),
    };
    let data = match chunk_data {
        Some(d) => d,
        None => return (StatusCode::BAD_REQUEST, "Missing data").into_response(),
    };

    // Guard against a zero total: `chunk_index == total_chunks - 1` below would underflow
    // (usize) and, in release (`panic = "abort"` is off for this arithmetic but the wrap
    // yields a bogus huge index), the upload would never finalize and leak the temp file.
    if meta.total_chunks == 0 || meta.chunk_index >= meta.total_chunks {
        return (StatusCode::BAD_REQUEST, "Invalid chunk indices").into_response();
    }

    let sessions = state.app_handle.state::<UploadSessions>();
    let now = std::time::Instant::now();
    const UPLOAD_SESSION_TTL: std::time::Duration = std::time::Duration::from_secs(600);

    // Pass 1: evict sessions idle beyond the TTL (closed tab / lost Wi-Fi / failed finalize)
    // so they free their capacity slot, and delete their leftover temp files. Collect under
    // the lock, then delete after releasing it — and BEFORE any early return below, so the
    // 409/429 paths still perform the cleanup.
    let stale_temp_files: Vec<std::path::PathBuf> = {
        let mut sessions_map = sessions.0.lock().unwrap();
        let stale = sessions_map
            .iter()
            .filter(|(_, (_, last))| now.duration_since(*last) >= UPLOAD_SESSION_TTL)
            .map(|(_, (path, _))| path.clone())
            .collect();
        sessions_map.retain(|_, (_, last)| now.duration_since(*last) < UPLOAD_SESSION_TTL);
        stale
    };
    for stale in stale_temp_files {
        let _ = tokio::fs::remove_file(&stale).await;
    }

    // Pass 2: resolve (or create) this upload's session + temp path.
    let temp_path = {
        let mut sessions_map = sessions.0.lock().unwrap();
        // A chunk with index > 0 for a session we don't have means it expired / was evicted
        // (or never started). Tell the client to restart with a fresh upload rather than
        // silently truncating; chunk 0 below always starts the temp file clean.
        if meta.chunk_index != 0 && !sessions_map.contains_key(&meta.upload_id) {
            return (
                StatusCode::CONFLICT,
                "Upload session expired; restart the upload with a new upload_id",
            )
                .into_response();
        }
        // Reject brand-new sessions once at capacity so a malicious LAN client can't open
        // unlimited chunked uploads and exhaust memory/disk (P1). A resuming session
        // (already tracked) is always allowed to continue.
        if !sessions_map.contains_key(&meta.upload_id)
            && sessions_map.len() >= super::MAX_UPLOAD_SESSIONS
        {
            return (StatusCode::TOO_MANY_REQUESTS, "Too many upload sessions").into_response();
        }
        let entry = sessions_map.entry(meta.upload_id.clone()).or_insert_with(|| {
            let mut path = state
                .app_handle
                .path()
                .download_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            if let Ok(Some(custom)) = state
                .app_handle
                .state::<DbState>()
                .settings_repo
                .get("file_transfer_path")
            {
                if !custom.trim().is_empty() {
                    path = std::path::PathBuf::from(custom);
                }
            }
            if !path.exists() {
                let _ = std::fs::create_dir_all(&path);
            }
            // Hash the raw upload_id for the temp name: collision-resistant (unlike the
            // many-to-one sanitize_upload_filename) so distinct sessions don't share a file.
            (
                path.join(format!(".tmp_{}", stable_upload_temp_name(&meta.upload_id))),
                now,
            )
        });
        entry.1 = now; // refresh last-activity on each received chunk
        entry.0.clone()
    };

    // Chunk 0 starts the temp file clean; later chunks append. Truncating in the same open
    // call (rather than a separate remove) is robust even if a delete would fail (Windows
    // handle contention) and closes the window where a concurrent rebuild of the same
    // deterministic path could race a delete — so we never append onto stale bytes and
    // finalize a silently-corrupt file.
    // Chunks are appended blindly, so check each lands where it belongs (uniform chunk size,
    // the last one ends at total_size). A skipped or repeated chunk would otherwise finalize
    // a corrupt file and register it as received.
    let is_last = meta.chunk_index == meta.total_chunks - 1;
    let len = data.len() as u64;
    let expected_offset = if is_last {
        meta.total_size.checked_sub(len)
    } else {
        (meta.chunk_index as u64).checked_mul(len)
    };
    let Some(expected_offset) = expected_offset else {
        return (StatusCode::BAD_REQUEST, "Chunk size mismatch").into_response();
    };
    if meta.chunk_index == 0 {
        if expected_offset != 0 {
            return (StatusCode::BAD_REQUEST, "Chunk size mismatch").into_response();
        }
    } else {
        let current = tokio::fs::metadata(&temp_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        if current == expected_offset + len && !is_last {
            // A retry of a chunk that already landed.
            return (StatusCode::OK, "Chunk received").into_response();
        }
        if current != expected_offset {
            sessions.0.lock().unwrap().remove(&meta.upload_id);
            let _ = tokio::fs::remove_file(&temp_path).await;
            return (
                StatusCode::CONFLICT,
                "Chunk out of order; restart the upload with a new upload_id",
            )
                .into_response();
        }
    }

    let mut options = tokio::fs::OpenOptions::new();
    if meta.chunk_index == 0 {
        options.create(true).write(true).truncate(true);
    } else {
        options.create(true).append(true).write(true);
    }

    if let Ok(mut file) = options.open(&temp_path).await {
        // Flush before replying: tokio's File finishes writes in the background, and the next
        // chunk's offset check (and the finalize rename) must see these bytes on disk.
        let written = match file.write_all(&data).await {
            Ok(()) => file.flush().await,
            Err(e) => Err(e),
        };
        if let Err(e) = written {
            eprintln!("Error writing chunk: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Write failed").into_response();
        }
    } else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Open failed").into_response();
    }

    if meta.chunk_index == meta.total_chunks - 1 {
        // Reserve a name no other upload holds, then move the temp file over the (empty)
        // reservation. The handle is closed first: Windows cannot replace an open file.
        let reserved = match temp_path.parent() {
            Some(parent) => create_unique_upload_file(parent, &meta.file_name).await,
            None => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid temp path").into_response()
            }
        };
        let rename_result = match reserved {
            Ok((final_path, reservation)) => {
                drop(reservation);
                match tokio::fs::rename(&temp_path, &final_path).await {
                    Ok(()) => Ok(final_path),
                    Err(e) => {
                        let _ = tokio::fs::remove_file(&final_path).await;
                        Err(e)
                    }
                }
            }
            Err(e) => Err(e),
        };

        let final_path = match rename_result {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Error finalizing file: {}", e);
                // Drop the session + temp file so a failed finalize doesn't leak a slot/file.
                {
                    let mut sessions_map = sessions.0.lock().unwrap();
                    sessions_map.remove(&meta.upload_id);
                }
                let _ = tokio::fs::remove_file(&temp_path).await;
                return (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response();
            }
        };

        {
            let mut sessions_map = sessions.0.lock().unwrap();
            sessions_map.remove(&meta.upload_id);
        }

        let content_type = meta
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
        register_received_file(
            &state.app_handle,
            final_path,
            meta.file_name,
            content_type,
            meta.sender_id,
            meta.sender_name,
        )
        .await;

        return (StatusCode::OK, "Upload complete").into_response();
    }

    (StatusCode::OK, "Chunk received").into_response()
}

/// Parses a single `Range: bytes=...` header against a file of `total` bytes.
/// `Ok(Some((start, end)))` is an inclusive satisfiable range, `Ok(None)` means ignore the
/// header and serve the whole file (malformed, multi-range, `end < start`), and `Err(())`
/// means 416 (start past the end, empty suffix, empty file).
fn parse_byte_range(header: &str, total: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(spec) = header.trim().strip_prefix("bytes=") else {
        return Ok(None);
    };
    if spec.contains(',') {
        return Ok(None);
    }
    let Some((first, last)) = spec.split_once('-') else {
        return Ok(None);
    };
    let (first, last) = (first.trim(), last.trim());
    if first.is_empty() {
        // Suffix range `bytes=-N`: the last N bytes.
        let Ok(suffix) = last.parse::<u64>() else {
            return Ok(None);
        };
        if suffix == 0 || total == 0 {
            return Err(());
        }
        return Ok(Some((total.saturating_sub(suffix), total - 1)));
    }
    let Ok(start) = first.parse::<u64>() else {
        return Ok(None);
    };
    let end = if last.is_empty() {
        u64::MAX
    } else {
        match last.parse::<u64>() {
            Ok(end) => end,
            Err(_) => return Ok(None),
        }
    };
    if start >= total {
        return Err(());
    }
    if end < start {
        return Ok(None);
    }
    Ok(Some((start, end.min(total - 1))))
}

#[cfg(test)]
mod range_tests {
    use super::parse_byte_range;

    #[test]
    fn byte_ranges() {
        assert_eq!(parse_byte_range("bytes=0-9", 100), Ok(Some((0, 9))));
        assert_eq!(parse_byte_range("bytes=90-", 100), Ok(Some((90, 99))));
        assert_eq!(parse_byte_range("bytes=90-500", 100), Ok(Some((90, 99))));
        assert_eq!(parse_byte_range("bytes=-10", 100), Ok(Some((90, 99))));
        assert_eq!(parse_byte_range("bytes=-500", 100), Ok(Some((0, 99))));
        assert_eq!(parse_byte_range("bytes=100-", 100), Err(()));
        assert_eq!(parse_byte_range("bytes=-0", 100), Err(()));
        assert_eq!(parse_byte_range("bytes=5-2", 100), Ok(None));
        assert_eq!(parse_byte_range("bytes=0-1,5-6", 100), Ok(None));
        assert_eq!(parse_byte_range("items=0-1", 100), Ok(None));
    }
}

pub async fn handle_file_download_proxy(
    Path(token): Path<String>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let app_handle = &state.app_handle;
    update_activity(app_handle);
    let shared_state = app_handle.state::<SharedFileState>();

    let file_path = {
        let guard = shared_state.0.lock().unwrap();
        guard.get(&token).cloned()
    };

    if let Some(path_str) = file_path {
        let path = std::path::PathBuf::from(&path_str);
        if path.exists() {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let is_image = mime.starts_with("image/");
            let is_video = mime.starts_with("video/");
            let encoded_name = urlencoding::encode(&filename);
            // Sanitize the plain-ASCII fallback `filename=` value: strip quotes/backslashes
            // and control chars that could break out of the Content-Disposition header or
            // inject a new header (CRLF). The accurate name is carried by the RFC 5987
            // `filename*` below, so this only affects legacy fallback rendering (P3).
            let header_safe_name: String = filename
                .chars()
                .map(|c| match c {
                    '"' | '\\' => '_',
                    c if (c as u32) < 0x20 => '_',
                    c => c,
                })
                .collect();
            let disposition = if is_image || is_video {
                format!(
                    "inline; filename=\"{}\"; filename*=UTF-8''{}",
                    header_safe_name, encoded_name
                )
            } else {
                format!(
                    "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                    header_safe_name, encoded_name
                )
            };

            if let Ok(mut file) = File::open(&path).await {
                let metadata = match file.metadata().await {
                    Ok(m) => m,
                    Err(_) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Metadata failed")
                            .into_response()
                    }
                };
                let total_size = metadata.len();
                let range_header = headers.get(header::RANGE).and_then(|h| h.to_str().ok());

                if let Some(range) = range_header {
                    let parsed = parse_byte_range(range, total_size);
                    if parsed.is_err() {
                        return (
                            StatusCode::RANGE_NOT_SATISFIABLE,
                            [(header::CONTENT_RANGE, format!("bytes */{}", total_size))],
                        )
                            .into_response();
                    }
                    if let Ok(Some((start, end))) = parsed {
                        let content_length = end - start + 1;

                        if file.seek(SeekFrom::Start(start)).await.is_ok() {
                            let stream =
                                ReaderStream::with_capacity(file.take(content_length), 64 * 1024);
                            let body = Body::from_stream(stream);

                            return (
                                StatusCode::PARTIAL_CONTENT,
                                [
                                    (header::CONTENT_TYPE, mime),
                                    (header::CONTENT_DISPOSITION, disposition),
                                    (header::ACCEPT_RANGES, "bytes".to_string()),
                                    (
                                        header::CONTENT_RANGE,
                                        format!("bytes {}-{}/{}", start, end, total_size),
                                    ),
                                    (header::CONTENT_LENGTH, content_length.to_string()),
                                ],
                                body,
                            )
                                .into_response();
                        }
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Seek failed").into_response();
                    }
                }

                let stream = ReaderStream::with_capacity(file, 64 * 1024);
                let body = Body::from_stream(stream);

                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime),
                        (header::CONTENT_DISPOSITION, disposition),
                        (header::ACCEPT_RANGES, "bytes".to_string()),
                        (header::CONTENT_LENGTH, total_size.to_string()),
                    ],
                    body,
                )
                    .into_response();
            }
        }
    }

    (StatusCode::NOT_FOUND, "File not found").into_response()
}
