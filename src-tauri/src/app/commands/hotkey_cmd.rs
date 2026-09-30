use crate::app_state::SettingsState;
use crate::database::DbState;
use crate::error::{AppError, AppResult};
use crate::global_state::{HOTKEY_STRING, IS_HIDDEN};
use crate::infrastructure::repository::settings_repo::SettingsRepository;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

fn register_shortcut(app_handle: &AppHandle, hotkey: &str) {
    if hotkey.is_empty()
        || hotkey.eq_ignore_ascii_case("MouseMiddle")
        || hotkey.eq_ignore_ascii_case("MButton")
    {
        return;
    }

    let normalized = hotkey.replace("Win", "Super");
    if let Ok(shortcut) = normalized.parse::<Shortcut>() {
        let _ = app_handle.global_shortcut().register(shortcut);
    }
}

/// 按作用域分流注册：仅 `Global` / `BackgroundOnly` 进行全局注册，`InAppOnly` 跳过
/// （InAppOnly 仅由前端 webview keydown 响应，需求 19.2）。
fn register_shortcut_with_scope(app_handle: &AppHandle, hotkey: &str, scope: HotkeyScope) {
    if scope == HotkeyScope::InAppOnly {
        return;
    }
    register_shortcut(app_handle, hotkey);
}

pub(crate) fn sync_registered_hotkeys(app_handle: &AppHandle) -> AppResult<()> {
    let _ = app_handle.global_shortcut().unregister_all();

    let Some(settings) = app_handle.try_state::<SettingsState>() else {
        return Ok(());
    };

    let main_hotkey = settings.main_hotkey.lock().unwrap().clone();
    // The main hotkey summons the hidden panel, so it has no in-app meaning: a stored
    // InAppOnly (older builds offered it) would leave the panel unreachable by keyboard.
    let main_scope = match hotkey_scope(app_handle, "main") {
        HotkeyScope::InAppOnly => HotkeyScope::Global,
        scope => scope,
    };
    register_shortcut_with_scope(app_handle, &main_hotkey, main_scope);

    let sequential_mode = settings.sequential_mode.load(Ordering::Relaxed);
    let sequential_hotkey = settings.sequential_paste_hotkey.lock().unwrap().clone();
    if sequential_mode {
        register_shortcut_with_scope(
            app_handle,
            &sequential_hotkey,
            hotkey_scope(app_handle, "sequential"),
        );
    }

    let rich_hotkey = settings.rich_paste_hotkey.lock().unwrap().clone();
    register_shortcut_with_scope(app_handle, &rich_hotkey, hotkey_scope(app_handle, "rich"));

    let search_hotkey = settings.search_hotkey.lock().unwrap().clone();
    register_shortcut_with_scope(app_handle, &search_hotkey, hotkey_scope(app_handle, "search"));

    Ok(())
}

#[tauri::command]
pub fn register_hotkey(app_handle: AppHandle, hotkey: String) -> AppResult<()> {
    {
        let mut guard = HOTKEY_STRING.lock().unwrap();
        *guard = hotkey.clone();
    }

    if let Some(settings) = app_handle.try_state::<SettingsState>() {
        let mut guard = settings.main_hotkey.lock().unwrap();
        *guard = hotkey.clone();
    }

    sync_registered_hotkeys(&app_handle)
}

#[tauri::command]
pub fn test_hotkey_available(app_handle: AppHandle, hotkey: String) -> AppResult<bool> {
    if hotkey.is_empty()
        || hotkey.eq_ignore_ascii_case("MouseMiddle")
        || hotkey.eq_ignore_ascii_case("MButton")
    {
        return Ok(true);
    }

    let normalized = hotkey.replace("Win", "Super");
    let shortcut = normalized
        .parse::<Shortcut>()
        .map_err(|_| AppError::Validation("快捷键格式无效".to_string()))?;

    // Already registered by us (e.g. re-recording the current hotkey): available, and probing
    // it would fail as "taken by another program" and then unregister our own binding.
    if app_handle.global_shortcut().is_registered(shortcut.clone()) {
        return Ok(true);
    }

    match app_handle.global_shortcut().register(shortcut.clone()) {
        Ok(_) => {
            let _ = app_handle.global_shortcut().unregister(shortcut);
            Ok(true)
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            let user_msg = if err_str.contains("AlreadyRegistered") {
                "该快捷键已被其他程序占用".to_string()
            } else {
                "快捷键不可用".to_string()
            };
            Err(AppError::Internal(user_msg))
        }
    }
}

// ===== F5（需求 19、39）：快捷键作用域 Scope =====

/// 快捷键作用域。
///
/// - `Global`：全局注册，无论主面板是否可见均可触发。
/// - `InAppOnly`：不进行全局注册，仅在主面板可见且 webview 聚焦时由 webview keydown 响应。
/// - `BackgroundOnly`：进行全局注册，但仅在主面板不可见时响应。
///
/// 通过 serde 序列化，序列化后的字符串（"Global" / "InAppOnly" / "BackgroundOnly"）
/// 即为 settings 表中的持久化值，与 `as_str` 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HotkeyScope {
    Global,
    InAppOnly,
    BackgroundOnly,
}

impl Default for HotkeyScope {
    /// 缺省作用域为 `Global`，保证零回归。
    fn default() -> Self {
        HotkeyScope::Global
    }
}

impl HotkeyScope {
    /// 持久化用的字符串表示，与 serde 序列化结果一致。
    pub fn as_str(self) -> &'static str {
        match self {
            HotkeyScope::Global => "Global",
            HotkeyScope::InAppOnly => "InAppOnly",
            HotkeyScope::BackgroundOnly => "BackgroundOnly",
        }
    }
}

/// 兜底解析快捷键作用域。
///
/// `None`（v0.4.0 升级时缺少 Scope 字段）或无法识别的值一律视为 `Global`，
/// 从而使既有快捷键的行为与 v0.4.0 逐字节一致（零回归，需求 19.4 / 39.1）。
pub fn parse_scope(raw: Option<&str>) -> HotkeyScope {
    match raw.map(str::trim) {
        Some("Global") => HotkeyScope::Global,
        Some("InAppOnly") => HotkeyScope::InAppOnly,
        Some("BackgroundOnly") => HotkeyScope::BackgroundOnly,
        _ => HotkeyScope::Global,
    }
}

/// 生成快捷键作用域在 settings 表中的持久化 key：`app.hotkey.scope.<id>`。
///
/// `hotkey_id` ∈ {main, sequential, rich, search, quick_paste, tag, sensitive}。
pub fn scope_setting_key(hotkey_id: &str) -> String {
    format!("app.hotkey.scope.{}", hotkey_id)
}

/// 从 settings 表读取指定快捷键的作用域。
///
/// 读取 `app.hotkey.scope.<id>`，缺失或无法识别时兜底为 `Global`（需求 19.4 / 39.1）。
pub fn hotkey_scope(app_handle: &AppHandle, hotkey_id: &str) -> HotkeyScope {
    let raw = app_handle
        .try_state::<DbState>()
        .and_then(|db| db.settings_repo.get(&scope_setting_key(hotkey_id)).ok())
        .flatten();
    parse_scope(raw.as_deref())
}

/// 判断主面板当前是否可见。
///
/// 用于 `BackgroundOnly` 作用域：主面板可见时该类快捷键不响应（需求 19.7）。
/// 可见性以窗口实际可见状态为准，并排除「边缘停靠隐藏」（`IS_HIDDEN`）状态——
/// 边缘停靠隐藏时窗口虽未 hide，但对用户而言处于不可见状态，应按不可见处理。
pub fn is_main_panel_visible(app_handle: &AppHandle) -> bool {
    let Some(window) = app_handle.get_webview_window("main") else {
        return false;
    };
    let is_visible = window.is_visible().unwrap_or(false);
    let is_hidden_by_edge = IS_HIDDEN.load(Ordering::Relaxed);
    is_visible && !is_hidden_by_edge
}

/// 按当前设置重新分流注册全部快捷键（需求 19.9）。
///
/// 暴露给前端：在前端改写各作用域设置（`app.hotkey.scope.<id>`）后调用一次，
/// 触发即时重新分流生效，使改动无需重启即在 1 秒内生效。
#[tauri::command]
pub fn sync_hotkeys(app_handle: AppHandle) -> AppResult<()> {
    sync_registered_hotkeys(&app_handle)
}

// Feature: magpie-v0-4-1, Property 9: 缺 Scope 字段默认 Global（零回归）
#[cfg(test)]
mod property_9_scope_default_global {
    use super::{parse_scope, HotkeyScope};
    use proptest::prelude::*;

    /// 生成「可识别的合法 Scope 字符串」之外的任意原始值，用于覆盖「未知值」输入空间：
    /// 排除恰好等于三个合法标识符（含首尾空白可被 trim 还原）的情况，其余一律应兜底为 Global。
    fn unknown_scope_raw() -> impl Strategy<Value = String> {
        any::<String>().prop_filter("排除可识别的合法 Scope（含可被 trim 的形态）", |s| {
            !matches!(s.trim(), "Global" | "InAppOnly" | "BackgroundOnly")
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn 缺_scope_字段默认_global(unknown in unknown_scope_raw()) {
            // 缺少 Scope 字段（None，来自 v0.4.0）应解析为 Global
            prop_assert_eq!(
                parse_scope(None),
                HotkeyScope::Global,
                "缺少 Scope 字段（None）必须默认为 Global"
            );
            // 任意无法识别的 Scope 值同样应兜底为 Global，保证零回归
            prop_assert_eq!(
                parse_scope(Some(&unknown)),
                HotkeyScope::Global,
                "无法识别的 Scope 值必须兜底为 Global：raw={:?}",
                unknown
            );
        }
    }

    #[test]
    fn 合法_scope_值正常解析() {
        // 守门单测：确保兜底逻辑没有把合法值也吞成 Global
        assert_eq!(parse_scope(Some("Global")), HotkeyScope::Global);
        assert_eq!(parse_scope(Some("InAppOnly")), HotkeyScope::InAppOnly);
        assert_eq!(parse_scope(Some("BackgroundOnly")), HotkeyScope::BackgroundOnly);
        // 首尾空白应被 trim 后仍正确识别
        assert_eq!(parse_scope(Some("  InAppOnly  ")), HotkeyScope::InAppOnly);
    }
}
