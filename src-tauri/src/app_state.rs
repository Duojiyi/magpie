use crate::domain::models::ClipboardEntry;
use crate::services::encryption_queue::EncryptionQueue;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct SettingsState {
    pub deduplicate: AtomicBool,
    pub persistent: AtomicBool,
    pub file_server_auto_close: AtomicBool,
    pub theme: Mutex<String>,
    pub capture_files: AtomicBool,
    pub capture_rich_text: AtomicBool,
    pub auto_copy_file: AtomicBool,
    pub silent_start: AtomicBool,
    pub delete_after_paste: AtomicBool,
    pub privacy_protection: AtomicBool,
    pub privacy_protection_kinds: Mutex<Vec<String>>,
    pub privacy_protection_custom_rules: Mutex<Vec<String>>,
    pub cleanup_rules: Mutex<String>,
    pub app_cleanup_policies: Mutex<String>,
    pub sequential_mode: AtomicBool,
    pub sequential_paste_hotkey: Mutex<String>,
    pub rich_paste_hotkey: Mutex<String>,
    pub search_hotkey: Mutex<String>,
    pub quick_paste_modifier: Mutex<String>,
    pub sound_enabled: AtomicBool,
    pub hide_tray_icon: AtomicBool,
    pub edge_docking: AtomicBool,
    pub follow_mouse: AtomicBool,
    pub arrow_key_selection: AtomicBool,
    pub main_hotkey: Mutex<String>,
    pub monitors: Mutex<Vec<tauri::Monitor>>,
}

#[derive(Default)]
pub struct PasteQueueState {
    pub items: VecDeque<i64>,
    pub last_action_was_paste: bool,
    pub last_pasted_content: Option<String>,
    pub last_pasted_fingerprint: Option<String>,
    pub last_paste_timestamp_ms: u64,
}

#[derive(Default)]
pub struct PasteQueue(pub Mutex<PasteQueueState>);

pub struct SessionHistory(pub Mutex<VecDeque<ClipboardEntry>>);

/// Session history cap.
pub const SESSION_HISTORY_LIMIT: usize = 500;

/// Trims `session` to [`SESSION_HISTORY_LIMIT`], dropping the oldest session-only items
/// first, and returns the ids the UI must be told were removed. Promoted items (id > 0:
/// pinned/tagged, saved to the database) still exist, so they are never reported removed;
/// they are only dropped (silently) when nothing else is left to evict.
pub fn trim_session_history(session: &mut VecDeque<ClipboardEntry>) -> Vec<i64> {
    let mut removed = Vec::new();
    while session.len() > SESSION_HISTORY_LIMIT {
        match session.iter().position(|item| item.id < 0) {
            Some(index) => {
                if let Some(item) = session.remove(index) {
                    removed.push(item.id);
                }
            }
            None => {
                session.pop_front();
            }
        }
    }
    removed
}

pub struct AppDataDir(pub Mutex<std::path::PathBuf>);

pub struct EncryptionQueueState(pub EncryptionQueue);
