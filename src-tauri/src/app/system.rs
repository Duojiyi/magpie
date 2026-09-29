use crate::global_state::TASKBAR_CREATED_MSG;
use std::sync::atomic::Ordering;
use tauri::Manager;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::DefSubclassProc;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::WM_DISPLAYCHANGE;

#[cfg(target_os = "windows")]
const NOTIFY_FOR_THIS_SESSION: u32 = 0;
#[cfg(target_os = "windows")]
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
#[cfg(target_os = "windows")]
const WTS_CONSOLE_CONNECT: usize = 0x1;
#[cfg(target_os = "windows")]
const WTS_REMOTE_CONNECT: usize = 0x3;
#[cfg(target_os = "windows")]
const WTS_SESSION_LOGON: usize = 0x5;
#[cfg(target_os = "windows")]
const WTS_SESSION_UNLOCK: usize = 0x8;
#[cfg(target_os = "windows")]
const WTS_SESSION_REMOTE_CONTROL: usize = 0x9;

#[cfg(target_os = "windows")]
#[link(name = "wtsapi32")]
extern "system" {
    fn WTSRegisterSessionNotification(hwnd: HWND, dwflags: u32) -> i32;
}

/// 获取硬件机器码（基于硬件唯一标识）
/// 返回格式: 8字符的十六进制字符串 (例如: "ef785433")
pub fn get_machine_id() -> String {
    use sha2::{Digest, Sha256};

    match machine_uid::get() {
        Ok(machine_uid) => {
            let mut hasher = Sha256::new();
            hasher.update(machine_uid.as_bytes());
            let result = hasher.finalize();
            let hex = format!("{:x}", result);
            hex.chars().take(8).collect()
        }
        Err(e) => {
            eprintln!("[WARN] Failed to get machine UID: {}. Using fallback.", e);
            let mut hasher = Sha256::new();
            if let Ok(computer_name) = std::env::var("COMPUTERNAME") {
                hasher.update(computer_name.as_bytes());
            }
            if let Ok(username) = std::env::var("USERNAME") {
                hasher.update(username.as_bytes());
            }
            let result = hasher.finalize();
            let hex = format!("{:x}", result);
            hex.chars().take(8).collect()
        }
    }
}

pub fn build_anon_id(machine_id: &str) -> String {
    machine_id.to_string()
}

pub fn is_legacy_placeholder_anon_id(id: &str) -> bool {
    id.contains("-0000-0000-0000-000000000000")
}

pub fn normalize_anon_id(id: &str) -> Option<String> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(short) = trimmed.split('-').next() {
        if (short.len() == 8 || short.len() == 9) && short.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(short.to_string());
        }
    }
    None
}

pub fn is_same_device_id(id1: &str, id2: &str) -> bool {
    // Reflexive first: normalize_anon_id only recognises the hex-prefixed form and returns
    // None for anything else, which would otherwise make an id compare unequal to itself.
    // Callers now discard batches whose device id doesn't match, so that would silently drop
    // data if the id format ever changes.
    if !id1.is_empty() && id1 == id2 {
        return true;
    }
    let n1 = normalize_anon_id(id1);
    let n2 = normalize_anon_id(id2);
    n1.is_some() && n1 == n2
}

pub fn same_anon_id(left: &str, right: &str) -> bool {
    is_same_device_id(left, right)
}

/// Re-hide the tray icon (if the user chose to hide it) after the shell may have re-added
/// it. The shell does that asynchronously and at no fixed time, so retry a few times.
#[cfg(target_os = "windows")]
fn reapply_hidden_tray_icon(reason: &'static str) {
    if let Some(app_handle) = crate::GLOBAL_APP_HANDLE.get() {
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            for delay_ms in [500_u64, 1500, 3000] {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                if let Some(settings) = handle.try_state::<crate::app_state::SettingsState>() {
                    if settings.hide_tray_icon.load(Ordering::Relaxed) {
                        if let Some(tray) = handle.tray_by_id("main_tray") {
                            let _ = tray.set_visible(false);
                            println!(">>> [TRAY] {reason} detected, re-hiding tray icon per user setting.");
                        }
                    }
                }
            }
        });
    }
}

/// Ask for WM_WTSSESSION_CHANGE so remote-desktop reconnects and unlocks, after which the
/// shell re-shows every tray icon, can re-apply the "hide tray icon" setting.
#[cfg(target_os = "windows")]
pub unsafe fn register_tray_session_notifications(hwnd: HWND) {
    if WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) == 0 {
        eprintln!(">>> [TRAY] Failed to register Windows session notifications.");
    }
}

/// Window subclass procedure for shell/session changes that may recreate the tray icon:
/// explorer restart (TaskbarCreated), session reconnect/unlock, and display changes.
#[cfg(target_os = "windows")]
pub unsafe extern "system" fn tray_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
    if msg != 0 && msg == taskbar_msg {
        reapply_hidden_tray_icon("Explorer restart");
    } else if msg == WM_WTSSESSION_CHANGE {
        let reason = match wparam.0 {
            WTS_REMOTE_CONNECT => Some("Remote desktop reconnect"),
            WTS_CONSOLE_CONNECT => Some("Console reconnect"),
            WTS_SESSION_LOGON => Some("Session logon"),
            WTS_SESSION_UNLOCK => Some("Session unlock"),
            WTS_SESSION_REMOTE_CONTROL => Some("Remote control change"),
            _ => None,
        };
        if let Some(reason) = reason {
            reapply_hidden_tray_icon(reason);
        }
    } else if msg == WM_DISPLAYCHANGE {
        reapply_hidden_tray_icon("Display change");
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::is_same_device_id;

    #[test]
    fn device_id_matching_is_reflexive_for_any_format() {
        // Callers now drop batches whose device id does not match, so an id must always
        // equal itself even when it is not in the recognised hex-prefixed form.
        assert!(is_same_device_id("a1b2c3d4", "a1b2c3d4"));
        assert!(is_same_device_id("not-a-hex-id", "not-a-hex-id"));
        assert!(!is_same_device_id("", ""));
    }

    #[test]
    fn long_and_short_forms_of_the_same_id_match() {
        assert!(is_same_device_id(
            "a1b2c3d4-1111-2222-3333-444444444444",
            "a1b2c3d4"
        ));
        assert!(!is_same_device_id("a1b2c3d4", "b9b8b7b6"));
    }
}
