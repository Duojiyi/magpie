use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, MOD_WIN, VK_CONTROL, VK_LWIN,
    VK_MENU, VK_RWIN, VK_SHIFT,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::Ime::{
    ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext, GCS_COMPSTR,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetAncestor, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    PostMessageW, WindowFromPoint, GA_ROOT, GUITHREADINFO, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEWHEEL, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::app::window_manager::{hide_window_cmd, toggle_window};
use crate::app_state::SettingsState;
use crate::global_state::*;
#[cfg(target_os = "windows")]
use crate::infrastructure::windows_ext::WindowExt;

// Store registered hotkey IDs for cleanup
static BLOCKED_HOTKEY_IDS: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());

#[cfg(target_os = "windows")]
fn quick_paste_index_from_vk(vk: u32) -> Option<usize> {
    match vk {
        0x31 | 0x61 => Some(0),
        0x32 | 0x62 => Some(1),
        0x33 | 0x63 => Some(2),
        0x34 | 0x64 => Some(3),
        0x35 | 0x65 => Some(4),
        0x36 | 0x66 => Some(5),
        0x37 | 0x67 => Some(6),
        0x38 | 0x68 => Some(7),
        0x39 | 0x69 => Some(8),
        0x30 | 0x60 => Some(9),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn quick_paste_modifier_from_settings() -> String {
    if let Some(handle) = GLOBAL_APP_HANDLE.get() {
        let settings = handle.state::<SettingsState>();
        // 可恢复：键盘钩子每次按键都会读取该设置，锁中毒时取回内部值即可，
        // 不能让钩子回调 panic（panic 会破坏全局键盘钩子，使快捷键彻底失效）。
        return settings
            .quick_paste_modifier
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
            .to_ascii_lowercase();
    }

    "disabled".to_string()
}

#[cfg(target_os = "windows")]
fn quick_paste_modifier_active(
    modifier: &str,
    ctrl_down: bool,
    alt_down: bool,
    shift_down: bool,
    win_down: bool,
) -> bool {
    match modifier {
        "disabled" => false,
        "ctrl" => ctrl_down && !alt_down && !shift_down && !win_down,
        "alt" => alt_down && !ctrl_down && !shift_down && !win_down,
        "shift" => shift_down && !ctrl_down && !alt_down && !win_down,
        "win" => win_down && !ctrl_down && !alt_down && !shift_down,
        _ => ctrl_down && !alt_down && !shift_down && !win_down,
    }
}

#[tauri::command]
pub fn set_recording_mode(app_handle: AppHandle, enabled: bool) -> Result<(), String> {
    IS_RECORDING.store(enabled, Ordering::SeqCst);

    // 可恢复：已注册热键 ID 列表锁中毒时取回内部值继续操作，避免命令直接 panic。
    let mut ids = BLOCKED_HOTKEY_IDS
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    #[cfg(target_os = "windows")]
    if enabled {
        // Register ALL Win+ combinations to block system from handling them
        if let Some(window) = app_handle.get_webview_window("main") {
            if let Ok(hwnd_raw) = window.hwnd() {
                let hwnd = HWND(hwnd_raw.0);
                let mut id_counter = 0x1000i32;

                // Block Win + A-Z
                for vk in 0x41u32..=0x5Au32 {
                    unsafe {
                        if RegisterHotKey(Some(hwnd), id_counter, MOD_WIN | MOD_NOREPEAT, vk)
                            .is_ok()
                        {
                            ids.push(id_counter);
                        }
                    }
                    id_counter += 1;
                }

                // Block Win + 0-9
                for vk in 0x30u32..=0x39u32 {
                    unsafe {
                        if RegisterHotKey(Some(hwnd), id_counter, MOD_WIN | MOD_NOREPEAT, vk)
                            .is_ok()
                        {
                            ids.push(id_counter);
                        }
                    }
                    id_counter += 1;
                }

                // Block special keys
                let special_keys = [0x20u32, 0x0D, 0x09, 0x1B, 0x2C]; // Space, Enter, Tab, Esc, PrintScreen
                for vk in special_keys {
                    unsafe {
                        if RegisterHotKey(Some(hwnd), id_counter, MOD_WIN | MOD_NOREPEAT, vk)
                            .is_ok()
                        {
                            ids.push(id_counter);
                        }
                    }
                    id_counter += 1;
                }
                println!("Recording mode ON: Blocked {} Win+ combinations", ids.len());
            }
        }
    } else {
        // Unregister all blocked hotkeys
        if let Some(window) = app_handle.get_webview_window("main") {
            if let Ok(hwnd_raw) = window.hwnd() {
                let hwnd = HWND(hwnd_raw.0);
                for id in ids.drain(..) {
                    unsafe {
                        let _ = UnregisterHotKey(Some(hwnd), id);
                    }
                }
                println!("Recording mode OFF: Released blocked hotkeys");
            }
        }
    }

    Ok(())
}

/// 探测「前台窗口此刻是否正处于 IME 合成态」（U5 / 需求 14.2）。
///
/// 用途：在低级键盘钩子吞掉全局导航键（↑/↓/Enter/Esc）之前做守卫——若用户正在
/// **其他应用**里用输入法合成中文，则应放行这些键给输入法（上屏/选候选/取消），
/// 避免 Magpie 抢键导致吞字。
///
/// 安全约束（钩子热路径）：
/// - 只读、不阻塞、绝不 panic（全程 `is_null()`/`is_err()`/`let _ =` 兜底）；
/// - 探测不到（纯 TSF 输入法返回 0、API 失败、句柄为空）一律返回 `false`，
///   使调用方回退到原有导航行为，保证零回归。
///
/// 局限：仅对经 IMM32 兼容层暴露合成串的输入法有效；Windows 25H2 默认微软输入法的
/// 纯 TSF 路径可能返回 0，此时回退原行为（不恶化）。详见 docs/ime-25h2-investigation.md。
#[cfg(target_os = "windows")]
unsafe fn foreground_ime_composing() -> bool {
    let fg = GetForegroundWindow();
    if fg.0.is_null() {
        return false;
    }

    // 前台窗口可能是容器，真正持有键盘焦点的是其线程内的某个子控件。
    // 经 GetGUIThreadInfo 取该线程的焦点窗口（hwndFocus），合成上下文挂在它上面。
    let mut gui = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    let thread_id = GetWindowThreadProcessId(fg, None);
    let target = if thread_id != 0 && GetGUIThreadInfo(thread_id, &mut gui).is_ok() {
        if gui.hwndFocus.0.is_null() {
            fg
        } else {
            gui.hwndFocus
        }
    } else {
        fg
    };

    let himc = ImmGetContext(target);
    if himc.0.is_null() {
        return false;
    }

    // 先以 0 长度查询合成串字节数；>0 即表示当前存在未上屏的合成内容。
    let bytes = ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0);
    let _ = ImmReleaseContext(target, himc);

    bytes > 0
}

// Low-level Keyboard Hook Procedure
#[cfg(target_os = "windows")]
pub unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let msg = w_param.0 as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    if n_code >= 0 && (is_down || is_up) {
        let kbd_struct = *(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk = kbd_struct.vkCode;

        // Handle Recording Mode - Black Hole Logic
        if IS_RECORDING.load(Ordering::SeqCst) {
            let ctrl_down = GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0;
            let shift_down = GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0;
            let alt_down = GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0;
            let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000 != 0)
                || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000 != 0);

            // ESC to cancel
            if vk == 0x1B && is_down {
                IS_RECORDING.store(false, Ordering::SeqCst);
                if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                    let _ = handle.emit("recording-cancelled", ());
                }
                return CallNextHookEx(None, n_code, w_param, l_param);
            }

            let is_win = vk == 0x5B || vk == 0x5C;
            let is_other_modifier = (vk >= 0x10 && vk <= 0x12) || (vk >= 0xA0 && vk <= 0xA5);

            if is_other_modifier {
                return CallNextHookEx(None, n_code, w_param, l_param);
            }

            if !is_win && is_down {
                if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                    let key_name = match vk {
                        0x20 => "Space".to_string(),
                        0x0D => "Enter".to_string(),
                        0x09 => "Tab".to_string(),
                        0x08 => "Backspace".to_string(),
                        0x2E => "Delete".to_string(),
                        0x2D => "Insert".to_string(),
                        0x21 => "PageUp".to_string(),
                        0x22 => "PageDown".to_string(),
                        0x23 => "End".to_string(),
                        0x24 => "Home".to_string(),
                        0x25 => "Left".to_string(),
                        0x26 => "Up".to_string(),
                        0x27 => "Right".to_string(),
                        0x28 => "Down".to_string(),
                        0xBB => "Plus".to_string(),
                        0xBC => "Comma".to_string(),
                        0xBD => "Minus".to_string(),
                        0xBE => "Period".to_string(),
                        0xBF => "/".to_string(),
                        0xC0 => "`".to_string(),
                        0xBA => ";".to_string(),
                        0xDB => "[".to_string(),
                        0xDC => "\\".to_string(),
                        0xDD => "]".to_string(),
                        0xDE => "'".to_string(),
                        k if k >= 0x70 && k <= 0x87 => format!("F{}", k - 0x6F),
                        k if (k >= 0x30 && k <= 0x39) || (k >= 0x41 && k <= 0x5A) => {
                            // 可恢复：该范围均为合法 ASCII 码点，from_u32 理论恒成功；
                            // 仍以兜底回退避免键盘钩子回调 panic 破坏全局钩子。
                            char::from_u32(k)
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| format!("Key_{}", vk))
                        }
                        _ => format!("Key_{}", vk),
                    };

                    let final_hotkey = format!(
                        "{}{}{}{}{}",
                        if ctrl_down { "Ctrl+" } else { "" },
                        if shift_down { "Shift+" } else { "" },
                        if alt_down { "Alt+" } else { "" },
                        if win_down { "Win+" } else { "" },
                        key_name
                    );

                    println!("Recorded Hotkey: {}", final_hotkey);
                    let _ = handle.emit("hotkey-recorded", final_hotkey);
                    IS_RECORDING.store(false, Ordering::SeqCst);
                }
            }
            return LRESULT(1);
        }

        // 2.5 Win+V 接管唤起（默认开启）：
        // Win+V 是系统剪贴板历史快捷键，global_shortcut 插件无法注册带 Win 的组合，
        // 因此由低级键盘钩子直接拦截。启用接管时（WIN_V_TAKEOVER_ENABLED），按下 Win+V
        // 唤起/切换 Magpie 主窗口，并吞掉该按键阻止透传到系统剪贴板历史；松开 V 同样吞掉，
        // 避免 V 字符或系统浮层残留。与现有主快捷键（默认 Alt+C）并存，互不影响。
        if WIN_V_TAKEOVER_ENABLED.load(Ordering::Relaxed) && vk == 0x56 {
            let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000 != 0)
                || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000 != 0);
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;

            // 仅响应「纯 Win+V」（不含 Ctrl/Alt/Shift），避免与其他组合键冲突。
            if win_down && !ctrl_down && !alt_down && !shift_down {
                if is_down {
                    // 去抖：长按 Win+V 会持续产生 WM_KEYDOWN 自动重复，若每次都派发 toggle_window
                    // 会导致窗口快速显隐闪烁。距上次触发 < 350ms 则只吞键、不再切换。
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let last = LAST_TOGGLE_TIMESTAMP.load(Ordering::Relaxed);
                    if now_ms.saturating_sub(last) >= 350 {
                        LAST_TOGGLE_TIMESTAMP.store(now_ms, Ordering::Relaxed);
                        if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                            let handle_clone = handle.clone();
                            // 切换窗口涉及 UI 操作，派发到异步运行时执行，避免在钩子回调内阻塞。
                            tauri::async_runtime::spawn(async move {
                                crate::app::window_manager::toggle_window(&handle_clone);
                            });
                        }
                    }
                }
                // 按下与松开都吞掉，阻止 Win+V 透传给系统剪贴板历史浮层。
                return LRESULT(1);
            }
        }

        // 3. Global Paste Sound Trigger (Ctrl+V)
        {
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
            let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000 != 0)
                || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000 != 0);

            if vk == 0x56 && ctrl_down && !alt_down && !shift_down && !win_down {
                if is_down {
                    if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                        let settings = handle.state::<SettingsState>();
                        if settings.sound_enabled.load(Ordering::Relaxed) {
                            std::thread::spawn(move || {
                                let _ = handle.emit("play-sound", "paste");
                            });
                        }
                    }
                }
            }
        }

        // 4. Quick Paste by Modifier+Number
        //
        // 需求 16（F2）将数字快捷粘贴的 Scope 固定为 InAppOnly：主面板可见时由前端 webview
        // keydown 响应 Ctrl+1~9，主面板隐藏时不得拦截、必须透传至前台应用（需求 16.7）。
        // 因此当主面板隐藏（IS_HIDDEN）时，本全局钩子分支直接跳过，不消费数字键，避免后台拦截。
        if !IS_HIDDEN.load(Ordering::Relaxed) {
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
            let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000 != 0)
                || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000 != 0);
            let quick_paste_modifier = quick_paste_modifier_from_settings();

            if is_up
                && matches!(
                    vk,
                    0x11 | 0xA2 | 0xA3 | 0x12 | 0xA4 | 0xA5 | 0x10 | 0xA0 | 0xA1 | 0x5B | 0x5C
                )
            {
                QUICK_PASTE_DIGIT_MASK.store(0, Ordering::SeqCst);
            }

            if let Some(index) = quick_paste_index_from_vk(vk) {
                let bit = 1u32 << index;

                if is_up {
                    let pressed_mask = QUICK_PASTE_DIGIT_MASK.fetch_and(!bit, Ordering::SeqCst);
                    if pressed_mask & bit != 0 {
                        return LRESULT(1);
                    }
                }

                if is_down
                    && quick_paste_modifier_active(
                        &quick_paste_modifier,
                        ctrl_down,
                        alt_down,
                        shift_down,
                        win_down,
                    )
                {
                    let pressed_mask = QUICK_PASTE_DIGIT_MASK.fetch_or(bit, Ordering::SeqCst);
                    if pressed_mask & bit != 0 {
                        return LRESULT(1);
                    }

                    if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                        let handle_clone = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(err) =
                                crate::services::clipboard_ops::paste_history_item_by_index(
                                    handle_clone,
                                    index,
                                )
                                .await
                            {
                                eprintln!(
                                    "[ERROR] Quick paste by {}+{} failed: {}",
                                    quick_paste_modifier,
                                    index + 1,
                                    err
                                );
                            }
                        });
                    }

                    return LRESULT(1);
                }
            }
        }

        // 5. Global Navigation Keys (Up/Down, Enter, Esc)
        if NAVIGATION_ENABLED.load(Ordering::SeqCst) && !IS_RECORDING.load(Ordering::SeqCst) {
            if IS_HIDDEN.load(Ordering::Relaxed) {
                return CallNextHookEx(None, n_code, w_param, l_param);
            }
            let allow_navigation = if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                let settings = handle.state::<SettingsState>();
                settings.arrow_key_selection.load(Ordering::Relaxed)
            } else {
                true
            };

            if !allow_navigation {
                return CallNextHookEx(None, n_code, w_param, l_param);
            }

            let is_navigation_key = vk == 0x26 || vk == 0x28 || vk == 0x0D || vk == 0x1B;
            let is_enter = vk == 0x0D;
            let _is_escape = vk == 0x1B;

            if is_navigation_key && is_down {
                // IME 合成守卫（U5 / 需求 14.2）：若前台应用正在输入法合成中，
                // 放行 ↑/↓/Enter/Esc 给输入法（选候选/上屏/取消），避免抢键吞字。
                // 探测不到合成（纯 TSF 或 API 失败）时回退原行为，保证零回归。
                if foreground_ime_composing() {
                    return CallNextHookEx(None, n_code, w_param, l_param);
                }

                // Only Enter requires navigation mode to be active
                // Escape can always close the window when it's visible
                if is_enter && !NAVIGATION_MODE_ACTIVE.load(Ordering::Relaxed) {
                    return CallNextHookEx(None, n_code, w_param, l_param);
                }
                let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
                let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
                let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000 != 0)
                    || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000 != 0);

                if !ctrl_down && !alt_down && !win_down {
                    if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                        let action = match vk {
                            0x26 => "up",
                            0x28 => "down",
                            0x0D => "enter",
                            0x1B => "escape",
                            _ => "",
                        };

                        if !action.is_empty() {
                            if vk == 0x26 || vk == 0x28 {
                                NAVIGATION_MODE_ACTIVE.store(true, Ordering::Relaxed);
                            } else if vk == 0x1B {
                                NAVIGATION_MODE_ACTIVE.store(false, Ordering::Relaxed);
                            }
                            if action == "escape" {
                                let handle_clone = handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    let _ = handle_clone.emit("navigation-action", "escape");
                                    toggle_window(&handle_clone);
                                });
                            } else {
                                let _ = handle.emit("navigation-action", action);
                            }
                            return LRESULT(1);
                        }
                    }
                }
            }
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}

// Low-level Mouse Hook Procedure
#[cfg(target_os = "windows")]
pub unsafe extern "system" fn mouse_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param.0 as u32;

        // 处理鼠标滚轮穿透（U3 / 需求 12.1、12.2）
        // 固定窗口（pinned）模式下窗口设了 set_focusable(false) 不获取焦点，系统会把
        // WM_MOUSEWHEEL 路由到下层有焦点的窗口，导致滚轮“穿透”到背后的其他应用。
        // 当光标悬停在 Magpie 窗口矩形内时，将滚轮消息直接投递给光标正下方的窗口
        // （即 Magpie 的 webview 子窗口），并吞掉原始事件，确保滚动作用于 Magpie 自身、不穿透。
        if msg == WM_MOUSEWHEEL && WINDOW_PINNED.load(Ordering::Relaxed) {
            if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                if let Some(window) = handle.get_webview_window("main") {
                    if let Ok(hwnd_raw) = window.hwnd() {
                        let main_hwnd = HWND(hwnd_raw.0);
                        // 窗口不可见时不处理，放行
                        if WindowExt::is_window_visible(main_hwnd) {
                            let mouse_struct = *(l_param.0 as *const MSLLHOOKSTRUCT);
                            let point = mouse_struct.pt;

                            // 复用与“点击别处隐藏”一致的窗口矩形判断逻辑
                            let mut rect = RECT::default();
                            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                                main_hwnd, &mut rect,
                            );

                            let is_inside = point.x >= rect.left
                                && point.x <= rect.right
                                && point.y >= rect.top
                                && point.y <= rect.bottom;

                            if is_inside {
                                // 取光标正下方的窗口（通常是 Magpie 的 webview 子窗口）
                                let target = WindowFromPoint(point);
                                // 仅当该窗口的根祖先为 Magpie 主窗口时才转发，避免误投递到其他应用
                                if !target.0.is_null() && GetAncestor(target, GA_ROOT) == main_hwnd {
                                    // 重建 WM_MOUSEWHEEL 参数：
                                    // wParam 高字为滚轮增量、低字为按键修饰标志（此处置 0）；
                                    // lParam 低字为屏幕坐标 x、高字为屏幕坐标 y。
                                    let wheel_delta = (mouse_struct.mouseData >> 16) as i16;
                                    let wparam =
                                        WPARAM(((wheel_delta as u16 as u32) << 16) as usize);
                                    let lparam = LPARAM(
                                        ((((point.y as u32) & 0xFFFF) << 16)
                                            | ((point.x as u32) & 0xFFFF))
                                            as i32 as isize,
                                    );
                                    let _ =
                                        PostMessageW(Some(target), WM_MOUSEWHEEL, wparam, lparam);
                                    // 吞掉原始滚轮事件，阻止穿透到下层应用
                                    return LRESULT(1);
                                }
                            }
                        }
                    }
                }
            }
        }

        if msg == WM_MBUTTONDOWN
            || msg == WM_LBUTTONDOWN
            || msg == WM_RBUTTONDOWN
            || msg == windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP
            || msg == windows::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP
        {
            // Track mouse state globally
            if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN {
                IS_MOUSE_BUTTON_DOWN.store(true, Ordering::SeqCst);
            } else if msg == windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP
                || msg == windows::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP
            {
                IS_MOUSE_BUTTON_DOWN.store(false, Ordering::SeqCst);
                return CallNextHookEx(None, n_code, w_param, l_param); // Return early for up events
            }

            // Handle Recording Mode
            if IS_RECORDING.load(Ordering::SeqCst) && msg == WM_MBUTTONDOWN {
                IS_RECORDING.store(false, Ordering::SeqCst);
                if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                    let _ = handle.emit("hotkey-recorded", "MouseMiddle");
                }
                return LRESULT(1);
            }

            // Click Elsewhere to Hide Logic
            if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN {
                if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                    if let Some(window) = handle.get_webview_window("main") {
                        if !IGNORE_BLUR.load(Ordering::Relaxed) {
                            let mouse_struct = *(l_param.0 as *const MSLLHOOKSTRUCT);
                            let point = mouse_struct.pt;

                            if let Ok(hwnd_raw) = window.hwnd() {
                                let main_hwnd = HWND(hwnd_raw.0);
                                if !WindowExt::is_window_visible(main_hwnd) {
                                    return CallNextHookEx(None, n_code, w_param, l_param);
                                }
                                let mut rect = RECT::default();
                                let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(
                                    main_hwnd, &mut rect,
                                );

                                // Boundary check: Is point outside the rect? (with 5px margin of safety)
                                let margin = 5;
                                let is_outside = point.x < rect.left - margin
                                    || point.x > rect.right + margin
                                    || point.y < rect.top - margin
                                    || point.y > rect.bottom + margin;

                                if is_outside {
                                    // Status check before hiding
                                    if !WindowExt::is_window_visible(main_hwnd) {
                                        return CallNextHookEx(None, n_code, w_param, l_param);
                                    }

                                    if WINDOW_PINNED.load(Ordering::Relaxed) {
                                        // Pinned: Just reset focusable state to ensure we don't retain focus
                                        let _ = window.set_focusable(false);
                                    } else {
                                        let _ = hide_window_cmd(handle.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Handle configured middle mouse hotkey
            if msg == WM_MBUTTONDOWN {
                // 可恢复：鼠标钩子每次中键都会读取该热键字符串，锁中毒时取回内部值即可，
                // 不能让鼠标钩子回调 panic（会破坏全局鼠标钩子）。
                let current = HOTKEY_STRING
                    .lock()
                    .map(|g| g.to_lowercase())
                    .unwrap_or_else(|e| e.into_inner().to_lowercase());
                if current == "mousemiddle" || current == "mbutton" {
                    if let Some(handle) = GLOBAL_APP_HANDLE.get() {
                        toggle_window(&handle);
                    }
                    return LRESULT(1);
                }
            }
        }
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

pub fn parse_hotkey_for_hook(hotkey: &str) -> Option<HookHotkey> {
    let parts: Vec<&str> = hotkey.split('+').collect();
    let mut vk = 0u32;
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut win = false;

    for part in parts {
        let part_upper = part.trim().to_uppercase();
        match part_upper.as_str() {
            "CTRL" | "CONTROL" => ctrl = true,
            "SHIFT" => shift = true,
            "ALT" | "MENU" => alt = true,
            "SUPER" | "WIN" | "COMMAND" | "META" => win = true,
            "SPACE" => vk = 0x20,
            "ENTER" | "RETURN" => vk = 0x0D,
            "TAB" => vk = 0x09,
            "BACKSPACE" => vk = 0x08,
            "DELETE" => vk = 0x2E,
            "INSERT" => vk = 0x2D,
            "PAGEUP" => vk = 0x21,
            "PAGEDOWN" => vk = 0x22,
            "END" => vk = 0x23,
            "HOME" => vk = 0x24,
            "LEFT" => vk = 0x25,
            "UP" => vk = 0x26,
            "RIGHT" => vk = 0x27,
            "DOWN" => vk = 0x28,
            "PLUS" | "=" => vk = 0xBB,
            "COMMA" | "," => vk = 0xBC,
            "MINUS" | "-" => vk = 0xBD,
            "PERIOD" | "." => vk = 0xBE,
            "/" | "SLASH" => vk = 0xBF,
            "`" | "TILDE" | "GRAVE" => vk = 0xC0,
            ";" | "SEMICOLON" => vk = 0xBA,
            "[" | "LBRACKET" => vk = 0xDB,
            "\\" | "BACKSLASH" => vk = 0xDC,
            "]" | "RBRACKET" => vk = 0xDD,
            "'" | "QUOTE" => vk = 0xDE,
            key if key.starts_with('F') && key.len() > 1 => {
                if let Ok(num) = key[1..].parse::<u32>() {
                    if (1..=24).contains(&num) {
                        vk = 0x6F + num;
                    }
                }
            }
            key => {
                if key.len() == 1 {
                    // 可恢复：key.len()==1 时一定有首字符，仍以 if let 兜底避免解析快捷键时 panic。
                    if let Some(c) = key.chars().next() {
                        vk = c as u32;
                    }
                }
            }
        }
    }

    if vk != 0 {
        Some(HookHotkey {
            vk,
            ctrl,
            shift,
            alt,
            win,
        })
    } else {
        None
    }
}

pub fn is_win_v_hotkey(hotkey: &str) -> bool {
    let parts: Vec<String> = hotkey
        .split('+')
        .map(|p| p.trim().to_uppercase())
        .filter(|p| !p.is_empty())
        .collect();

    if parts.is_empty() {
        return false;
    }

    let mut has_win = false;
    let mut has_v = false;

    for part in &parts {
        match part.as_str() {
            "WIN" | "SUPER" | "COMMAND" | "META" => has_win = true,
            "V" => has_v = true,
            _ => return false,
        }
    }

    has_win && has_v
}
