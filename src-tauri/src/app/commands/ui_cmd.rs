use crate::app_state::SettingsState;
use crate::database::DbState;
use crate::error::{AppError, AppResult};
use crate::infrastructure::repository::settings_repo::SettingsRepository;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State, Theme, WebviewWindow};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Serialize)]
pub struct PlatformInfo {
    pub platform: String,
    pub is_windows_10: bool,
    pub is_windows_11: bool,
}

#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    #[cfg(target_os = "windows")]
    {
        let build = windows_version::OsVersion::current().build;
        let is_windows_11 = build >= 22000;
        let is_windows_10 = build >= 10240 && build < 22000;
        PlatformInfo {
            platform: "windows".to_string(),
            is_windows_10,
            is_windows_11,
        }
    }

    #[cfg(target_os = "macos")]
    {
        PlatformInfo {
            platform: "macos".to_string(),
            is_windows_10: false,
            is_windows_11: false,
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        PlatformInfo {
            platform: "other".to_string(),
            is_windows_10: false,
            is_windows_11: false,
        }
    }
}

#[tauri::command]
pub fn send_system_notification(app: AppHandle, title: String, body: String) -> AppResult<()> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|err| AppError::Internal(format!("发送系统通知失败: {}", err)))?;

    Ok(())
}

/// 旧主题别名归一（与前端 `LEGACY_THEME_MAP` 语义一致，权威见设计文档表 4.1）。
///
/// 映射规则：`mica`/`sakura`→`mist`、`acrylic`→`dusk`、`retro`→`ink`、
/// `sticky-note`→`paper`；新主题（`ink`/`paper`/`mist`/`dusk`）原样返回；
/// `store-*` 前缀、空串及任意未知值一律落到默认主题 `ink`。
pub(crate) fn normalize_theme_id(theme: &str) -> &str {
    match theme {
        "mica" | "sakura" => "mist",
        "acrylic" => "dusk",
        "retro" => "ink",
        "sticky-note" => "paper",
        "ink" | "paper" | "mist" | "dusk" => theme,
        t if t.starts_with("store-") => "ink",
        _ => "ink", // 未知（含空串）一律落默认
    }
}

/// 玻璃主题判定：仅归一后为 `mist` / `dusk` 时触发 DWM vibrancy。
/// 取代原 `theme == "mica" || theme == "acrylic"` 的散落硬编码。
pub(crate) fn is_glass_theme(theme: &str) -> bool {
    matches!(normalize_theme_id(theme), "mist" | "dusk")
}

/// 当前 Windows 是否支持 DWM mica（Win11，build ≥ 22000）。
///
/// 单一权威阈值定义点——`set_theme` / `window_manager::toggle_window` 重应用 vibrancy /
/// `get_vibrancy_capability` 三处共用此函数，避免阈值散落与未来漏改。
/// 非 Windows 平台返回 false（mica 是 Windows 专有，macOS 走 NSVisualEffectMaterial 另判）。
pub(crate) fn supports_mica() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_version::OsVersion::current().build >= 22000
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// 施加玻璃主题效果（Win11 用 mica：背景仅采样一次，拖动跟手零开销，性能远优于 acrylic）；
/// 失败时记录中文日志并忽略，窗口退化为不透明实色不崩溃。
///
/// 设计权衡：mica vs acrylic
/// - mica：DWM 仅在窗口显示时**采样桌面壁纸一次**，窗口拖动期间不重新采样。Win11 设计语言。
/// - acrylic：DWM 实时模糊**当前窗口背后的桌面+其他窗口**，窗口高速移动时每帧重新采样并做高斯模糊，
///   开销大且依赖显卡驱动。0.4.4 的 mist/dusk 曾用 acrylic + tint，导致拖动卡顿。
/// - 本次改用 mica 以恢复 0.4.1 mica 主题的流畅手感（mica 不接受 tint，主题色调由前端 CSS 表面层提供）。
#[cfg(target_os = "windows")]
fn apply_glass_effect(window: &WebviewWindow, theme: &str, is_dark: bool) {
    if let Err(err) = window_vibrancy::apply_mica(window, Some(is_dark)) {
        eprintln!("[主题] 应用 mica 玻璃效果失败（{theme}），窗口退化为不透明实色背景: {err}");
    }
}

#[tauri::command]
pub fn set_theme(
    window: WebviewWindow,
    state: State<'_, SettingsState>,
    db_state: State<'_, DbState>,
    theme: String,
    color_mode: Option<String>,
    show_app_border: Option<bool>,
) -> AppResult<()> {
    // 先归一主题：接收旧别名/未知值也能正确分派，并以新主题值持久化与广播
    let theme = normalize_theme_id(&theme).to_string();

    let mut effective_color_mode = color_mode.clone();
    if effective_color_mode
        .as_deref()
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
    {
        effective_color_mode = db_state
            .settings_repo
            .get("app.color_mode")
            .unwrap_or(Some("system".to_string()));
    }
    let mut effective_show_app_border = show_app_border;
    if effective_show_app_border.is_none() {
        effective_show_app_border = db_state
            .settings_repo
            .get("app.show_app_border")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v != "false");
    }
    let show_border = effective_show_app_border.unwrap_or(true);

    if let Ok(mut guard) = state.theme.lock() {
        *guard = theme.clone();
    }

    #[cfg(target_os = "windows")]
    use windows::core::BOOL;
    #[cfg(target_os = "windows")]
    use windows::Win32::Foundation::HWND;
    #[cfg(target_os = "windows")]
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWM_WINDOW_CORNER_PREFERENCE,
    };

    #[cfg(target_os = "windows")]
    {
        let hwnd = window
            .hwnd()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let hwnd = HWND(hwnd.0 as _);
        let _ = window_vibrancy::clear_vibrancy(&window);

        let is_dark = match effective_color_mode.as_deref() {
            Some("light") => false,
            Some("dark") => true,
            _ => window.theme().unwrap_or(Theme::Dark) == Theme::Dark,
        };

        let dark_mode = BOOL::from(is_dark);
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const _ as _,
                std::mem::size_of::<BOOL>() as u32,
            );
            // Toggle native DWM border visibility while preserving the window frame/corners.
            const DWMWA_COLOR_DEFAULT: u32 = 0xFFFFFFFF;
            const DWMWA_COLOR_NONE: u32 = 0xFFFFFFFE;
            let border_color: u32 = if show_border {
                DWMWA_COLOR_DEFAULT
            } else {
                DWMWA_COLOR_NONE
            };
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &border_color as *const _ as _,
                std::mem::size_of::<u32>() as u32,
            );
            // Keep rounded corners even when border/shadow are disabled.
            let corner_pref = DWM_WINDOW_CORNER_PREFERENCE(DWMWCP_ROUND.0);
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const _ as _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            );
        }

        let build = windows_version::OsVersion::current().build;
        let is_win11 = build >= 22000;
        // mica 支持判定走共享函数 supports_mica()（Win11 唯一权威阈值定义点），
        // Win10 上玻璃主题降级为不透明实色，避免 acrylic 的实时背景模糊在拖动时造成卡顿。
        let supports_mica_now = supports_mica();

        match theme.as_str() {
            // 玻璃主题 mist：Win11 用 mica（DWM 仅采样壁纸一次，拖动跟手零开销）
            "mist" if supports_mica_now => {
                apply_glass_effect(&window, "mist", is_dark);
                let _ = window.set_shadow(show_border);
            }
            // 玻璃主题 dusk：同上，mica 不接受 tint，深紫色调由前端 CSS 表面层提供
            "dusk" if supports_mica_now => {
                apply_glass_effect(&window, "dusk", is_dark);
                let _ = window.set_shadow(show_border);
            }
            // 扁平主题 ink / paper（及玻璃主题在 Win10 上的降级）：
            // 已 clear_vibrancy，使用不透明实色 + 阴影
            _ => {
                let _ = window.set_shadow(show_border && is_win11 && !is_glass_theme(&theme));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let is_dark = match effective_color_mode.as_deref() {
            Some("light") => false,
            Some("dark") => true,
            _ => window.theme().unwrap_or(Theme::Dark) == Theme::Dark,
        };

        let _ = window_vibrancy::clear_vibrancy(&window);
        if is_glass_theme(&theme) {
            let _ = window_vibrancy::apply_vibrancy(
                &window,
                window_vibrancy::NSVisualEffectMaterial::HudWindow,
                None,
                None,
            );
        }
    }

    let _ = window.emit("theme-changed", theme);
    Ok(())
}

/// 查询当前平台是否支持玻璃 vibrancy。
///
/// - Windows 11（build ≥ 22000）：返回 true，玻璃主题（mist/dusk）使用 mica 渲染。
/// - Windows 10：返回 false，玻璃主题降级为不透明实色背景；前端应据此挂 `no-vibrancy`
///   class，使 mist.css / dusk.css 中的不透明 fallback 规则生效，避免透明窗口直接看到
///   桌面壁纸。
/// - 非 Windows（macOS 等）：返回 true（macOS 走 NSVisualEffectMaterial，玻璃可用）。
#[tauri::command]
pub fn get_vibrancy_capability() -> bool {
    #[cfg(target_os = "windows")]
    {
        supports_mica()
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{is_glass_theme, normalize_theme_id};

    /// `normalize_theme_id`：旧别名按表 4.1 精确映射为新主题。
    #[test]
    fn normalize_legacy_aliases() {
        assert_eq!(normalize_theme_id("mica"), "mist");
        assert_eq!(normalize_theme_id("sakura"), "mist");
        assert_eq!(normalize_theme_id("acrylic"), "dusk");
        assert_eq!(normalize_theme_id("retro"), "ink");
        assert_eq!(normalize_theme_id("sticky-note"), "paper");
    }

    /// `normalize_theme_id`：新主题值原样返回。
    #[test]
    fn normalize_new_themes_unchanged() {
        assert_eq!(normalize_theme_id("ink"), "ink");
        assert_eq!(normalize_theme_id("paper"), "paper");
        assert_eq!(normalize_theme_id("mist"), "mist");
        assert_eq!(normalize_theme_id("dusk"), "dusk");
    }

    /// `normalize_theme_id`：`store-*` 前缀一律落默认 `ink`。
    #[test]
    fn normalize_store_prefix_to_ink() {
        assert_eq!(normalize_theme_id("store-"), "ink");
        assert_eq!(normalize_theme_id("store-123"), "ink");
        assert_eq!(normalize_theme_id("store-dark-neon"), "ink");
    }

    /// `normalize_theme_id`：空串与未知值一律落默认 `ink`。
    #[test]
    fn normalize_empty_and_unknown_to_ink() {
        assert_eq!(normalize_theme_id(""), "ink");
        assert_eq!(normalize_theme_id("unknown"), "ink");
        assert_eq!(normalize_theme_id("Mica"), "ink"); // 区分大小写，非精确匹配落默认
        assert_eq!(normalize_theme_id("ink-extra"), "ink");
    }

    /// `normalize_theme_id`：满足幂等性——结果集合恒为 4 套新主题之一。
    #[test]
    fn normalize_is_idempotent() {
        for input in [
            "mica",
            "sakura",
            "acrylic",
            "retro",
            "sticky-note",
            "paper",
            "ink",
            "mist",
            "dusk",
            "store-x",
            "",
            "unknown",
        ] {
            let once = normalize_theme_id(input);
            assert_eq!(normalize_theme_id(once), once, "幂等性失败: {input}");
            assert!(
                matches!(once, "ink" | "paper" | "mist" | "dusk"),
                "归一结果不在新主题集合内: {input} -> {once}"
            );
        }
    }

    /// `is_glass_theme`：仅 mist/dusk（含其旧别名）为真。
    #[test]
    fn glass_only_for_mist_and_dusk() {
        // 新玻璃主题
        assert!(is_glass_theme("mist"));
        assert!(is_glass_theme("dusk"));
        // 玻璃主题的旧别名（mica/sakura→mist、acrylic→dusk）
        assert!(is_glass_theme("mica"));
        assert!(is_glass_theme("sakura"));
        assert!(is_glass_theme("acrylic"));
    }

    /// `is_glass_theme`：扁平主题（含旧别名）与未知/空值均为假。
    #[test]
    fn flat_and_unknown_are_not_glass() {
        // 新扁平主题
        assert!(!is_glass_theme("ink"));
        assert!(!is_glass_theme("paper"));
        // 扁平主题的旧别名（retro→ink、sticky-note→paper）
        assert!(!is_glass_theme("retro"));
        assert!(!is_glass_theme("sticky-note"));
        // store-* / 空串 / 未知值
        assert!(!is_glass_theme("store-foo"));
        assert!(!is_glass_theme(""));
        assert!(!is_glass_theme("unknown"));
    }

    /// Property 3: 前后端判定一致
    /// **Validates: Requirements 4.1, 4.2, 4.3**
    ///
    /// 以共享测试向量表（前后端唯一权威输入源，前端 glassParity.test.ts 读取同一文件）
    /// 驱动后端 `is_glass_theme` 对照：逐向量断言后端结果等于向量期望 `expectedGlass`，
    /// 且 `expectedGlass` 仅在归一后为 mist/dusk 时为真。两侧对每个向量结论一致即证前后端判定一致。
    #[test]
    fn glass_parity_matches_shared_vectors() {
        // include_str! 在编译期嵌入同一份 JSON 向量表（路径相对本文件）
        const VECTORS_JSON: &str = include_str!(
            "../../../../src/shared/config/__fixtures__/glassThemeVectors.json"
        );

        let parsed: serde_json::Value =
            serde_json::from_str(VECTORS_JSON).expect("共享向量表 JSON 解析失败");
        let vectors = parsed["vectors"]
            .as_array()
            .expect("共享向量表缺少 vectors 数组");

        // 断言四类输入均被覆盖（合法值/旧别名/空串/未知值）
        let mut seen_categories = std::collections::HashSet::new();
        for v in vectors {
            seen_categories.insert(v["category"].as_str().expect("向量缺少 category"));
        }
        for expected in ["legal", "legacy", "empty", "unknown"] {
            assert!(
                seen_categories.contains(expected),
                "共享向量表未覆盖输入类别: {expected}"
            );
        }

        // 逐向量对照
        for v in vectors {
            let input = v["input"].as_str().expect("向量 input 必须为字符串");
            let expected_glass = v["expectedGlass"].as_bool().expect("向量缺少 expectedGlass");

            // 后端判定与向量期望完全一致
            assert_eq!(
                is_glass_theme(input),
                expected_glass,
                "is_glass_theme({input:?}) 与共享向量期望不一致"
            );
            // expectedGlass 当且仅当归一后为 mist/dusk
            assert_eq!(
                expected_glass,
                matches!(normalize_theme_id(input), "mist" | "dusk"),
                "向量 {input:?} 的 expectedGlass 与归一后 mist/dusk 判定不符"
            );
        }
    }
}
