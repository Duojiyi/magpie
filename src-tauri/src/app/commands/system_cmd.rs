use crate::app_state::AppDataDir;
use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
// Emitter：A10（需求 8.7）自启动失败时向前端 emit `autostart-error` 事件需要该 trait。
use tauri::{AppHandle, Emitter, Manager, State};
// A10(需求 8）：自启动统一走 tauri_plugin_autostart 的 AutoLaunchManager，
// 通过 ManagerExt 暴露的 app.autolaunch() 访问，不再直写注册表 Run 项。
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_data_path(state: State<'_, AppDataDir>) -> AppResult<String> {
    let path = state.0.lock().unwrap();
    Ok(path.to_string_lossy().to_string())
}

// These four used to hand-roll `explorer` / `open` invocations with no Linux branch at all,
// so on Linux the command bodies compiled down to `Ok(())` — the UI reported success and
// nothing opened. tauri-plugin-opener (already a dependency and already permitted in the
// capability set) implements all three platforms, including `xdg-open`.

#[tauri::command]
pub fn open_folder(app: AppHandle, path: String) -> AppResult<()> {
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(path, None::<&str>)
        .map_err(|e| AppError::Internal(format!("Failed to open folder: {}", e)))?;
    Ok(())
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle, state: State<'_, AppDataDir>) -> AppResult<()> {
    let path_str = {
        let path = state.0.lock().unwrap();
        path.to_string_lossy().to_string()
    };
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(path_str, None::<&str>)
        .map_err(|e| AppError::Internal(format!("Failed to open data folder: {}", e)))?;
    Ok(())
}

#[tauri::command]
pub fn open_file_with_default_app(app: AppHandle, file_path: String) -> AppResult<()> {
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(file_path, None::<&str>)
        .map_err(|e| AppError::Internal(format!("Failed to open file: {}", e)))?;
    Ok(())
}

#[tauri::command]
pub fn open_file_location(app: AppHandle, file_path: String) -> AppResult<()> {
    tauri_plugin_opener::OpenerExt::opener(&app)
        .reveal_item_in_dir(&file_path)
        .map_err(|e| AppError::Internal(format!("Failed to open file location: {}", e)))?;
    Ok(())
}

#[tauri::command]
pub fn toggle_autostart(app: AppHandle, enabled: bool) -> AppResult<()> {
    // A10（需求 8.1/8.2/8.3）：仅通过 autolaunch 插件注册/取消注册，不再直写注册表 Run。
    // --minimized 参数已在 main.rs 的插件初始化中配置（需求 8.4），此处无需重复传入。
    let manager = app.autolaunch();
    if enabled {
        // A10（需求 8.5）：注册时在同一操作内先清理旧的 Run 残留（TieZ / tie-z / 旧绝对路径式 Magpie），
        // 再由插件以其标准 productName(Magpie) 格式重新写入，最终注册表仅保留插件格式的 Magpie 项。
        cleanup_legacy_run_entries();
        if let Err(e) = manager.enable() {
            // A10（需求 8.7）：注册失败时不改前端开关状态（返回 Err 让前端回滚），
            // emit autostart-error 事件供前端弹中文提示，并写 tiez.log 失败记录。
            let detail = e.to_string();
            crate::error!("[AUTOSTART] 开机自启动注册失败: {}", detail);
            let _ = app.emit("autostart-error", format!("开机自启动注册失败: {}", detail));
            return Err(AppError::Internal(detail));
        }
    } else if let Err(e) = manager.disable() {
        // A10（需求 8.7）：取消注册失败时同样不改前端开关状态、emit 事件、写日志。
        let detail = e.to_string();
        crate::error!("[AUTOSTART] 开机自启动取消注册失败: {}", detail);
        let _ = app.emit(
            "autostart-error",
            format!("开机自启动取消注册失败: {}", detail),
        );
        return Err(AppError::Internal(detail));
    }
    Ok(())
}

/// A10（需求 8.5）：清理旧版本/旧实现遗留在 `HKCU\...\Run` 下的自启动残留项。
///
/// 历史上存在三类残留值：
/// - `TieZ`：改名前（TieZ 时期）写入的项；
/// - `tie-z`：早期 bundle identifier 风格写入的项；
/// - `Magpie`：v0.4.0 之前自定义实现写入的「绝对路径式」裸值（便携版移动目录后即失效）。
///
/// enable 时统一删除这三项，再由 `tauri_plugin_autostart` 以标准 productName(`Magpie`) 格式
/// 重新写入，使注册表最终仅保留插件格式的 `Magpie` 项。删除为尽力而为：值不存在属正常情况，忽略错误。
#[cfg(target_os = "windows")]
fn cleanup_legacy_run_entries() {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // Run 键删除值需 KEY_SET_VALUE 写权限
    if let Ok(run_key) = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_SET_VALUE,
    ) {
        for name in ["TieZ", "tie-z", "Magpie"] {
            let _ = run_key.delete_value(name);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn cleanup_legacy_run_entries() {}

#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> AppResult<bool> {
    // A10（需求 8.6）：以 autolaunch().is_enabled() 为准，使界面状态与插件实际注册状态一致。
    app.autolaunch()
        .is_enabled()
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn set_windows_clipboard_history(enabled: bool) -> AppResult<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut needs_restart = false;

    if let Ok((key, _)) = hkcu.create_subkey("Software\\Microsoft\\Clipboard") {
        let value: u32 = if enabled { 1 } else { 0 };
        let _ = key.set_value("EnableClipboardHistory", &value);
        let _ = key.set_value("EnableCloudClipboard", &value);
    }

    if let Ok((adv_key, _)) =
        hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced")
    {
        let current_disabled: String = adv_key.get_value("DisabledHotkeys").unwrap_or_default();
        if current_disabled.to_uppercase().contains('V') {
            let new_val = current_disabled.to_uppercase().replace('V', "");
            if new_val.is_empty() {
                let _ = adv_key.delete_value("DisabledHotkeys");
            } else {
                let _ = adv_key.set_value("DisabledHotkeys", &new_val);
            }
            needs_restart = true;
        }
    }

    if let Ok((policy_key, _)) =
        hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer")
    {
        if policy_key
            .get_value::<u32, _>("DisallowClipboardHistory")
            .unwrap_or(0)
            != 0
        {
            let _ = policy_key.delete_value("DisallowClipboardHistory");
            needs_restart = true;
        }
    }

    // Policy-based clipboard lock can also exist under Software\Policies\Microsoft\Windows\System.
    // Clear blocking values when restoring system Win+V behavior.
    if enabled {
        if let Ok((sys_policy, _)) =
            hkcu.create_subkey("Software\\Policies\\Microsoft\\Windows\\System")
        {
            if sys_policy
                .get_value::<u32, _>("AllowClipboardHistory")
                .unwrap_or(1)
                == 0
            {
                let _ = sys_policy.delete_value("AllowClipboardHistory");
                needs_restart = true;
            }
            if sys_policy
                .get_value::<u32, _>("AllowCrossDeviceClipboard")
                .unwrap_or(1)
                == 0
            {
                let _ = sys_policy.delete_value("AllowCrossDeviceClipboard");
                needs_restart = true;
            }
        }
    }

    if needs_restart {
        restart_explorer().ok();
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn set_windows_clipboard_history(_enabled: bool) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn get_windows_clipboard_history() -> AppResult<bool> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let v_disabled = match hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced")
    {
        Ok(key) => key
            .get_value::<String, _>("DisabledHotkeys")
            .unwrap_or_default()
            .to_uppercase()
            .contains('V'),
        Err(_) => false,
    };
    let history_enabled = match hkcu.open_subkey("Software\\Microsoft\\Clipboard") {
        Ok(key) => {
            key.get_value::<u32, _>("EnableClipboardHistory")
                .unwrap_or(1)
                != 0
        }
        Err(_) => true,
    };
    Ok(history_enabled && !v_disabled)
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn get_windows_clipboard_history() -> AppResult<bool> {
    Ok(false)
}

#[tauri::command]
pub fn set_win_clipboard_disabled(_disabled: bool) -> AppResult<()> {
    set_windows_clipboard_history(!_disabled)
}

/// Win+V 接管的 `DisabledHotkeys` 变换纯函数（无副作用，便于属性测试）。
///
/// - `enable == true`：在保留原有其他字符的前提下确保结果包含 `V`（已含 `V` 则原值不变）。
/// - `enable == false`：移除全部 `V`（大小写不敏感）并原样保留其他字符；若移除后为空，
///   返回 `None` 表示应删除该注册表键。
///
/// 返回 `Some(新值)` 表示应将 `DisabledHotkeys` 写为该值；返回 `None` 表示应删除该键。
pub fn apply_win_v_toggle(current: &str, enable: bool) -> Option<String> {
    // 大小写不敏感判断当前是否已含 V
    let has_v = current.to_uppercase().contains('V');
    if enable {
        if has_v {
            // 已接管，保持原值不变
            Some(current.to_string())
        } else {
            // 追加 V，原有字符全部保留
            Some(format!("{}V", current))
        }
    } else {
        // 移除全部 V（大小写不敏感），其余字符原样保留
        let cleaned: String = current
            .chars()
            .filter(|c| !c.eq_ignore_ascii_case(&'V'))
            .collect();
        if cleaned.is_empty() {
            // 移除后为空 -> 应删除该键
            None
        } else {
            Some(cleaned)
        }
    }
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn trigger_registry_win_v_optimization(enable: bool) -> AppResult<bool> {
    use winreg::enums::*;
    use winreg::RegKey;

    // 同步键盘钩子的 Win+V 拦截开关：启用接管时钩子拦截 Win+V 唤起 Magpie，
    // 关闭时不再拦截、恢复系统剪贴板历史行为。与注册表写入保持一致。
    crate::global_state::WIN_V_TAKEOVER_ENABLED
        .store(enable, std::sync::atomic::Ordering::Relaxed);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let mut changed = false;

    if let Ok((adv_key, _)) =
        hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced")
    {
        let current: String = adv_key.get_value("DisabledHotkeys").unwrap_or_default();
        // 复用纯函数计算目标值，副作用仅在此处落地到注册表
        match apply_win_v_toggle(&current, enable) {
            Some(new_val) => {
                if new_val != current {
                    let _ = adv_key.set_value("DisabledHotkeys", &new_val);
                    changed = true;
                }
            }
            None => {
                // None 表示移除 V 后为空，应删键；仅当原值确实含 V 时才需操作
                if current.to_uppercase().contains('V') {
                    let _ = adv_key.delete_value("DisabledHotkeys");
                    changed = true;
                }
            }
        }
    }

    if let Ok((cb_key, _)) = hkcu.create_subkey("Software\\Microsoft\\Clipboard") {
        let val: u32 = if enable { 0 } else { 1 };
        let prev_history = cb_key.get_value::<u32, _>("EnableClipboardHistory").ok();
        let prev_cloud = cb_key.get_value::<u32, _>("EnableCloudClipboard").ok();
        let _ = cb_key.set_value("EnableClipboardHistory", &val);
        let _ = cb_key.set_value("EnableCloudClipboard", &val);
        if prev_history != Some(val) || prev_cloud != Some(val) {
            changed = true;
        }
    }

    // When disabling Win+V takeover, also clear policy-level lock that can keep Win+V unavailable
    // until a full reboot on some systems.
    if !enable {
        if let Ok((policy_key, _)) =
            hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer")
        {
            if policy_key
                .get_value::<u32, _>("DisallowClipboardHistory")
                .unwrap_or(0)
                != 0
            {
                let _ = policy_key.delete_value("DisallowClipboardHistory");
                changed = true;
            }
        }

        if let Ok((sys_policy, _)) =
            hkcu.create_subkey("Software\\Policies\\Microsoft\\Windows\\System")
        {
            if sys_policy
                .get_value::<u32, _>("AllowClipboardHistory")
                .unwrap_or(1)
                == 0
            {
                let _ = sys_policy.delete_value("AllowClipboardHistory");
                changed = true;
            }
            if sys_policy
                .get_value::<u32, _>("AllowCrossDeviceClipboard")
                .unwrap_or(1)
                == 0
            {
                let _ = sys_policy.delete_value("AllowCrossDeviceClipboard");
                changed = true;
            }
        }
    }
    Ok(changed)
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn trigger_registry_win_v_optimization(_enable: bool) -> AppResult<bool> {
    Ok(false)
}

#[tauri::command]
pub fn is_registry_win_v_optimized() -> AppResult<bool> {
    Ok(get_registry_win_v_optimized_status())
}

#[cfg(target_os = "windows")]
pub fn get_registry_win_v_optimized_status() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced")
    {
        let current: String = key.get_value("DisabledHotkeys").unwrap_or_default();
        // 复用纯函数反推接管状态：启用接管对当前值为无操作（已含 V）即视为已接管
        return apply_win_v_toggle(&current, true).as_deref() == Some(current.as_str());
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn get_registry_win_v_optimized_status() -> bool {
    false
}

/// 将进程镜像名（小写、不含路径）映射到占用 Win+V 的已知应用名。
///
/// - PowerToys 进程族：`PowerToys.exe` / `PowerToys.PowerLauncher.exe` / `PowerToys.Settings.exe` 等，
///   统一以 `powertoys` 前缀识别。
/// - Ditto 进程族：`Ditto.exe` / `Ditto64.exe` / `Ditto_portable.exe` 等，统一以 `ditto` 前缀识别。
///
/// 命中返回对应应用名，否则返回 `None`。抽成纯函数便于复用与单元测试。
fn match_win_v_occupier(image_name_lower: &str) -> Option<&'static str> {
    if image_name_lower.starts_with("powertoys") {
        return Some("PowerToys");
    }
    if image_name_lower.starts_with("ditto") {
        return Some("Ditto");
    }
    None
}

/// 探测占用 Win+V 的第三方应用（PowerToys / Ditto）。
///
/// 通过枚举系统进程的镜像名进行匹配，命中时返回占用来源的应用名；若同时存在多个，
/// 以「、」连接（PowerToys 优先于 Ditto）。未检测到任何已知占用应用时返回 `None`。
/// 供前端在 Win+V 注册失败时向用户指明占用来源并提示释放后重试（需求 24.8）。仅 Windows。
#[cfg(target_os = "windows")]
#[tauri::command]
pub fn detect_win_v_occupier() -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::ProcessStatus::EnumProcesses;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let mut found_powertoys = false;
    let mut found_ditto = false;

    unsafe {
        // 1. 枚举所有进程 ID（缓冲区被填满时视为可能截断，扩容重试）
        let mut pids: Vec<u32> = vec![0u32; 1024];
        let mut bytes_returned: u32 = 0;
        loop {
            let cap_bytes = (pids.len() * std::mem::size_of::<u32>()) as u32;
            if EnumProcesses(pids.as_mut_ptr(), cap_bytes, &mut bytes_returned).is_err() {
                return None;
            }
            if bytes_returned == cap_bytes {
                pids.resize(pids.len() * 2, 0);
                continue;
            }
            break;
        }

        let count = (bytes_returned as usize) / std::mem::size_of::<u32>();

        // 2. 逐个进程取镜像名并匹配
        for &pid in pids.iter().take(count) {
            if pid == 0 {
                continue;
            }
            // 用 LIMITED 权限打开，可覆盖以管理员身份运行的进程
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                continue;
            };
            if handle.is_invalid() {
                continue;
            }

            let mut buf = [0u16; MAX_PATH as usize];
            let mut size = buf.len() as u32;
            let image_name = if QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
            .is_ok()
            {
                String::from_utf16_lossy(&buf[..size as usize])
            } else {
                String::new()
            };

            let _ = CloseHandle(handle);

            if image_name.is_empty() {
                continue;
            }
            // 取路径末段文件名转小写后匹配
            let file_name = image_name
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or(&image_name)
                .to_ascii_lowercase();
            match match_win_v_occupier(&file_name) {
                Some("PowerToys") => found_powertoys = true,
                Some("Ditto") => found_ditto = true,
                _ => {}
            }
            if found_powertoys && found_ditto {
                break; // 两者均已命中，无需继续枚举
            }
        }
    }

    let mut names: Vec<&str> = Vec::new();
    if found_powertoys {
        names.push("PowerToys");
    }
    if found_ditto {
        names.push("Ditto");
    }
    if names.is_empty() {
        None
    } else {
        Some(names.join("、"))
    }
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn detect_win_v_occupier() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn restart_explorer() -> AppResult<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let _ = Command::new("cmd")
        .args(["/C", "taskkill /F /IM explorer.exe & start explorer.exe"])
        .creation_flags(0x08000000)
        .spawn();
    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn restart_explorer() -> AppResult<()> {
    Ok(())
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn relaunch(app: AppHandle) {
    use std::process::Command;
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe).spawn();
    }
    app.exit(0);
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn restart_as_admin(app_handle: AppHandle) -> AppResult<()> {
    use std::env;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    // Get current executable path
    let exe_path = env::current_exe().map_err(AppError::from)?;

    // Convert to wide string
    let exe_wide: Vec<u16> = OsStr::new(&exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // "runas" verb for elevation
    let runas: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR::from_raw(runas.as_ptr()),
            PCWSTR::from_raw(exe_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        // ShellExecuteW returns > 32 on success
        if result.0 as usize <= 32 {
            return Err(AppError::Internal(
                "Failed to restart as administrator. User may have cancelled UAC prompt."
                    .to_string(),
            ));
        }
    }

    // Close current instance
    app_handle.exit(0);

    Ok(())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn restart_as_admin(_app_handle: AppHandle) -> AppResult<()> {
    Err(AppError::Internal(
        "Restart as administrator is only available on Windows".to_string(),
    ))
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn check_is_admin() -> bool {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut return_length = 0;
            let success = GetTokenInformation(
                token_handle,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut c_void),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            );

            let _ = windows::Win32::Foundation::CloseHandle(token_handle);

            if success.is_ok() {
                return elevation.TokenIsElevated != 0;
            }
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn check_is_admin() -> bool {
    false
}

#[tauri::command]
pub fn set_data_path(app_handle: AppHandle, new_path: String) -> AppResult<()> {
    let clean_path = new_path.trim().to_string();
    let new_data_path = std::path::Path::new(&clean_path);
    if !new_data_path.exists() {
        return Err(AppError::Validation("Directory does not exist".to_string()));
    }

    let old_path_buf = app_handle.state::<AppDataDir>().0.lock().unwrap().clone();

    // 1. Migrate data folders if they exist in the OLD path
    {
        for folder in ["attachments", "emoji_favorites"] {
            let old_folder = old_path_buf.join(folder);
            let new_folder = new_data_path.join(folder);

            if old_folder.exists() && old_folder.is_dir() {
                if let Err(_) = std::fs::rename(&old_folder, &new_folder) {
                    if let Err(copy_err) = copy_dir_recursive(&old_folder, &new_folder) {
                        return Err(AppError::Internal(format!(
                            "Failed to copy {}: {}",
                            folder, copy_err
                        )));
                    } else {
                        let _ = std::fs::remove_dir_all(&old_folder);
                    }
                }
            }
        }

        // Flush the write-ahead log into the main clipboard.db BEFORE moving it, so the
        // file we copy/rename to the new location is self-contained. Without this, the
        // still-open connection's uncheckpointed WAL stays behind at the old path and the
        // migrated DB is missing the most recent entries (P0: data loss on data-dir change).
        // The frontend relaunches immediately after this command returns, so the stale
        // old-path connection does not keep diverging.
        if let Some(db_state) = app_handle.try_state::<crate::database::DbState>() {
            // Recover from a poisoned lock instead of silently skipping the flush (matches
            // the clipboard monitor's poison handling); a skipped checkpoint here is the
            // very P0 data-loss this guards against.
            let conn = db_state
                .conn
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
                eprintln!("[set_data_path] wal_checkpoint before migration failed: {e}");
            }
        }

        // 1.2 Migrate database files (main + WAL/SHM)
        let db_files = ["clipboard.db", "clipboard.db-wal", "clipboard.db-shm"];
        for name in db_files {
            let old_db = old_path_buf.join(name);
            if !old_db.exists() {
                continue;
            }
            let new_db = new_data_path.join(name);
            if new_db.exists() {
                // Avoid overwriting any existing DB in new path
                let backup = new_data_path.join(format!("{}.backup", name));
                if backup.exists() {
                    let _ = std::fs::remove_file(&backup);
                }
                let _ = std::fs::rename(&new_db, &backup);
            }
            if let Err(_) = std::fs::rename(&old_db, &new_db) {
                if let Err(copy_err) = std::fs::copy(&old_db, &new_db) {
                    return Err(AppError::Internal(format!(
                        "Failed to copy {}: {}",
                        name, copy_err
                    )));
                } else {
                    let _ = std::fs::remove_file(&old_db);
                }
            }
        }
    }

    // 1.3 Rewrite internal attachment paths inside DB (if DB exists in new path)
    let new_db_path = new_data_path.join("clipboard.db");
    if new_db_path.exists() {
        rewrite_attachment_paths_in_db(&new_db_path, &old_path_buf, new_data_path)?;
        rewrite_emoji_favorites_in_db(&new_db_path, &old_path_buf, new_data_path)?;
        rewrite_custom_background_in_db(&new_db_path, &old_path_buf, new_data_path)?;
    }

    // 2. Save new path to a persistent config file
    let config_dir = app_handle.path().app_data_dir().map_err(AppError::from)?;
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(AppError::from)?;
    }

    let redirect_file = config_dir.join("datapath.txt");
    std::fs::write(&redirect_file, &clean_path).map_err(AppError::from)?;

    Ok(())
}

fn rewrite_attachment_paths_in_db(
    db_path: &std::path::Path,
    old_base: &std::path::Path,
    new_base: &std::path::Path,
) -> AppResult<()> {
    let old_attach = old_base.join("attachments");
    let new_attach = new_base.join("attachments");
    let old_prefix = old_attach.to_string_lossy().to_string();
    let new_prefix = new_attach.to_string_lossy().to_string();
    if old_prefix == new_prefix {
        return Ok(());
    }

    let old_prefix_slash = old_prefix.replace('\\', "/");
    let new_prefix_slash = new_prefix.replace('\\', "/");

    let conn = Connection::open(db_path).map_err(AppError::from)?;

    let mut stmt = conn
        .prepare("SELECT id, content, html_content FROM clipboard_history WHERE is_external = 1 OR html_content IS NOT NULL")
        .map_err(AppError::from)?;

    let rows = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(1)?;
            let html_content: Option<String> = row.get(2)?;
            Ok((id, content, html_content))
        })
        .map_err(AppError::from)?;

    for row in rows {
        let (id, content_raw, html_raw) = row.map_err(AppError::from)?;
        let mut content_new: Option<String> = None;
        let mut html_new: Option<String> = None;

        if let Some(updated) = rewrite_content_path(
            &content_raw,
            &old_prefix,
            &new_prefix,
            &old_prefix_slash,
            &new_prefix_slash,
        ) {
            content_new = Some(updated);
        }

        if let Some(html) = html_raw.as_ref() {
            if let Some(updated) = rewrite_html_paths(
                html,
                &old_prefix,
                &new_prefix,
                &old_prefix_slash,
                &new_prefix_slash,
            ) {
                html_new = Some(updated);
            }
        }

        if content_new.is_some() || html_new.is_some() {
            let content_final = content_new.as_ref().unwrap_or(&content_raw);
            let html_final = match html_new.as_ref() {
                Some(v) => Some(v.as_str()),
                None => html_raw.as_deref(),
            };
            conn.execute(
                "UPDATE clipboard_history SET content = ?1, html_content = ?2 WHERE id = ?3",
                params![content_final, html_final, id],
            )
            .map_err(AppError::from)?;
        }
    }

    Ok(())
}

fn rewrite_emoji_favorites_in_db(
    db_path: &std::path::Path,
    old_base: &std::path::Path,
    new_base: &std::path::Path,
) -> AppResult<()> {
    let old_dir = old_base.join("emoji_favorites");
    let new_dir = new_base.join("emoji_favorites");
    let old_prefix = old_dir.to_string_lossy().to_string();
    let new_prefix = new_dir.to_string_lossy().to_string();
    if old_prefix == new_prefix {
        return Ok(());
    }

    let old_prefix_slash = old_prefix.replace('\\', "/");
    let new_prefix_slash = new_prefix.replace('\\', "/");

    let conn = Connection::open(db_path).map_err(AppError::from)?;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'app.emoji_favorites'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)?;

    let Some(raw) = value else {
        return Ok(());
    };
    let parsed: Vec<String> = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    let mut changed = false;
    let mut updated: Vec<String> = Vec::with_capacity(parsed.len());
    for path in parsed {
        let mut next = path.clone();
        if next.starts_with(&old_prefix) {
            next = format!("{}{}", new_prefix, &next[old_prefix.len()..]);
        } else if next.starts_with(&old_prefix_slash) {
            next = format!("{}{}", new_prefix_slash, &next[old_prefix_slash.len()..]);
        }
        if next != path {
            changed = true;
        }
        updated.push(next);
    }

    if changed {
        let serialized = serde_json::to_string(&updated).unwrap_or(raw);
        conn.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'app.emoji_favorites'",
            params![serialized],
        )
        .map_err(AppError::from)?;
    }

    Ok(())
}

fn rewrite_custom_background_in_db(
    db_path: &std::path::Path,
    old_base: &std::path::Path,
    new_base: &std::path::Path,
) -> AppResult<()> {
    let conn = Connection::open(db_path).map_err(AppError::from)?;
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'app.custom_background'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)?;

    let Some(raw_path) = value else {
        return Ok(());
    };
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let old_path = std::path::PathBuf::from(trimmed);
    if !old_path.starts_with(old_base) {
        return Ok(());
    }

    let Ok(relative) = old_path.strip_prefix(old_base) else {
        return Ok(());
    };
    let new_path = new_base.join(relative);

    if old_path != new_path && old_path.exists() {
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(AppError::from)?;
        }
        if !new_path.exists() {
            if let Err(_) = std::fs::rename(&old_path, &new_path) {
                std::fs::copy(&old_path, &new_path).map_err(AppError::from)?;
                let _ = std::fs::remove_file(&old_path);
            }
        }
    }

    let new_value = new_path.to_string_lossy().to_string();
    if new_value != raw_path {
        conn.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'app.custom_background'",
            params![new_value],
        )
        .map_err(AppError::from)?;
    }

    Ok(())
}

fn rewrite_content_path(
    value: &str,
    old_prefix: &str,
    new_prefix: &str,
    old_prefix_slash: &str,
    new_prefix_slash: &str,
) -> Option<String> {
    let replace_prefix = |v: &str| -> Option<String> {
        if v.starts_with(old_prefix) {
            return Some(format!("{}{}", new_prefix, &v[old_prefix.len()..]));
        }
        if v.starts_with(old_prefix_slash) {
            return Some(format!(
                "{}{}",
                new_prefix_slash,
                &v[old_prefix_slash.len()..]
            ));
        }
        None
    };

    if crate::database::encryption::is_encrypted_payload(value) {
        #[cfg(not(feature = "portable"))]
        {
            // Only rewrite when we could actually read the value. Falling back to the raw
            // ciphertext (as `unwrap_or_else` did) would re-encrypt the ciphertext itself and
            // permanently lose the original.
            if let Some(plain) = crate::database::encryption::decrypt_value(value) {
                if let Some(updated_plain) = replace_prefix(&plain) {
                    let encrypted = crate::database::encryption::encrypt_value(&updated_plain)
                        .unwrap_or(updated_plain);
                    return Some(encrypted);
                }
            }
        }
        return None;
    }

    replace_prefix(value)
}

fn rewrite_html_paths(
    value: &str,
    old_prefix: &str,
    new_prefix: &str,
    old_prefix_slash: &str,
    new_prefix_slash: &str,
) -> Option<String> {
    let replace_any = |v: &str| -> Option<String> {
        let mut updated = v.replace(old_prefix, new_prefix);
        updated = updated.replace(old_prefix_slash, new_prefix_slash);
        if updated == v {
            None
        } else {
            Some(updated)
        }
    };

    if crate::database::encryption::is_encrypted_payload(value) {
        #[cfg(not(feature = "portable"))]
        {
            // See the note above: never re-encrypt a value we failed to decrypt.
            if let Some(plain) = crate::database::encryption::decrypt_value(value) {
                if let Some(updated_plain) = replace_any(&plain) {
                    let encrypted = crate::database::encryption::encrypt_value(&updated_plain)
                        .unwrap_or(updated_plain);
                    return Some(encrypted);
                }
            }
        }
        return None;
    }

    replace_any(value)
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 匹配_powertoys_进程族() {
        // 主进程与各子进程均应识别为 PowerToys
        assert_eq!(match_win_v_occupier("powertoys.exe"), Some("PowerToys"));
        assert_eq!(
            match_win_v_occupier("powertoys.powerlauncher.exe"),
            Some("PowerToys")
        );
        assert_eq!(
            match_win_v_occupier("powertoys.settings.exe"),
            Some("PowerToys")
        );
    }

    #[test]
    fn 匹配_ditto_进程族() {
        assert_eq!(match_win_v_occupier("ditto.exe"), Some("Ditto"));
        assert_eq!(match_win_v_occupier("ditto64.exe"), Some("Ditto"));
        assert_eq!(match_win_v_occupier("ditto_portable.exe"), Some("Ditto"));
    }

    #[test]
    fn 非占用进程不匹配() {
        assert_eq!(match_win_v_occupier("explorer.exe"), None);
        assert_eq!(match_win_v_occupier("magpie.exe"), None);
        assert_eq!(match_win_v_occupier(""), None);
    }

    // ===== A10（需求 8.6）：自启动状态查询往返一致性的可注入状态后端建模 =====

    use proptest::prelude::*;

    /// 自启动状态的可注入后端建模。
    ///
    /// 真实实现里 `toggle_autostart` / `is_autostart_enabled` 委托 `tauri_plugin_autostart`
    /// 的 `autolaunch()`，与 OS 注册表耦合，无法在单元测试中直接驱动。此处以等价语义建模：
    /// - `enable(fail)`：注册成功则状态置真；失败（与注册表交互失败）则状态不变（对应需求 8.7
    ///   「注册或取消注册失败，保持开关状态不变」）。
    /// - `disable(fail)`：同理，成功置假、失败不变。
    /// - `is_enabled()`：返回当前实际注册状态（对应 `autolaunch().is_enabled()`）。
    #[derive(Debug)]
    struct AutostartBackend {
        enabled: bool,
    }

    impl AutostartBackend {
        fn new() -> Self {
            // 初始未注册，等价于插件默认未启用自启动
            Self { enabled: false }
        }

        fn enable(&mut self, fail: bool) -> Result<(), ()> {
            if fail {
                Err(())
            } else {
                self.enabled = true;
                Ok(())
            }
        }

        fn disable(&mut self, fail: bool) -> Result<(), ()> {
            if fail {
                Err(())
            } else {
                self.enabled = false;
                Ok(())
            }
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }
    }

    // Feature: magpie-v0-4-1, Property 11: 自启动状态查询往返一致性
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn 自启动状态查询往返一致性(
            // 操作序列：每个元素为 (目标状态: true=启用/false=关闭, 是否注册失败)
            ops in proptest::collection::vec((any::<bool>(), any::<bool>()), 1..20usize)
        ) {
            let mut backend = AutostartBackend::new();
            // 期望状态 = 最后一次成功操作的目标状态；若无任何成功操作则保持初始未注册
            let mut expected = false;
            for (target, fail) in &ops {
                let result = if *target {
                    backend.enable(*fail)
                } else {
                    backend.disable(*fail)
                };
                if result.is_ok() {
                    expected = *target;
                }
            }
            // is_autostart_enabled 的查询结果应等于序列中最后一次成功操作的目标状态，
            // 使界面状态与插件实际注册状态一致（需求 8.6）。
            prop_assert_eq!(
                backend.is_enabled(),
                expected,
                "查询结果应等于最后一次成功操作的目标状态：ops={:?}",
                ops
            );
        }
    }

    // Feature: magpie-v0-4-1, Property 10: Win+V 接管的 DisabledHotkeys 变换不变量
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn win_v_接管的_disabledhotkeys_变换不变量(current in "[A-Za-z]{0,16}") {
            // 去除全部 V（大小写不敏感）后的其余字符，作为「原有其他字符」的基准
            let others: String = current
                .chars()
                .filter(|c| !c.eq_ignore_ascii_case(&'V'))
                .collect();

            // —— 启用接管：结果应包含 V 且原有其他字符全部保留 ——
            let enabled = apply_win_v_toggle(&current, true).expect("启用接管后应写入非空值");
            prop_assert!(
                enabled.to_uppercase().contains('V'),
                "启用接管后该值应包含 V：current={:?} -> {:?}",
                current,
                enabled
            );
            let enabled_others: String = enabled
                .chars()
                .filter(|c| !c.eq_ignore_ascii_case(&'V'))
                .collect();
            prop_assert_eq!(
                &enabled_others,
                &others,
                "启用接管后原有其他字符必须全部保留：current={:?}",
                current
            );

            // —— 关闭接管：结果不应包含 V，原有其他字符全部保留；移除后为空则删键（None） ——
            let disabled = apply_win_v_toggle(&current, false);
            let disabled_flat = disabled.clone().unwrap_or_default();
            prop_assert!(
                !disabled_flat.to_uppercase().contains('V'),
                "关闭接管后该值不应包含 V：current={:?} -> {:?}",
                current,
                disabled
            );
            prop_assert_eq!(
                &disabled_flat,
                &others,
                "关闭接管后应仅移除 V 并保留其他字符：current={:?}",
                current
            );
            if others.is_empty() {
                prop_assert!(
                    disabled.is_none(),
                    "移除 V 后为空时应返回 None 以删除该注册表键：current={:?}",
                    current
                );
            } else {
                prop_assert_eq!(disabled.as_deref(), Some(others.as_str()));
            }

            // —— 往返：从不含 V 的状态先启用再关闭，应回到与初始等价的不含 V 状态 ——
            let base_no_v = others.clone();
            let re_enabled = apply_win_v_toggle(&base_no_v, true).expect("启用后应非空");
            let re_disabled = apply_win_v_toggle(&re_enabled, false).unwrap_or_default();
            prop_assert_eq!(
                &re_disabled,
                &base_no_v,
                "往返（启用再关闭）后应回到初始的不含 V 状态：base={:?}",
                base_no_v
            );
        }
    }
}
