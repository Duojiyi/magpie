#[cfg(target_os = "windows")]
use crate::app::hooks::{keyboard_proc, mouse_proc};
#[cfg(target_os = "windows")]
use crate::app::system::tray_subclass_proc;
use crate::app::window_manager::{release_win_keys, restore_last_focus, toggle_window};
use crate::app_state::{
    AppDataDir, EncryptionQueueState, PasteQueue, SessionHistory, SettingsState,
};
use crate::database::{self, DbState};
use crate::global_state::*;
use crate::info;
use crate::infrastructure::repository::clipboard_repo::SqliteClipboardRepository;
use crate::infrastructure::repository::settings_repo::{
    SettingsRepository, SqliteSettingsRepository,
};
use crate::infrastructure::repository::tag_repo::SqliteTagRepository;
use crate::infrastructure::windows_ext::WindowExt;
use crate::services::encryption_queue::init_encryption_queue;
use crate::services::sensitive_align::spawn_sensitive_alignment;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{App, AppHandle, Emitter, Manager};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HINSTANCE, HWND, POINT, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::SetWindowSubclass;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowRect, RegisterWindowMessageW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
};

static WINDOW_SIZE_SAVE_PENDING: AtomicBool = AtomicBool::new(false);
static LAST_WINDOW_SIZE_EVENT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_WINDOW_SIZE: OnceLock<Mutex<(u32, u32)>> = OnceLock::new();

/// A10(需求 8.8)：标记本次启动是否因「期望便携但 data 目录缺失」而降级为标准模式。
/// `resolve_data_dir` 阶段前端 webview 尚未就绪，无法直接 emit；此处先置位，
/// 待启动后段（窗口/服务就绪）再向前端 emit 提示。
static PORTABLE_DEGRADED_TO_STANDARD: AtomicBool = AtomicBool::new(false);

/// A8(需求 6.5/6.7)：标记本次启动是否因 v0.4.0 数据迁移失败降级而改用 legacy 目录。
/// 与 PORTABLE_DEGRADED_TO_STANDARD 同理：`resolve_data_dir` 阶段 webview 未就绪，
/// 先置位，待启动后段再向前端 emit「数据迁移未完成，已使用原有数据启动」提示。
static MIGRATION_DEGRADED_TO_LEGACY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
struct WindowRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

pub fn init(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // Initialize GLOBAL_APP_HANDLE for Win32 hooks
    let _ = GLOBAL_APP_HANDLE.set(app_handle.clone());

    // 1. Data Directory & Migration
    let app_dir = resolve_data_dir(app)?;

    // 2. Logger Initialization
    crate::logger::init(app_dir.join("tiez.log"));
    info!(">>> [STARTUP] Magpie starting up...");

    // 3. Database Initialization
    let db_path = app_dir.join("clipboard.db");
    let db_path_str = db_path.to_string_lossy();
    let conn = database::init_db(&db_path_str).map_err(|e| {
        let err_msg = format!("数据库初始化失败: {}", e);
        WindowExt::show_error_box("Magpie 启动错误", &err_msg);
        e
    })?;
    let conn_arc = std::sync::Arc::new(std::sync::Mutex::new(conn));
    let settings_repo = SqliteSettingsRepository::new(conn_arc.clone());

    // 4. Initial Settings & Reset Safety
    apply_startup_resets(&settings_repo);

    let settings = load_settings(&settings_repo);

    // 5. App State Management
    setup_state(app, conn_arc.clone(), &settings, app_dir.clone());
    app.manage(EncryptionQueueState(init_encryption_queue(
        app_handle.clone(),
    )));
    spawn_sensitive_alignment(app_handle.clone());

    // 5.1 C4(需求 23.2)：异步执行 schema 健康检查与维护。
    // DbState 已在 setup_state 中 manage（首屏依赖的 schema 创建/迁移已在 init_db 同步完成），
    // 这里复用同一共享连接在后台做「不影响首屏」的关键表校验与 WAL/统计维护。
    database::spawn_schema_check(conn_arc.clone());

    // 6. Window Initialization (Pinned/Focus 等几何与样式，不含 show)
    setup_main_window(app, &settings);

    // 6.1 External Drag-Drop (Web Images)
    #[cfg(windows)]
    crate::infrastructure::windows_api::drag_drop::register_emoji_drag_drop(app_handle.clone());

    // 7. C4(需求 23.3) + 启动闪烁修复：先在「隐藏」状态下应用主题(DWM mica/acrylic vibrancy)，
    // 再显示窗口。窗口在 tauri.conf.json 配置为 visible:false 启动，避免透明窗口在
    // vibrancy 生效前以系统默认不透明背景短暂显示（便携版 release 下尤为明显的「白框一闪」）。
    // apply_mica/apply_acrylic 基于 HWND 的 DWM 调用对隐藏窗口同样有效，因此可先设透明、后显示。
    apply_initial_theme(app);

    // 8. 主题(透明效果)就绪后再显示窗口骨架，此时窗口一出现即为透明，无不透明方框闪烁。
    show_window_skeleton(app, &settings);

    // 9. Background Services & Monitors（C4 需求 23.4：tokio::join! 并行启动）
    start_services(app, &settings, app_handle.clone());

    // 10. Tray Setup
    setup_tray(app, settings.hide_tray_icon);

    // 11. Win32 Hook Initialization
    #[cfg(target_os = "windows")]
    init_win32_hooks(app);

    // 12. TaskbarCreated & Subclass
    #[cfg(target_os = "windows")]
    setup_taskbar_listener(app);

    // 13. 便携降级提示（需求 8.8）：若本次因 data 目录缺失而降级为标准模式，
    // 延迟向前端 emit 提示，确保 webview 已挂载事件监听后能收到。
    emit_portable_degraded_notice_if_needed(app_handle.clone());

    // 14. 迁移降级提示（需求 6.5/6.7）：若本次因 v0.4.0 迁移失败降级而改用 legacy 目录，
    // 同样延迟向前端 emit 提示。
    emit_migration_degraded_notice_if_needed(app_handle.clone());

    Ok(())
}

/// A10(需求 8.8)：若启动时检测到「期望便携但 data 目录不存在」已降级为标准模式，
/// 则向前端 emit「已降级为标准模式运行」提示。延迟发送以等待前端事件监听就绪。
fn emit_portable_degraded_notice_if_needed(app_handle: AppHandle) {
    if !PORTABLE_DEGRADED_TO_STANDARD.load(Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        // 等待前端 webview 完成挂载，避免事件先于监听器发出而丢失。
        std::thread::sleep(std::time::Duration::from_secs(3));
        let _ = app_handle.emit("portable-degraded-to-standard", "已降级为标准模式运行");
    });
}

/// A8(需求 6.5/6.7)：若启动时 v0.4.0 数据迁移失败已降级到 legacy 目录（com.tiez），
/// 则向前端 emit「数据迁移未完成，已使用原有数据启动」提示。延迟发送以等待前端事件监听就绪。
fn emit_migration_degraded_notice_if_needed(app_handle: AppHandle) {
    if !MIGRATION_DEGRADED_TO_LEGACY.load(Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        // 等待前端 webview 完成挂载，避免事件先于监听器发出而丢失。
        std::thread::sleep(std::time::Duration::from_secs(3));
        let _ = app_handle.emit(
            "migration-degraded-to-legacy",
            "数据迁移未完成，已使用原有数据启动",
        );
    });
}

fn resolve_data_dir(app: &App) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let default_app_dir = app.path().app_data_dir()?;

    // Perform migration if needed
    crate::migration::perform_migration_v028(&default_app_dir);
    let migration_outcome = crate::migration::perform_migration_v040(&default_app_dir);

    // A8(需求 6.5/6.7)：迁移失败降级时，本次启动改用 legacy 目录（com.tiez）作为基准目录，
    // 后续 datapath 重定向解析与回退都以该基准目录为准；正常情况下基准目录为 app.magpie。
    let degraded_legacy = matches!(
        migration_outcome,
        crate::migration::MigrationOutcome::DegradedToLegacy(_)
    );
    let base_dir = match migration_outcome {
        crate::migration::MigrationOutcome::DegradedToLegacy(legacy) => legacy,
        crate::migration::MigrationOutcome::UseTarget => default_app_dir.clone(),
    };

    // Cleanup temp files
    std::thread::spawn(|| {
        let temp_dir = std::env::temp_dir();
        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("TieZ_Clip_") || name.starts_with("Magpie_Clip_") {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    });

    let redirect_file = base_dir.join("datapath.txt");
    let mut app_dir = if redirect_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&redirect_file) {
            let custom_path = content.trim();
            if custom_path.is_empty() {
                base_dir.clone()
            } else if !drive_root_exists(custom_path) {
                // A5(需求 4)：自定义数据目录所在盘符根不存在（如外接盘已拔出），
                // 回退基准目录，并向 tiez.log 追加回退原因。
                // 此阶段 logger 尚未初始化，直接以 append 方式写入目标日志文件。
                append_datapath_fallback_log(
                    &base_dir,
                    &format!(
                        "datapath.txt 指向的盘符不存在，已回退默认数据目录。custom_path={}",
                        custom_path
                    ),
                );
                base_dir.clone()
            } else if std::path::Path::new(custom_path).exists() {
                std::path::PathBuf::from(custom_path)
            } else {
                base_dir.clone()
            }
        } else {
            base_dir.clone()
        }
    } else {
        base_dir.clone()
    };

    // Portable mode check
    let mut portable_used = false;
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let portable_data = exe_dir.join("data");
            let portable_data_exists = portable_data.exists() && portable_data.is_dir();
            let expecting_portable = is_expecting_portable(exe_dir);
            // 便携数据目录决策抽为纯函数 decide_portable_data_dir，副作用（写日志/置位降级标记）留在此处。
            let (resolved, used, degraded) = decide_portable_data_dir(
                &app_dir,
                &portable_data,
                &default_app_dir,
                portable_data_exists,
                expecting_portable,
            );
            app_dir = resolved;
            portable_used = used;
            if degraded {
                // A10(需求 8.8)：检测到便携标志（README_PORTABLE.md）说明期望以便携模式运行，
                // 但 exe_dir/data/ 缺失（如被误删）。降级为标准模式：使用 %APPDATA%\app.magpie，
                // 向 tiez.log 追加说明（此阶段 logger 未初始化，沿用 append 写日志模式），
                // 并标记降级以便启动后段向前端 emit「已降级为标准模式运行」提示。
                append_datapath_fallback_log(
                    &default_app_dir,
                    "检测到便携标志但 data 目录不存在，已降级为标准模式运行（使用 %APPDATA%\\app.magpie）。",
                );
                PORTABLE_DEGRADED_TO_STANDARD.store(true, Ordering::SeqCst);
            }
        }
    }

    // A8(需求 6.5/6.7)：迁移失败降级且本次最终未走便携数据目录时，标记使用了 legacy 目录，
    // 以便启动后段向前端 emit「数据迁移未完成，已使用原有数据启动」提示。
    MIGRATION_DEGRADED_TO_LEGACY.store(degraded_legacy && !portable_used, Ordering::SeqCst);

    std::fs::create_dir_all(&app_dir)?;
    Ok(app_dir)
}

/// A10(需求 8.8)：便携模式数据目录决策（纯函数，便于单元测试）。
///
/// 输入：
/// - `current_app_dir`：datapath 解析后的候选数据目录（默认为基准目录）。
/// - `portable_data`：exe 同级 `data` 目录路径。
/// - `default_app_dir`：标准模式数据目录 `%APPDATA%\app.magpie`。
/// - `portable_data_exists`：`portable_data` 是否存在且为目录。
/// - `expecting_portable`：是否检测到便携标志（README_PORTABLE.md）。
///
/// 返回 `(最终数据目录, 是否便携模式, 是否降级为标准模式)`：
/// - 便携 data 目录存在 → 使用便携目录；
/// - 否则若期望便携但 data 缺失 → 降级为标准模式（使用 `default_app_dir`）；
/// - 否则 → 保持候选目录不变（标准安装，非便携）。
fn decide_portable_data_dir(
    current_app_dir: &std::path::Path,
    portable_data: &std::path::Path,
    default_app_dir: &std::path::Path,
    portable_data_exists: bool,
    expecting_portable: bool,
) -> (std::path::PathBuf, bool, bool) {
    if portable_data_exists {
        (portable_data.to_path_buf(), true, false)
    } else if expecting_portable {
        (default_app_dir.to_path_buf(), false, true)
    } else {
        (current_app_dir.to_path_buf(), false, false)
    }
}

/// A10(需求 8.8)：判断是否「期望以便携模式运行」。
/// 便携包由 `scripts/build-portable.ps1` 打包，独有 `README_PORTABLE.md` 标志文件
/// （安装版不含此文件）。据此在 data 目录缺失时区分「期望便携」与「标准安装」。
fn is_expecting_portable(exe_dir: &std::path::Path) -> bool {
    exe_dir.join("README_PORTABLE.md").exists()
}

/// A5(需求 4)：校验路径所在盘符根是否存在。
/// 提取形如 `E:\` 的盘符根并判断其存在性；当外接盘被拔出时返回 false。
/// 解析不出盘符（非 Windows 盘符前缀，如 UNC 路径）时按原路径自身存在性判定。
fn drive_root_exists(path: &str) -> bool {
    let bytes = path.as_bytes();
    // 形如 "E:\..." 或 "e:/..."：取盘符字母 + ":\" 作为盘符根
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive_root = format!("{}:\\", (bytes[0] as char).to_ascii_uppercase());
        return std::path::Path::new(&drive_root).exists();
    }
    // 非标准盘符前缀（如 UNC \\server\share）：回退到路径自身存在性判定。
    std::path::Path::new(path).exists()
}

/// A5(需求 4.3)：因盘符不存在而回退默认目录时，向目标 tiez.log 追加一条说明日志。
/// 此函数在 logger 初始化之前调用，故直接以 append 方式写入文件。
fn append_datapath_fallback_log(default_app_dir: &std::path::Path, reason: &str) {
    use std::io::Write;
    // 确保默认目录存在，避免日志写入失败。
    let _ = std::fs::create_dir_all(default_app_dir);
    let log_path = default_app_dir.join("tiez.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "[DATAPATH FALLBACK] {}", reason);
    }
}

/// F4(需求 18.3/18.6)：首启拷贝内置精选表情。
///
/// 用户表情库（F4，需求 18）默认为空，由用户自行通过「添加到表情包」加入。
/// v0.4.1 起不再随包提供内置精选表情，也不在首启拷贝任何内置素材
/// （此前的 copy_builtin_emojis_if_needed 机制已移除，空表情库为有意设计）。

fn apply_startup_resets(repo: &impl SettingsRepository) {
    let paste_method = repo
        .get("app.paste_method")
        .unwrap_or(Some("shift_insert".to_string()))
        .unwrap_or("shift_insert".to_string());
    if paste_method == "game_mode" && !crate::app::commands::system_cmd::check_is_admin() {
        info!(">>> [STARTUP] Game Mode active without Admin privileges. Resetting to default.");
        let _ = repo.set("app.paste_method", "shift_insert");
    }
}

pub struct StartupSettings {
    pub theme: String,
    pub persistent: bool,
    pub capture_files: bool,
    pub capture_rich_text: bool,
    pub deduplicate: bool,
    pub auto_copy_file: bool,
    pub silent_start: bool,
    pub delete_after_paste: bool,
    pub privacy_protection: bool,
    pub privacy_kinds: String,
    pub privacy_custom: String,
    pub cleanup_rules: String,
    pub app_cleanup_policies: String,
    pub sequential_mode: bool,
    pub sequential_hotkey: String,
    pub rich_paste_hotkey: String,
    pub search_hotkey: String,
    pub quick_paste_modifier: String,
    pub sound_enabled: bool,
    pub hide_tray_icon: bool,
    pub edge_docking: bool,
    pub follow_mouse: bool,
    pub window_pinned: bool,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub main_hotkey: String,
    pub arrow_key_selection: bool,
    pub auto_close_server: bool,
}

fn load_settings(repo: &impl SettingsRepository) -> StartupSettings {
    StartupSettings {
        // 主题统一兜底为 ink，并先经 normalize_theme_id 归一旧别名（mica/acrylic/sakura/retro/
        // sticky-note/store-* 等），使后端启动状态直接持有迁移后的新主题值，
        // 与玻璃判定（mist/dusk）保持一致，避免后端用旧值导致玻璃效果不生效。
        theme: {
            let raw = repo
                .get("app.theme")
                .unwrap_or(Some("ink".to_string()))
                .unwrap_or("ink".to_string());
            crate::app::commands::ui_cmd::normalize_theme_id(&raw).to_string()
        },
        persistent: repo
            .get("app.persistent")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v == "true")
            .unwrap_or(true),
        capture_files: repo
            .get("app.capture_files")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v == "true")
            .unwrap_or(true),
        capture_rich_text: repo
            .get("app.capture_rich_text")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        deduplicate: repo
            .get("app.deduplicate")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v == "true")
            .unwrap_or(true),
        auto_copy_file: repo
            .get("file_transfer_auto_copy")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        silent_start: repo
            .get("app.silent_start")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v == "true")
            .unwrap_or(true),
        delete_after_paste: repo
            .get("app.delete_after_paste")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        privacy_protection: repo
            .get("app.privacy_protection")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v == "true")
            .unwrap_or(true),
        privacy_kinds: repo
            .get("app.privacy_protection_kinds")
            .unwrap_or(Some("phone,idcard,email,secret".to_string()))
            .unwrap_or("phone,idcard,email,secret".to_string()),
        privacy_custom: repo
            .get("app.privacy_protection_custom_rules")
            .unwrap_or(Some("".to_string()))
            .unwrap_or("".to_string()),
        cleanup_rules: repo
            .get("app.cleanup_rules")
            .unwrap_or(Some("".to_string()))
            .unwrap_or("".to_string()),
        app_cleanup_policies: repo
            .get("app.app_cleanup_policies")
            .unwrap_or(Some("[]".to_string()))
            .unwrap_or("[]".to_string()),
        sequential_mode: repo
            .get("app.sequential_mode")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        sequential_hotkey: repo
            .get("app.sequential_hotkey")
            .unwrap_or(Some("Alt+V".to_string()))
            .unwrap_or("Alt+V".to_string()),
        rich_paste_hotkey: repo
            .get("app.rich_paste_hotkey")
            .unwrap_or(Some("Ctrl+Shift+Z".to_string()))
            .unwrap_or("Ctrl+Shift+Z".to_string()),
        search_hotkey: repo
            .get("app.search_hotkey")
            .unwrap_or(Some("Alt+F".to_string()))
            .unwrap_or("Alt+F".to_string()),
        quick_paste_modifier: repo
            .get("app.quick_paste_modifier")
            .unwrap_or(Some("disabled".to_string()))
            .unwrap_or("disabled".to_string()),
        sound_enabled: repo
            .get("app.sound_enabled")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        hide_tray_icon: repo
            .get("app.hide_tray_icon")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        edge_docking: repo
            .get("app.edge_docking")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        follow_mouse: repo
            .get("app.follow_mouse")
            .unwrap_or(Some("true".to_string()))
            .map(|v| v == "true")
            .unwrap_or(true),
        window_pinned: repo
            .get("app.window_pinned")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        window_width: repo
            .get("app.window_width")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u32>().ok()),
        window_height: repo
            .get("app.window_height")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u32>().ok()),
        main_hotkey: repo
            .get("app.hotkey")
            .unwrap_or(Some("Win+V".to_string()))
            .unwrap_or("Win+V".to_string()),
        arrow_key_selection: repo
            .get("app.arrow_key_selection")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
        auto_close_server: repo
            .get("file_transfer_auto_close")
            .unwrap_or(Some("false".to_string()))
            .map(|v| v == "true")
            .unwrap_or(false),
    }
}

fn setup_state(
    app: &App,
    conn_arc: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    s: &StartupSettings,
    app_dir: std::path::PathBuf,
) {
    let repo = SqliteClipboardRepository::new(conn_arc.clone());
    let settings_repo = SqliteSettingsRepository::new(conn_arc.clone());
    let tag_repo = SqliteTagRepository::new(conn_arc.clone());
    app.manage(DbState {
        conn: conn_arc,
        repo,
        settings_repo,
        tag_repo,
    });

    app.manage(SettingsState {
        deduplicate: AtomicBool::new(s.deduplicate),
        persistent: AtomicBool::new(s.persistent),
        file_server_auto_close: AtomicBool::new(s.auto_close_server),
        theme: std::sync::Mutex::new(s.theme.clone()),
        capture_files: AtomicBool::new(s.capture_files),
        capture_rich_text: AtomicBool::new(s.capture_rich_text),
        auto_copy_file: AtomicBool::new(s.auto_copy_file),
        silent_start: AtomicBool::new(s.silent_start),
        delete_after_paste: AtomicBool::new(s.delete_after_paste),
        privacy_protection: AtomicBool::new(s.privacy_protection),
        privacy_protection_kinds: std::sync::Mutex::new(
            s.privacy_kinds
                .split(',')
                .map(|x| x.trim().to_string())
                .collect(),
        ),
        privacy_protection_custom_rules: std::sync::Mutex::new(
            s.privacy_custom
                .lines()
                .map(|x| x.trim().to_string())
                .collect(),
        ),
        cleanup_rules: std::sync::Mutex::new(s.cleanup_rules.clone()),
        app_cleanup_policies: std::sync::Mutex::new(s.app_cleanup_policies.clone()),
        sequential_mode: AtomicBool::new(s.sequential_mode),
        sequential_paste_hotkey: std::sync::Mutex::new(s.sequential_hotkey.clone()),
        rich_paste_hotkey: std::sync::Mutex::new(s.rich_paste_hotkey.clone()),
        search_hotkey: std::sync::Mutex::new(s.search_hotkey.clone()),
        quick_paste_modifier: std::sync::Mutex::new(s.quick_paste_modifier.clone()),
        sound_enabled: AtomicBool::new(s.sound_enabled),
        hide_tray_icon: AtomicBool::new(s.hide_tray_icon),
        edge_docking: AtomicBool::new(s.edge_docking),
        follow_mouse: AtomicBool::new(s.follow_mouse),
        arrow_key_selection: AtomicBool::new(s.arrow_key_selection),
        main_hotkey: std::sync::Mutex::new(s.main_hotkey.clone()),
        monitors: std::sync::Mutex::new(Vec::new()),
    });

    app.manage(SessionHistory(std::sync::Mutex::new(
        std::collections::VecDeque::new(),
    )));
    app.manage(AppDataDir(std::sync::Mutex::new(app_dir)));
    app.manage(crate::services::file_transfer::ChatState::default());
    app.manage(crate::services::file_transfer::SharedFileState(
        std::sync::Mutex::new(std::collections::HashMap::new()),
    ));
    app.manage(crate::services::file_transfer::ServerInfo {
        port: std::sync::atomic::AtomicU16::new(0),
        ip: std::sync::Mutex::new(String::new()),
    });
    app.manage(crate::services::file_transfer::UploadSessions::default());
    app.manage(crate::services::file_transfer::ServerActivityState::default());
    app.manage(crate::services::file_transfer::WsBroadcaster(
        std::sync::Mutex::new(None),
    ));
    app.manage(crate::services::file_transfer::OnlineDevices(
        std::sync::Mutex::new(std::collections::HashMap::new()),
    ));
    app.manage(PasteQueue::default());
}

fn setup_main_window(app: &App, s: &StartupSettings) {
    let effective_pinned = s.window_pinned;
    WINDOW_PINNED.store(effective_pinned, Ordering::Relaxed);

    if let Some(window) = app.get_webview_window("main") {
        if let (Some(w), Some(h)) = (s.window_width, s.window_height) {
            if w >= 360 && h >= 240 {
                let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: w,
                    height: h,
                }));
            }
        }
        let _ = window.set_always_on_top(effective_pinned);
        let _ = window.set_focusable(!effective_pinned);

        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let ex_style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                    HWND(hwnd.0),
                    GWL_EXSTYLE,
                );
                if effective_pinned {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                        HWND(hwnd.0),
                        GWL_EXSTYLE,
                        ex_style | WS_EX_NOACTIVATE.0 as isize,
                    );
                } else {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                        HWND(hwnd.0),
                        GWL_EXSTYLE,
                        ex_style & !(WS_EX_NOACTIVATE.0 as isize),
                    );
                }
            }
        }

        if repair_window_position_if_needed(&window, s.edge_docking) {
            IS_HIDDEN.store(false, Ordering::Relaxed);
            CURRENT_DOCK.store(0, Ordering::Relaxed);
        }
    }

    schedule_window_position_repair(app.handle().clone(), s.edge_docking);
}

/// C4(需求 23.3)：在主题应用之前先显示窗口骨架。
///
/// 从 `setup_main_window` 中拆出窗口 `show` 逻辑，使「显示骨架 → 应用主题」的顺序
/// 在 `init` 中显式可见：先让 webview 骨架尽快可见，再做耗时的 DWM/vibrancy 主题应用。
/// 仍遵循静默启动语义：`--autostart`/`--minimized` 或开启「静默启动」时不显示窗口。
fn show_window_skeleton(app: &App, s: &StartupSettings) {
    let args: Vec<String> = std::env::args().collect();
    let is_autostart =
        args.contains(&"--autostart".to_string()) || args.contains(&"--minimized".to_string());
    if !is_autostart && !s.silent_start {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
        }
    }
}

fn schedule_window_position_repair(app_handle: AppHandle, edge_docking_enabled: bool) {
    std::thread::spawn(move || {
        for _ in 0..8 {
            std::thread::sleep(std::time::Duration::from_millis(250));

            let Some(window) = app_handle.get_webview_window("main") else {
                continue;
            };

            if repair_window_position_if_needed(&window, edge_docking_enabled) {
                IS_HIDDEN.store(false, Ordering::Relaxed);
                CURRENT_DOCK.store(0, Ordering::Relaxed);
                info!(">>> [STARTUP] Repaired off-screen window position after state restore.");
                break;
            }
        }
    });
}

/// 多屏位置修复：当窗口矩形在所有显示器上都不足够可见（离屏/显示器掉线/分辨率变更）时，
/// 复用 `clamp_window_rect_to_monitor` 将其钳制回目标显示器可见区域。返回是否发生了修复。
/// 既用于启动期，也供唤起（`toggle_window`）路径做多屏定位兜底（需求 13.1）。
pub(crate) fn repair_window_position_if_needed(
    window: &tauri::WebviewWindow,
    edge_docking_enabled: bool,
) -> bool {
    let Ok(position) = window.outer_position() else {
        return false;
    };
    let Ok(size) = window.outer_size() else {
        return false;
    };
    let Ok(monitors) = window.available_monitors() else {
        return false;
    };
    if monitors.is_empty() {
        return false;
    }

    let rect = WindowRect {
        x: position.x,
        y: position.y,
        width: size.width as i32,
        height: size.height as i32,
    };

    if rect.width <= 0 || rect.height <= 0 {
        return false;
    }

    let visible_enough = monitors
        .iter()
        .any(|monitor| window_rect_has_enough_visible_area(rect, monitor, edge_docking_enabled));
    if visible_enough {
        return false;
    }

    let target_monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| monitors.first().cloned());

    let Some(monitor) = target_monitor else {
        return false;
    };

    let (target_x, target_y) = clamp_window_rect_to_monitor(rect, &monitor);
    if target_x == rect.x && target_y == rect.y {
        return false;
    }

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
        x: target_x,
        y: target_y,
    }));
    true
}

fn window_rect_has_enough_visible_area(
    rect: WindowRect,
    monitor: &tauri::Monitor,
    edge_docking_enabled: bool,
) -> bool {
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let monitor_left = monitor_pos.x;
    let monitor_top = monitor_pos.y;
    let monitor_right = monitor_left + monitor_size.width as i32;
    let monitor_bottom = monitor_top + monitor_size.height as i32;

    let visible_left = rect.x.max(monitor_left);
    let visible_top = rect.y.max(monitor_top);
    let visible_right = (rect.x + rect.width).min(monitor_right);
    let visible_bottom = (rect.y + rect.height).min(monitor_bottom);
    let visible_width = (visible_right - visible_left).max(0);
    let visible_height = (visible_bottom - visible_top).max(0);

    if visible_width == 0 || visible_height == 0 {
        return false;
    }

    let min_visible_width = if edge_docking_enabled {
        24.min(rect.width)
    } else {
        1
    };
    let min_visible_height = if edge_docking_enabled {
        24.min(rect.height)
    } else {
        1
    };

    visible_width >= min_visible_width && visible_height >= min_visible_height
}

fn clamp_window_rect_to_monitor(rect: WindowRect, monitor: &tauri::Monitor) -> (i32, i32) {
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let margin = 10;

    let min_x = monitor_pos.x + margin;
    let min_y = monitor_pos.y + margin;
    let max_x = (monitor_pos.x + monitor_size.width as i32 - rect.width - margin).max(min_x);
    let max_y = (monitor_pos.y + monitor_size.height as i32 - rect.height - margin).max(min_y);

    let target_x = if rect.width + margin * 2 >= monitor_size.width as i32 {
        monitor_pos.x
    } else {
        rect.x.clamp(min_x, max_x)
    };
    let target_y = if rect.height + margin * 2 >= monitor_size.height as i32 {
        monitor_pos.y
    } else {
        rect.y.clamp(min_y, max_y)
    };

    (target_x, target_y)
}

fn start_services(app: &App, s: &StartupSettings, app_handle: AppHandle) {
    // C4(需求 23.4)：用 `tokio::join!` 并行启动后台服务。
    // 这些启动函数本身是非阻塞的（内部各自 spawn 线程/异步任务），用 join! 并发驱动
    // 其启动逻辑，避免顺序等待。前提：依赖的 DbState/SettingsState 等已在 setup_state 中 manage。
    // 各 future 内不持有 DbState 锁、不跨 await 借用 app，保证无数据竞争。
    let (h_track, h_clip, h_mqtt, h_cloud, h_edge) = (
        app_handle.clone(),
        app_handle.clone(),
        app_handle.clone(),
        app_handle.clone(),
        app_handle.clone(),
    );
    tauri::async_runtime::block_on(async move {
        tokio::join!(
            async {
                crate::infrastructure::windows_api::window_tracker::start_window_tracking(h_track);
            },
            async {
                crate::services::clipboard::start_clipboard_monitor(h_clip);
            },
            async {
                crate::services::mqtt_sub::start_mqtt_client(h_mqtt);
            },
            async {
                crate::services::cloud_sync::start_cloud_sync_client(h_cloud);
            },
            async {
                start_edge_docking_monitor(h_edge);
            },
        );
    });

    let db_state = app.state::<DbState>();
    if db_state
        .settings_repo
        .get("file_server_enabled")
        .unwrap_or(Some("false".to_string()))
        == Some("true".to_string())
    {
        let port = db_state
            .settings_repo
            .get("file_server_port")
            .unwrap_or(None)
            .and_then(|x| x.parse::<u16>().ok());

        let h = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::services::file_transfer::toggle_file_server(h, true, port).await;
        });
    }

    // Daily app announcement ping
    init_announcement_ping(app, &db_state.settings_repo);

    // Register active hotkeys based on current settings.
    let _ = crate::app::commands::register_hotkey(app_handle.clone(), s.main_hotkey.clone());

    // Win+V 接管唤起（默认开启）：
    // app.use_win_v_shortcut 缺省视为 "true"，启动时确保系统 Win+V 已被接管
    // （写注册表 DisabledHotkeys 禁用系统剪贴板历史），并同步键盘钩子的拦截开关。
    let win_v_enabled = db_state
        .settings_repo
        .get("app.use_win_v_shortcut")
        .unwrap_or(Some("true".to_string()))
        != Some("false".to_string());

    WIN_V_TAKEOVER_ENABLED.store(win_v_enabled, Ordering::Relaxed);

    if win_v_enabled {
        if !crate::app::commands::system_cmd::get_registry_win_v_optimized_status() {
            let _ = crate::app::commands::trigger_registry_win_v_optimization(true);
        }
    }
}

#[cfg(target_os = "windows")]
fn start_edge_docking_monitor(app_handle: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(150));

            let settings = match app_handle.try_state::<SettingsState>() {
                Some(s) => s,
                None => continue,
            };

            if !settings.edge_docking.load(Ordering::Relaxed) {
                if IS_HIDDEN.load(Ordering::Relaxed) {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        IS_HIDDEN.store(false, Ordering::Relaxed);
                        CURRENT_DOCK.store(0, Ordering::Relaxed);
                    }
                }
                continue;
            }

            if let Some(window) = app_handle.get_webview_window("main") {
                // Skip if window is minimized
                if window.is_minimized().unwrap_or(false) {
                    continue;
                }

                let is_window_visible = window.is_visible().unwrap_or(true);
                let is_hidden_by_edge = IS_HIDDEN.load(Ordering::Relaxed);

                // Skip edge docking checks if window was hidden by other mechanisms (paste, blur, etc.)
                if !is_window_visible && !is_hidden_by_edge {
                    continue;
                }

                let last_show = LAST_SHOW_TIMESTAMP.load(Ordering::Relaxed);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                // While the clipboard window is actively shown via hotkey navigation,
                // avoid immediate auto-docking right after showing.
                if !is_hidden_by_edge
                    && NAVIGATION_ENABLED.load(Ordering::SeqCst)
                    && now.saturating_sub(last_show) < 800
                {
                    continue;
                }

                // Grace period after showing to prevent immediate re-dock
                if now.saturating_sub(last_show) < 500 {
                    continue;
                }

                let mut rect = RECT::default();
                let hwnd = match window.hwnd() {
                    Ok(h) => h,
                    Err(_) => continue,
                };
                unsafe {
                    let _ = GetWindowRect(HWND(hwnd.0), &mut rect);
                }

                let mut point = POINT::default();
                unsafe {
                    let _ = GetCursorPos(&mut point);
                }

                // Get current monitor info and validate
                let monitor = match window.current_monitor() {
                    Ok(Some(m)) => m,
                    _ => continue,
                };
                let screen_size = monitor.size();
                let screen_pos = monitor.position();

                // Calculate monitor boundaries
                let screen_left = screen_pos.x;
                let screen_top = screen_pos.y;
                let screen_right = screen_pos.x + screen_size.width as i32;
                let screen_bottom = screen_pos.y + screen_size.height as i32;

                // When hidden, check if mouse is near the edge sliver
                let threshold = 5;
                let is_mouse_near_edge = if is_hidden_by_edge {
                    let current_dock = CURRENT_DOCK.load(Ordering::Relaxed);
                    match current_dock {
                        1 => {
                            point.y <= screen_top + threshold
                                && point.x >= rect.left
                                && point.x <= rect.right
                        } // Top
                        2 => {
                            point.x <= screen_left + threshold
                                && point.y >= rect.top
                                && point.y <= rect.bottom
                        } // Left
                        3 => {
                            point.x >= screen_right - threshold
                                && point.y >= rect.top
                                && point.y <= rect.bottom
                        } // Right
                        _ => false,
                    }
                } else {
                    false
                };

                let is_mouse_in = if is_hidden_by_edge {
                    is_mouse_near_edge
                } else {
                    point.x >= rect.left
                        && point.x <= rect.right
                        && point.y >= rect.top
                        && point.y <= rect.bottom
                };

                // Ensure window is actually on this monitor
                let window_center_x = (rect.left + rect.right) / 2;
                let window_center_y = (rect.top + rect.bottom) / 2;
                let is_on_current_monitor = window_center_x >= screen_left
                    && window_center_x < screen_right
                    && window_center_y >= screen_top
                    && window_center_y < screen_bottom;

                if !is_hidden_by_edge && !is_on_current_monitor {
                    if IS_HIDDEN.load(Ordering::Relaxed) {
                        IS_HIDDEN.store(false, Ordering::Relaxed);
                        CURRENT_DOCK.store(0, Ordering::Relaxed);
                    }
                    continue;
                }

                let hide_size = 3;

                let mut dock = DockPosition::None;
                if rect.top <= screen_top + threshold {
                    dock = DockPosition::Top;
                } else if rect.left <= screen_left + threshold {
                    dock = DockPosition::Left;
                } else if rect.right >= screen_right - threshold {
                    dock = DockPosition::Right;
                }

                if is_hidden_by_edge {
                    if is_mouse_in {
                        let current_dock = CURRENT_DOCK.load(Ordering::Relaxed);
                        let dock_actual = match current_dock {
                            1 => DockPosition::Top,
                            2 => DockPosition::Left,
                            3 => DockPosition::Right,
                            _ => DockPosition::None,
                        };

                        if dock_actual != DockPosition::None {
                            let _ = window.show();
                            match dock_actual {
                                DockPosition::Top => {
                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition {
                                            x: rect.left,
                                            y: screen_top,
                                        },
                                    ));
                                }
                                DockPosition::Left => {
                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition {
                                            x: screen_left,
                                            y: rect.top,
                                        },
                                    ));
                                }
                                DockPosition::Right => {
                                    let w = rect.right - rect.left;
                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition {
                                            x: screen_right - w,
                                            y: rect.top,
                                        },
                                    ));
                                }
                                _ => {}
                            }

                            IS_HIDDEN.store(false, Ordering::Relaxed);
                            CURRENT_DOCK.store(0, Ordering::Relaxed);
                        }
                    }
                } else if dock != DockPosition::None {
                    // Don't dock while dragging (Left Mouse Button down)
                    let is_lbutton_down = unsafe { (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0 };
                    if is_mouse_in || is_lbutton_down {
                        continue;
                    }

                    if !IS_HIDDEN.load(Ordering::Relaxed) {
                        // Auto-enable pin only when docking occurs (runtime only, no DB write)
                        if !WINDOW_PINNED.load(Ordering::Relaxed) {
                            WINDOW_PINNED.store(true, Ordering::Relaxed);
                            let _ = window.set_always_on_top(true);
                            let _ = window.set_focusable(false);
                            #[cfg(windows)]
                            if let Ok(hwnd) = window.hwnd() {
                                unsafe {
                                    let ex_style =
                                        windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                                            HWND(hwnd.0),
                                            GWL_EXSTYLE,
                                        );
                                    let _ =
                                        windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                                            HWND(hwnd.0),
                                            GWL_EXSTYLE,
                                            ex_style | WS_EX_NOACTIVATE.0 as isize,
                                        );
                                }
                            }
                            let _ = app_handle.emit("window-pinned-changed", true);
                        }

                        let window_height = rect.bottom - rect.top;
                        let window_width = rect.right - rect.left;
                        match dock {
                            DockPosition::Top => {
                                let _ = window.set_position(tauri::PhysicalPosition::new(
                                    rect.left,
                                    screen_top - window_height + hide_size,
                                ));
                                CURRENT_DOCK.store(1, Ordering::Relaxed);
                            }
                            DockPosition::Left => {
                                let _ = window.set_position(tauri::PhysicalPosition::new(
                                    screen_left - window_width + hide_size,
                                    rect.top,
                                ));
                                CURRENT_DOCK.store(2, Ordering::Relaxed);
                            }
                            DockPosition::Right => {
                                let _ = window.set_position(tauri::PhysicalPosition::new(
                                    screen_right - hide_size,
                                    rect.top,
                                ));
                                CURRENT_DOCK.store(3, Ordering::Relaxed);
                            }
                            _ => {}
                        }
                        IS_HIDDEN.store(true, Ordering::Relaxed);
                    }
                } else if IS_HIDDEN.load(Ordering::Relaxed) {
                    IS_HIDDEN.store(false, Ordering::Relaxed);
                    CURRENT_DOCK.store(0, Ordering::Relaxed);

                    // Restore pinned state based on user setting when undocked
                    let mut user_pinned = WINDOW_PINNED.load(Ordering::Relaxed);
                    if let Some(db_state) = app_handle.try_state::<DbState>() {
                        if let Ok(val) = db_state.settings_repo.get("app.window_pinned") {
                            user_pinned = val.as_deref() == Some("true");
                        }
                    }

                    let prev = WINDOW_PINNED.swap(user_pinned, Ordering::Relaxed);
                    if prev != user_pinned {
                        let _ = window.set_always_on_top(user_pinned);
                        let _ = window.set_focusable(!user_pinned);
                        #[cfg(windows)]
                        if let Ok(hwnd) = window.hwnd() {
                            unsafe {
                                let ex_style =
                                    windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                                        HWND(hwnd.0),
                                        GWL_EXSTYLE,
                                    );
                                let next = if user_pinned {
                                    ex_style | WS_EX_NOACTIVATE.0 as isize
                                } else {
                                    ex_style & !(WS_EX_NOACTIVATE.0 as isize)
                                };
                                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                                    HWND(hwnd.0),
                                    GWL_EXSTYLE,
                                    next,
                                );
                            }
                        }
                        let _ = app_handle.emit("window-pinned-changed", user_pinned);
                    }
                }
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn start_edge_docking_monitor(_app_handle: AppHandle) {}

fn init_announcement_ping(app: &App, repo: &impl SettingsRepository) {
    let machine_id = crate::app::system::get_machine_id();
    let stored_anon_id = repo.get("app.anon_id").unwrap_or(None);
    let anon_id = stored_anon_id
        .as_deref()
        .and_then(crate::app::system::normalize_anon_id)
        .unwrap_or_else(|| crate::app::system::build_anon_id(&machine_id));

    if stored_anon_id
        .as_deref()
        .map(|value| value.trim() != anon_id)
        .unwrap_or(true)
    {
        let _ = repo.set("app.anon_id", &anon_id);
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if repo.get("app.last_ping_date").unwrap_or(None).as_deref() != Some(&today) {
        let _ = repo.set("app.last_ping_date", &today);
        let version = app.package_info().version.to_string();
        if let Ok(base_url) = std::env::var("MAGPIE_ANNOUNCEMENT_PING_URL") {
            let base_url = base_url.trim().to_string();
            if !base_url.is_empty() {
                std::thread::spawn(move || {
                    let sep = if base_url.contains('?') { "&" } else { "?" };
                    let ping_url = format!(
                        "{}{}v={}&id={}",
                        base_url,
                        sep,
                        urlencoding::encode(&version),
                        urlencoding::encode(&anon_id)
                    );
                    let _ = reqwest::blocking::get(ping_url);
                });
            }
        }
    }
}

fn setup_tray(app: &App, hide_tray: bool) {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

    let show_i = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>).unwrap();
    let quit_i = MenuItem::with_id(app, "quit", "退出 贴汁", true, None::<&str>).unwrap();
    let menu = Menu::with_items(app, &[&show_i, &quit_i]).unwrap();
    let icon =
        tauri::image::Image::from_bytes(include_bytes!("../../icons/tray-icon.png")).unwrap();

    let tray = TrayIconBuilder::with_id("main_tray")
        .icon(icon)
        .tooltip("Magpie")
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "show" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            } else if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    LAST_SHOW_TIMESTAMP.store(now, Ordering::Relaxed);
                }
            }
        })
        .build(app)
        .expect("Failed to build tray");

    let _ = tray.set_visible(!hide_tray);
    app.manage(tray);
}

fn apply_initial_theme(app: &App) {
    let db_state = app.state::<DbState>();
    // 读取已保存主题，兜底为 ink；并先经 normalize_theme_id 归一旧别名，
    // 使主窗口首次可见前即应用 ink（或迁移后的玻璃主题）效果，首屏无 ink 以外主题的中间态。
    let raw_theme = db_state
        .settings_repo
        .get("app.theme")
        .unwrap_or(Some("ink".to_string()))
        .unwrap_or("ink".to_string());
    let theme = crate::app::commands::ui_cmd::normalize_theme_id(&raw_theme).to_string();
    let mode = db_state
        .settings_repo
        .get("app.color_mode")
        .unwrap_or(Some("system".to_string()));

    if let Some(window) = app.get_webview_window("main") {
        let _ = crate::app::commands::set_theme(
            window,
            app.state::<SettingsState>(),
            db_state,
            theme,
            mode,
            None,
        );
    }
}

#[cfg(target_os = "windows")]
fn init_win32_hooks(_app: &App) {
    std::thread::spawn(move || {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
        };
        unsafe {
            HOOK_THREAD_ID.store(
                windows::Win32::System::Threading::GetCurrentThreadId(),
                Ordering::Relaxed,
            );
            let h_instance = GetModuleHandleW(None).expect("Failed to get module handle");
            let h_hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                Some(HINSTANCE(h_instance.0)),
                0,
            )
            .expect("Failed to set hook");
            HOOK_HANDLE.store(h_hook.0 as _, Ordering::SeqCst);
            let h_mouse_hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_proc),
                Some(HINSTANCE(h_instance.0)),
                0,
            )
            .expect("Failed to set mouse hook");
            HOOK_MOUSE_HANDLE.store(h_mouse_hook.0 as _, Ordering::SeqCst);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = UnhookWindowsHookEx(h_hook);
            let h_mouse = HOOK_MOUSE_HANDLE.swap(null_mut(), Ordering::SeqCst);
            if !h_mouse.is_null() {
                let _ = UnhookWindowsHookEx(windows::Win32::UI::WindowsAndMessaging::HHOOK(
                    h_mouse as _,
                ));
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn setup_taskbar_listener(app: &App) {
    unsafe {
        let msg = RegisterWindowMessageW(windows::core::w!("TaskbarCreated"));
        if msg != 0 {
            TASKBAR_CREATED_MSG.store(msg, Ordering::Relaxed);
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(hwnd) = window.hwnd() {
                    let _ = SetWindowSubclass(HWND(hwnd.0), Some(tray_subclass_proc), 1337, 0);
                }
            }
        }
    }
}

pub fn handle_global_shortcut(app: &AppHandle, shortcut: &tauri_plugin_global_shortcut::Shortcut) {
    use crate::app::commands::hotkey_cmd::{hotkey_scope, is_main_panel_visible, HotkeyScope};
    use tauri_plugin_global_shortcut::Shortcut;
    let settings = app.state::<SettingsState>();

    // BackgroundOnly 作用域的快捷键在主面板可见时不响应（需求 19.7）。
    // 仅在主面板可见时需要判定一次，避免重复查询。
    let panel_visible = is_main_panel_visible(app);
    let ignore_background_only =
        |id: &str| panel_visible && hotkey_scope(app, id) == HotkeyScope::BackgroundOnly;

    if let Ok(main_s) = {
        let val = settings.main_hotkey.lock().unwrap().clone();
        val.replace("Win", "Super").parse::<Shortcut>()
    } {
        if shortcut == &main_s {
            if ignore_background_only("main") {
                return;
            }
            toggle_window(app);
            return;
        }
    }

    if let Ok(seq_s) = {
        let val = settings.sequential_paste_hotkey.lock().unwrap().clone();
        val.replace("Win", "Super").parse::<Shortcut>()
    } {
        if shortcut == &seq_s {
            if ignore_background_only("sequential") {
                return;
            }
            let is_seq = settings.sequential_mode.load(Ordering::Relaxed);
            let has_items = {
                let q_notification = app.state::<PasteQueue>().inner().0.lock().unwrap();
                !q_notification.items.is_empty()
            };
            if is_seq || has_items {
                tauri::async_runtime::spawn({
                    let app = app.clone();
                    async move {
                        crate::services::paste_queue::paste_next_step(app).await;
                    }
                });
            }
        }
    }

    if let Ok(rich_s) = {
        let val = settings.rich_paste_hotkey.lock().unwrap().clone();
        val.replace("Win", "Super").parse::<Shortcut>()
    } {
        if shortcut == &rich_s {
            if ignore_background_only("rich") {
                return;
            }
            crate::services::clipboard_ops::paste_latest_rich(app.clone());
        }
    }

    if let Ok(search_s) = {
        let val = settings.search_hotkey.lock().unwrap().clone();
        val.replace("Win", "Super").parse::<Shortcut>()
    } {
        if shortcut == &search_s {
            if ignore_background_only("search") {
                return;
            }
            toggle_window(app);
            let _ = app.emit("focus-search-input", ());
        }
    }
}

pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    match event {
        tauri::WindowEvent::Focused(focused) => {
            if window.label() != "main" {
                return;
            }
            IS_MAIN_WINDOW_FOCUSED.store(*focused, Ordering::Relaxed);
            if *focused {
                #[cfg(target_os = "windows")]
                unsafe {
                    let hwnd = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
                    if !hwnd.0.is_null() {
                        if let Ok(h) = window.hwnd() {
                            if hwnd.0 != h.0 {
                                crate::LAST_ACTIVE_HWND.store(hwnd.0 as usize, Ordering::Relaxed);
                            }
                        }
                    }
                }
            } else {
                handle_blur(window);
            }
        }
        tauri::WindowEvent::Resized(size) => {
            if window.label() != "main" {
                return;
            }
            if window.is_minimized().unwrap_or(false) || window.is_maximized().unwrap_or(false) {
                return;
            }
            persist_window_size(window, size.width, size.height);
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if window.label() != "main" {
                return;
            }
            api.prevent_close();
            
            // Clear vibrancy to stop GPU rendering
            #[cfg(target_os = "windows")]
            let _ = window_vibrancy::clear_vibrancy(&window);
            
            let _ = window.hide();
            NAVIGATION_ENABLED.store(false, Ordering::SeqCst);
            NAVIGATION_MODE_ACTIVE.store(false, Ordering::SeqCst);
        }
        _ => {}
    }
}

fn persist_window_size(window: &tauri::Window, width: u32, height: u32) {
    if width < 200 || height < 200 {
        return;
    }

    let store = LAST_WINDOW_SIZE.get_or_init(|| Mutex::new((0, 0)));
    {
        let mut guard = store.lock().unwrap();
        *guard = (width, height);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    LAST_WINDOW_SIZE_EVENT_MS.store(now, Ordering::Relaxed);

    if WINDOW_SIZE_SAVE_PENDING.swap(true, Ordering::SeqCst) {
        return;
    }

    let app_handle = window.app_handle().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let last_event = LAST_WINDOW_SIZE_EVENT_MS.load(Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        if now.saturating_sub(last_event) < 200 {
            continue;
        }

        let (w, h) = {
            let guard = LAST_WINDOW_SIZE.get().unwrap().lock().unwrap();
            *guard
        };

        if let Some(db_state) = app_handle.try_state::<DbState>() {
            let _ = db_state
                .settings_repo
                .set("app.window_width", &w.to_string());
            let _ = db_state
                .settings_repo
                .set("app.window_height", &h.to_string());
        }

        WINDOW_SIZE_SAVE_PENDING.store(false, Ordering::SeqCst);
        break;
    });
}

fn handle_blur(window: &tauri::Window) {
    if IGNORE_BLUR.load(Ordering::Relaxed) || WINDOW_PINNED.load(Ordering::Relaxed) {
        return;
    }

    let settings = window.app_handle().state::<SettingsState>();
    if settings.edge_docking.load(Ordering::Relaxed) {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    if now.saturating_sub(LAST_SHOW_TIMESTAMP.load(Ordering::Relaxed)) < 500 {
        return;
    }

    if IS_MOUSE_BUTTON_DOWN.load(Ordering::SeqCst) {
        return;
    }
    unsafe {
        if (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x01) as u16 & 0x8000)
            != 0
            || (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x02) as u16 & 0x8000)
                != 0
        {
            return;
        }
    }

    let w = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let down = IS_MOUSE_BUTTON_DOWN.load(Ordering::SeqCst)
            || unsafe {
                (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x01) as u16
                    & 0x8000)
                    != 0
                    || (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x02) as u16
                        & 0x8000)
                        != 0
            };
        if !down && matches!(w.is_focused(), Ok(false)) {
            if !IGNORE_BLUR.load(Ordering::Relaxed) && !WINDOW_PINNED.load(Ordering::Relaxed) {
                // Clear vibrancy to stop GPU rendering
                #[cfg(target_os = "windows")]
                let _ = window_vibrancy::clear_vibrancy(&w);
                
                let _ = w.hide();
                NAVIGATION_ENABLED.store(false, Ordering::SeqCst);
                release_win_keys();
                let _ = restore_last_focus(w.app_handle().clone());
            }
        }
    });
}

// A10(需求 8.8)：便携模式缺失 data 目录时降级标准模式 — 单元测试
#[cfg(test)]
mod portable_degrade_tests {
    use super::decide_portable_data_dir;
    use std::path::PathBuf;

    /// 模拟一组路径：便携 data 目录、标准模式目录、datapath 候选目录。
    fn paths() -> (PathBuf, PathBuf, PathBuf) {
        let exe_dir = PathBuf::from(r"D:\PortableApps\Magpie");
        let portable_data = exe_dir.join("data");
        let default_app_dir = PathBuf::from(r"C:\Users\tester\AppData\Roaming\app.magpie");
        // datapath 解析后的候选目录（正常情况下即基准目录）
        let current = default_app_dir.clone();
        (portable_data, default_app_dir, current)
    }

    // Requirements 8.8：期望便携但 data/ 目录不存在时，回退路径解析为 %APPDATA%\app.magpie，并标记降级。
    #[test]
    fn 缺_data_目录时降级为标准模式目录() {
        let (portable_data, default_app_dir, current) = paths();
        // portable_data_exists=false（data 被误删），expecting_portable=true（含便携标志）
        let (resolved, used, degraded) = decide_portable_data_dir(
            &current,
            &portable_data,
            &default_app_dir,
            false,
            true,
        );
        assert_eq!(
            resolved, default_app_dir,
            "缺 data 目录且期望便携时应回退到标准模式目录 %APPDATA%\\app.magpie"
        );
        assert!(!used, "降级为标准模式时不应标记为便携模式");
        assert!(degraded, "缺 data 目录且期望便携时应标记为已降级");
    }

    #[test]
    fn data_目录存在时使用便携目录() {
        let (portable_data, default_app_dir, current) = paths();
        // data 存在 → 使用便携目录，不降级
        let (resolved, used, degraded) = decide_portable_data_dir(
            &current,
            &portable_data,
            &default_app_dir,
            true,
            true,
        );
        assert_eq!(resolved, portable_data, "data 目录存在时应使用便携 data 目录");
        assert!(used, "应标记为便携模式");
        assert!(!degraded, "便携模式正常时不应降级");
    }

    #[test]
    fn 标准安装非便携时保持候选目录不变() {
        let (portable_data, default_app_dir, current) = paths();
        // 既无 data 目录，也无便携标志（标准安装版）→ 保持 datapath 候选目录不变，不降级
        let (resolved, used, degraded) = decide_portable_data_dir(
            &current,
            &portable_data,
            &default_app_dir,
            false,
            false,
        );
        assert_eq!(
            resolved,
            current.clone(),
            "标准安装（非便携）时应保持候选数据目录不变"
        );
        assert!(!used, "标准安装不应标记为便携模式");
        assert!(!degraded, "标准安装不应触发便携降级");
    }
}
