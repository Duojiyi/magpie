use std::fs;
use std::path::PathBuf;

/// v0.2.8 Rename Migration: 贴汁 -> TieZ
pub fn perform_migration_v028(default_app_dir: &PathBuf) {
    // Check multiple possible locations for old data folder
    let mut old_app_dirs_to_check = Vec::new();

    // Check parent of current app dir (AppData\Roaming or AppData\Local)
    if let Some(parent) = default_app_dir.parent() {
        old_app_dirs_to_check.push(parent.join("贴汁"));
    }

    // Also check AppData\Local explicitly
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        old_app_dirs_to_check.push(std::path::PathBuf::from(&local_app_data).join("贴汁"));
    }

    // Also check AppData\Roaming explicitly
    if let Ok(roaming_app_data) = std::env::var("APPDATA") {
        old_app_dirs_to_check.push(std::path::PathBuf::from(&roaming_app_data).join("贴汁"));
    }

    // Try each possible location
    for old_app_dir in old_app_dirs_to_check {
        if old_app_dir.exists() && old_app_dir.is_dir() {
            println!(
                ">>> [MIGRATION] Found old data folder at: {:?}",
                old_app_dir
            );
            let new_db = default_app_dir.join("clipboard.db");
            let old_db = old_app_dir.join("clipboard.db");

            let mut success = false;

            // 1. Check for custom data path redirect (datapath.txt)
            let old_redirect = old_app_dir.join("datapath.txt");
            let new_redirect = default_app_dir.join("datapath.txt");

            if old_redirect.exists() {
                println!(">>> [MIGRATION] Found custom data path configuration. Migrating...");
                let _ = std::fs::create_dir_all(&default_app_dir);
                if std::fs::copy(&old_redirect, &new_redirect).is_ok() {
                    success = true;
                    println!(">>> [MIGRATION] Migrated datapath.txt successfully.");
                }
            }

            // 2. Data Migration Logic
            if !default_app_dir.exists() && !success {
                println!(">>> [MIGRATION] Renaming old data folder '贴汁' to 'TieZ'...");
                success = std::fs::rename(&old_app_dir, &default_app_dir).is_ok();
            } else if old_db.exists() && !new_db.exists() {
                println!(">>> [MIGRATION] Pulling old data from '贴汁' to 'TieZ'...");
                let _ = std::fs::create_dir_all(&default_app_dir);
                if std::fs::copy(&old_db, &new_db).is_ok() {
                    success = true;
                    let old_log = old_app_dir.join("tiez.log");
                    if old_log.exists() {
                        let _ = std::fs::copy(&old_log, default_app_dir.join("tiez.log"));
                    }
                }
            } else if old_db.exists() && new_db.exists() {
                let old_size = std::fs::metadata(&old_db).map(|m| m.len()).unwrap_or(0);
                let new_size = std::fs::metadata(&new_db).map(|m| m.len()).unwrap_or(0);

                if old_size > new_size && new_size < 50_000 {
                    println!(">>> [MIGRATION] Old database ({} bytes) has more data than new ({} bytes). Replacing...", old_size, new_size);
                    let backup_db = default_app_dir.join("clipboard.db.backup");
                    let _ = std::fs::rename(&new_db, &backup_db);

                    if std::fs::copy(&old_db, &new_db).is_ok() {
                        success = true;
                        println!(">>> [MIGRATION] Successfully migrated old database to TieZ.");
                        let old_redirect = old_app_dir.join("datapath.txt");
                        if old_redirect.exists() {
                            let _ =
                                std::fs::copy(&old_redirect, default_app_dir.join("datapath.txt"));
                        }
                    } else {
                        let _ = std::fs::rename(&backup_db, &new_db);
                    }
                } else {
                    success = true;
                }
            } else {
                success = true;
            }

            if success {
                println!(">>> [CLEANUP] Cleaning up residues of old '贴汁' version...");
                if old_app_dir.exists() {
                    let _ = std::fs::remove_dir_all(&old_app_dir);
                }
                let custom_path = cleanup_old_install_registry();
                cleanup_old_start_menu();
                cleanup_old_install_folder(custom_path);
            }
        }
    }

    // Always try to clean up old version residues on every startup
    let custom_path = cleanup_old_install_registry();
    cleanup_old_start_menu();
    cleanup_old_install_folder(custom_path);
}

/// v0.2.8 Rename Migration: Registry Cleanup - Returns found install location if any
pub fn cleanup_old_install_registry() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
        let mut found_install_loc = None;

        println!(
            ">>> [CLEANUP] Scanning registry for old versions at HKCU\\{}",
            path
        );

        if let Ok(key) = hkcu.open_subkey_with_flags(path, KEY_READ | KEY_WRITE) {
            for subkey_name in key.enum_keys().filter_map(|x| x.ok()) {
                if let Ok(subkey) = key.open_subkey(&subkey_name) {
                    let name: String = subkey.get_value("DisplayName").unwrap_or_default();

                    if name.contains("贴汁") {
                        println!(
                            ">>> [CLEANUP] Found old registry entry: {} ({}).",
                            subkey_name, name
                        );

                        // Try to get InstallLocation
                        if let Ok(loc) = subkey.get_value::<String, _>("InstallLocation") {
                            if !loc.is_empty() {
                                println!(
                                    ">>> [CLEANUP] Found InstallLocation in registry: {}",
                                    loc
                                );
                                found_install_loc = Some(PathBuf::from(loc));
                            }
                        }
                        // Fallback: Try to parse from UninstallString "C:\path\to\uninstall.exe"
                        if found_install_loc.is_none() {
                            if let Ok(uninstall_str) =
                                subkey.get_value::<String, _>("UninstallString")
                            {
                                println!(">>> [CLEANUP] Found UninstallString: {}", uninstall_str);
                                // Simple heuristic: remove quotes and find parent of executable
                                let clean_str = uninstall_str.replace("\"", "");
                                let p = std::path::Path::new(&clean_str);
                                if let Some(parent) = p.parent() {
                                    println!(">>> [CLEANUP] inferred install path from uninstaller: {:?}", parent);
                                    found_install_loc = Some(parent.to_path_buf());
                                }
                            }
                        }

                        println!(">>> [CLEANUP] Deleting registry key...");
                        if let Err(e) = key.delete_subkey_all(&subkey_name) {
                            println!(">>> [CLEANUP ERROR] Failed to delete registry key: {}", e);
                        } else {
                            println!(">>> [CLEANUP] Registry entry deleted.");
                        }
                    }
                }
            }
        }
        return found_install_loc;
    }
    #[cfg(not(windows))]
    None
}

/// v0.2.8 Rename Migration: Start Menu & Desktop Cleanup
pub fn cleanup_old_start_menu() {
    #[cfg(windows)]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let start_menu =
                std::path::Path::new(&app_data).join("Microsoft\\Windows\\Start Menu\\Programs");
            println!(">>> [CLEANUP] Checking Start Menu at: {:?}", start_menu);

            // Delete old shortcut
            let old_lnk = start_menu.join("贴汁.lnk");
            if old_lnk.exists() {
                println!(
                    ">>> [CLEANUP] Deleting old start menu shortcut: {:?}",
                    old_lnk
                );
                let _ = fs::remove_file(old_lnk);
            }

            // Delete old start menu folder
            let old_folder = start_menu.join("贴汁");
            if old_folder.exists() && old_folder.is_dir() {
                println!(
                    ">>> [CLEANUP] Deleting old start menu folder: {:?}",
                    old_folder
                );
                let _ = fs::remove_dir_all(old_folder);
            }
        }

        // Desktop Cleanup
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let desktop = std::path::Path::new(&user_profile).join("Desktop");
            let old_desktop_lnk = desktop.join("贴汁.lnk");
            println!(
                ">>> [CLEANUP] Checking Desktop shortcut at: {:?}",
                old_desktop_lnk
            );
            if old_desktop_lnk.exists() {
                println!(
                    ">>> [CLEANUP] Deleting old desktop shortcut: {:?}",
                    old_desktop_lnk
                );
                let _ = fs::remove_file(old_desktop_lnk);
            }
        }
    }
}

/// v0.2.8 Rename Migration: Clean up old installation directory
pub fn cleanup_old_install_folder(custom_path: Option<PathBuf>) {
    #[cfg(windows)]
    {
        // Try to find and delete old installation folder
        // Common installation paths
        let mut possible_paths = vec![
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("Programs").join("贴汁")),
            std::env::var("ProgramFiles")
                .ok()
                .map(|p| PathBuf::from(p).join("贴汁")),
            std::env::var("ProgramFiles(x86)")
                .ok()
                .map(|p| PathBuf::from(p).join("贴汁")),
            // Also check direct local appdata just in case
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("贴汁")),
        ];

        // Add custom path from registry if found
        if let Some(path) = custom_path {
            println!(
                ">>> [CLEANUP] Adding custom path from registry to cleanup list: {:?}",
                path
            );
            possible_paths.push(Some(path));
        }

        for path_opt in possible_paths.iter() {
            if let Some(path) = path_opt {
                println!(">>> [CLEANUP] Checking installation path: {:?}", path);
                if path.exists() && path.is_dir() {
                    // Safety check: Don't delete if it's the current running dir (unlikely due to rename, but good practice)
                    if let Ok(current_exe) = std::env::current_exe() {
                        if let Some(current_dir) = current_exe.parent() {
                            if path == current_dir {
                                println!(
                                    ">>> [CLEANUP] Skipping current directory safety check: {:?}",
                                    path
                                );
                                continue;
                            }
                        }
                    }

                    println!(">>> [CLEANUP] Found old installation folder: {:?}", path);
                    // Try to delete - this might fail if files are in use
                    match fs::remove_dir_all(path) {
                        Ok(_) => {
                            println!(">>> [CLEANUP] Successfully deleted old installation folder")
                        }
                        Err(e) => println!(
                            ">>> [CLEANUP] Could not delete old installation folder: {}",
                            e
                        ),
                    }
                }
            }
        }
    }
}

/// 迁移结果，供 `resolve_data_dir` 决定本次启动使用哪个数据目录。
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationOutcome {
    /// 无需迁移、迁移成功、或目标目录已可用——使用 `default_app_dir`（app.magpie）。
    UseTarget,
    /// 迁移失败已回滚降级，本次启动改用 legacy 目录（com.tiez），且未写完成标记以便下次重试。
    DegradedToLegacy(PathBuf),
}

/// v0.4.0 Rename Migration: com.tiez -> app.magpie
///
/// 0.4.0 版本将 Tauri identifier 从 `com.tiez` 改为 `app.magpie`，默认数据目录
/// `%APPDATA%\com.tiez` 变成了 `%APPDATA%\app.magpie`。迁移采用「临时目录 + 同卷原子
/// 重命名」方案，保证迁移要么完整成功、要么完全回滚，绝不在目标目录留下半成品。
///
/// 行为说明：
/// - 在 `%APPDATA%`（与目标同卷）下建 `app.magpie.tmp`，复制并校验完整后再 `rename` 为
///   `app.magpie`，确保同卷原子重命名。
/// - 开始迁移前先删除任何已存在的 `app.magpie.tmp` 残留（6.2）。
/// - 半迁移检测（6.1）：目标存在 clipboard.db 但缺少关键配置（settings 表），或存在 tmp
///   残留，则视为上次未完成的迁移，本次重试（6.7）。
/// - 成功后写 `migration_v040.done` 幂等标记，重复启动不再迁移（6.6）。
/// - 任一步失败：删除不完整的 tmp、保持 `com.tiez` 不被修改、本次降级使用 legacy、且
///   **不写** 完成标记，以便下次启动重试（6.5、6.7）。
pub fn perform_migration_v040(default_app_dir: &PathBuf) -> MigrationOutcome {
    let marker = default_app_dir.join("migration_v040.done");

    // 幂等：已完成迁移则直接使用目标目录（6.6）
    if marker.exists() {
        return MigrationOutcome::UseTarget;
    }

    // tmp 残留目录与目标目录同父（同卷），保证后续为同卷原子重命名（6.2）
    let tmp_dir = tmp_dir_for(default_app_dir);

    // 半迁移检测（6.1）：决定是否需要无视已填充内容强制重试
    let half = is_half_migrated(default_app_dir, &tmp_dir);

    let old_dir = match find_legacy_dir(default_app_dir) {
        Some(p) => p,
        None => {
            // 没有旧数据可迁移：清掉任何 tmp 残留，写 marker 防止以后反复扫
            let _ = std::fs::remove_dir_all(&tmp_dir);
            let _ = std::fs::create_dir_all(default_app_dir);
            let _ = std::fs::write(&marker, b"no-old-data");
            return MigrationOutcome::UseTarget;
        }
    };

    // 目标目录已被正常数据填充（用户在 0.4.0 已积累新数据）且非半迁移 → 不覆盖，标记完成。
    // 若处于半迁移（db 缺关键配置或残留 tmp），则忽略此快捷路径继续重试迁移（6.7）。
    let new_db = default_app_dir.join("clipboard.db");
    if new_db.exists() && !half {
        if let Ok(meta) = std::fs::metadata(&new_db) {
            if meta.len() > 50_000 {
                println!(">>> [MIGRATION v040] new data dir already populated, skip copy");
                let _ = std::fs::remove_dir_all(&tmp_dir);
                let _ = std::fs::write(&marker, b"new-already-populated");
                return MigrationOutcome::UseTarget;
            }
        }
    }

    // === 开始迁移 ===
    // 核心迁移（复制到 tmp → 校验 → 同卷原子重命名 → 写幂等标记）。生产路径恒不注入失败，
    // 失败注入仅供 #[cfg(test)] 验证回滚/降级不变量。
    run_atomic_migration(
        default_app_dir,
        &old_dir,
        &marker,
        &tmp_dir,
        MigrationFault::None,
    )
}

/// 迁移失败注入点。生产调用恒为 `MigrationFault::None`；`ForceFail` 仅供属性测试在「复制到
/// tmp 完成后」强制触发失败，验证回滚/降级不变量（删 tmp、保留 legacy、不写 done）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationFault {
    None,
    ForceFail,
}

/// 核心原子迁移：从 `old_dir` 迁移到 `default_app_dir`（用同父 `tmp_dir` 中转），成功写 `marker`。
///
/// 不读取任何环境变量、仅依据传入路径操作，便于用临时目录可重复测试（设计：纯逻辑/副作用分离）。
/// 任一步失败：删除不完整的 tmp、保持 `old_dir` 不被修改、不写 `marker`，返回降级到 legacy。
fn run_atomic_migration(
    default_app_dir: &PathBuf,
    old_dir: &PathBuf,
    marker: &PathBuf,
    tmp_dir: &PathBuf,
    fault: MigrationFault,
) -> MigrationOutcome {
    // 迁移起始：写入可见迁移日志（A3，2.1/2.2）。db_size 取源目录数据库大小。
    append_migration_log(
        default_app_dir,
        &format!(
            "[MIGRATION v040] from={} to={} db_size={}",
            old_dir.display(),
            default_app_dir.display(),
            db_size_of(old_dir)
        ),
    );

    // 1. 删除任何已存在的 tmp 残留（6.2）
    if tmp_dir.exists() {
        if std::fs::remove_dir_all(tmp_dir).is_err() {
            println!(">>> [MIGRATION v040] failed to clear tmp residue, degrade to legacy");
            append_migration_log(
                default_app_dir,
                "[MIGRATION v040] FAILED clear tmp residue, degrade to legacy",
            );
            return MigrationOutcome::DegradedToLegacy(old_dir.clone());
        }
    }

    // 2. 复制到 tmp（6.4）
    if std::fs::create_dir_all(tmp_dir)
        .and_then(|_| copy_dir_contents(old_dir, tmp_dir))
        .is_err()
    {
        println!(">>> [MIGRATION v040] copy to tmp failed, degrade to legacy");
        append_migration_log(
            default_app_dir,
            "[MIGRATION v040] FAILED copy to tmp, degrade to legacy",
        );
        rollback_tmp(tmp_dir);
        return MigrationOutcome::DegradedToLegacy(old_dir.clone());
    }

    // 2.5 测试失败注入：模拟「校验/原子提升阶段失败」，触发与真实失败一致的回滚降级路径。
    if fault == MigrationFault::ForceFail {
        append_migration_log(
            default_app_dir,
            "[MIGRATION v040] FAILED injected fault, degrade to legacy",
        );
        rollback_tmp(tmp_dir);
        return MigrationOutcome::DegradedToLegacy(old_dir.clone());
    }

    // 3. 校验完整性（6.4）
    if !verify_copy(old_dir, tmp_dir) {
        println!(">>> [MIGRATION v040] verify failed, degrade to legacy");
        append_migration_log(
            default_app_dir,
            "[MIGRATION v040] FAILED verify copy (db size mismatch), degrade to legacy",
        );
        rollback_tmp(tmp_dir);
        return MigrationOutcome::DegradedToLegacy(old_dir.clone());
    }

    // 4. 原子重命名 tmp → 目标（6.4）
    if atomic_promote(tmp_dir, default_app_dir).is_err() {
        println!(">>> [MIGRATION v040] atomic rename failed, degrade to legacy");
        append_migration_log(
            default_app_dir,
            "[MIGRATION v040] FAILED atomic rename, degrade to legacy",
        );
        rollback_tmp(tmp_dir);
        return MigrationOutcome::DegradedToLegacy(old_dir.clone());
    }

    // 5. 成功 → 写幂等标记（6.6）。com.tiez 始终保留不动作为安全网（6.5）。
    let _ = std::fs::write(marker, b"migrated-from-com.tiez");
    append_migration_log(
        default_app_dir,
        &format!(
            "[MIGRATION v040] from={} to={} db_size={} OK",
            old_dir.display(),
            default_app_dir.display(),
            db_size_of(default_app_dir)
        ),
    );
    println!(">>> [MIGRATION v040] migration complete");
    MigrationOutcome::UseTarget
}

/// 检测目标目录是否处于「半迁移状态」（6.1）：
/// - 存在 `app.magpie.tmp` 残留目录；或
/// - 存在 clipboard.db 但缺少关键配置（DB 打不开或不含 settings 表）。
/// 其余状态返回 false。
pub fn is_half_migrated(target_dir: &PathBuf, tmp_dir: &PathBuf) -> bool {
    if tmp_dir.exists() {
        return true;
    }
    let db = target_dir.join("clipboard.db");
    if !db.exists() {
        return false;
    }
    // db 存在但缺关键配置（settings 表）视为半迁移
    !db_has_settings_table(&db)
}

/// 以只读方式打开 DB，检查是否包含 settings 表（关键配置）。打不开亦视为缺失。
fn db_has_settings_table(db_path: &PathBuf) -> bool {
    match rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(conn) => conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings'",
                [],
                |_| Ok(()),
            )
            .is_ok(),
        Err(_) => false,
    }
}

/// 查找 com.tiez 旧数据目录（Roaming 优先，与 Tauri `app_data_dir()` 行为对齐）。
fn find_legacy_dir(default_app_dir: &PathBuf) -> Option<PathBuf> {
    let mut old_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(roaming) = std::env::var("APPDATA") {
        old_dirs.push(PathBuf::from(&roaming).join("com.tiez"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        old_dirs.push(PathBuf::from(&local).join("com.tiez"));
    }
    if let Some(parent) = default_app_dir.parent() {
        old_dirs.push(parent.join("com.tiez"));
    }
    old_dirs.into_iter().find(|p| p.exists() && p.is_dir())
}

/// 校验复制完整性（6.4）：源目录的 clipboard.db 若存在，tmp 中必须存在且大小一致。
fn verify_copy(src: &PathBuf, tmp: &PathBuf) -> bool {
    let src_db = src.join("clipboard.db");
    if !src_db.exists() {
        // 源无数据库，无需严格校验
        return true;
    }
    let tmp_db = tmp.join("clipboard.db");
    if !tmp_db.exists() {
        return false;
    }
    let src_size = std::fs::metadata(&src_db).map(|m| m.len()).unwrap_or(0);
    let tmp_size = std::fs::metadata(&tmp_db).map(|m| m.len()).unwrap_or(u64::MAX);
    src_size == tmp_size
}

/// 原子提升：把 tmp 重命名为目标目录（6.4）。
/// 若目标已存在（半迁移残留的不完整目录），先将其移到 `.old` 备份腾出位置，
/// 重命名成功后删除备份；失败则把备份搬回原位回滚。
fn atomic_promote(tmp_dir: &PathBuf, target: &PathBuf) -> std::io::Result<()> {
    if target.exists() {
        let backup = backup_dir_for(target);
        let _ = std::fs::remove_dir_all(&backup);
        std::fs::rename(target, &backup)?;
        match std::fs::rename(tmp_dir, target) {
            Ok(_) => {
                let _ = std::fs::remove_dir_all(&backup);
                Ok(())
            }
            Err(e) => {
                // 回滚：旧目录搬回原位
                let _ = std::fs::rename(&backup, target);
                Err(e)
            }
        }
    } else {
        std::fs::rename(tmp_dir, target)
    }
}

/// 回滚：删除不完整的 tmp 目录（6.5）。com.tiez 不受影响。
fn rollback_tmp(tmp_dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(tmp_dir);
}

/// 以追加方式向目标数据目录的 `tiez.log` 写入一行可见迁移日志（A3，2.1/2.2）。
///
/// 迁移发生在 `resolve_data_dir` 内部、logger 尚未初始化的阶段，因此这里直接用
/// `OpenOptions::append` 写目标目录的 `tiez.log`（沿用兼容字段文件名，仅追加不覆盖，
/// 与后续 logger 输出同文件、互不冲突）。写入前确保目标目录存在。
fn append_migration_log(target_dir: &PathBuf, line: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(target_dir);
    let log_path = target_dir.join("tiez.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(f, "{}", line);
    }
}

/// 读取指定目录下 clipboard.db 的字节大小，读不到则返回 0（用于迁移日志的 db_size 字段）。
fn db_size_of(dir: &PathBuf) -> u64 {
    std::fs::metadata(dir.join("clipboard.db"))
        .map(|m| m.len())
        .unwrap_or(0)
}

/// 计算与目标目录同父（同卷）的临时迁移目录路径：`<parent>/app.magpie.tmp`。
fn tmp_dir_for(target: &PathBuf) -> PathBuf {
    sibling_with_suffix(target, "tmp")
}

/// 计算与目标目录同父的旧目录备份路径：`<parent>/app.magpie.old`。
fn backup_dir_for(target: &PathBuf) -> PathBuf {
    sibling_with_suffix(target, "old")
}

/// 在目标目录名后追加后缀生成同父兄弟目录路径。
fn sibling_with_suffix(target: &PathBuf, suffix: &str) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app.magpie".to_string());
    let sibling_name = format!("{}.{}", name, suffix);
    match target.parent() {
        Some(parent) => parent.join(sibling_name),
        None => PathBuf::from(sibling_name),
    }
}

/// 递归拷贝目录内容（不删除源），跳过已存在的同名文件
fn copy_dir_contents(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let name = entry.file_name();
        let to = dst.join(&name);

        if from.is_dir() {
            std::fs::create_dir_all(&to)?;
            copy_dir_contents(&from, &to)?;
        } else if !to.exists() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

// Feature: magpie-v0-4-1, Property 6: 迁移原子性、幂等性与失败回滚
#[cfg(test)]
mod property_6_migration_atomic {
    use super::{
        perform_migration_v040, run_atomic_migration, tmp_dir_for, MigrationFault,
        MigrationOutcome,
    };
    use proptest::prelude::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// 为单次 proptest 用例创建唯一的临时根目录，避免跨迭代/跨并行测试相互污染。
    fn unique_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "magpie_mig6_{}_{}_{}",
            std::process::id(),
            nanos,
            seq
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// 将目录下的「普通文件」收集为 (文件名 -> 内容字节) 的有序映射，用于内容比较。
    /// 仅收集顶层文件（生成器只产出扁平文件树）。
    fn snapshot_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_file() {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    let bytes = std::fs::read(&p).unwrap_or_default();
                    out.push((name, bytes));
                }
            }
        }
        out.sort();
        out
    }

    /// 生成扁平文件树：文件名为 `f<数字>`（避免与 clipboard.db / tiez.log 冲突），内容为任意字节。
    fn file_tree() -> impl Strategy<Value = Vec<(String, Vec<u8>)>> {
        proptest::collection::vec(
            (
                "f[0-9]{1,8}".prop_map(|s| s),
                proptest::collection::vec(any::<u8>(), 0..64),
            ),
            0..6,
        )
        .prop_map(|mut v| {
            // 文件名去重，保证写入的文件数确定
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v.dedup_by(|a, b| a.0 == b.0);
            v
        })
    }

    /// 在 legacy 目录写入生成的文件树 + 一个 clipboard.db（内容用于校验大小一致）。
    fn populate_legacy(legacy: &Path, tree: &[(String, Vec<u8>)], db_bytes: &[u8]) {
        std::fs::create_dir_all(legacy).unwrap();
        for (name, content) in tree {
            std::fs::write(legacy.join(name), content).unwrap();
        }
        std::fs::write(legacy.join("clipboard.db"), db_bytes).unwrap();
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(120))]
        #[test]
        fn 迁移原子性_幂等性与失败回滚(
            tree in file_tree(),
            db_bytes in proptest::collection::vec(any::<u8>(), 1..256),
            inject_fail in any::<bool>(),
        ) {
            let root = unique_root();
            let legacy = root.join("com.tiez");
            let target = root.join("app.magpie");
            let marker = target.join("migration_v040.done");
            let tmp = tmp_dir_for(&target);

            populate_legacy(&legacy, &tree, &db_bytes);
            let legacy_before = snapshot_files(&legacy);

            let fault = if inject_fail {
                MigrationFault::ForceFail
            } else {
                MigrationFault::None
            };

            let outcome = run_atomic_migration(&target, &legacy, &marker, &tmp, fault);

            if inject_fail {
                // 失败：降级到 legacy；不写完成标记；删除不完整 tmp；legacy 内容完全不变。
                prop_assert_eq!(
                    &outcome,
                    &MigrationOutcome::DegradedToLegacy(legacy.clone()),
                    "失败注入时应降级到 legacy"
                );
                prop_assert!(!marker.exists(), "失败时不得写入完成标记（以便下次重试）");
                prop_assert!(!tmp.exists(), "失败时不得残留不完整的 app.magpie.tmp");
                prop_assert_eq!(
                    snapshot_files(&legacy),
                    legacy_before,
                    "失败时 legacy（com.tiez）内容必须保持不被修改"
                );
            } else {
                // 成功：使用目标目录；写入完成标记；tmp 已清理；目标包含全部源内容且字节一致。
                prop_assert_eq!(&outcome, &MigrationOutcome::UseTarget, "成功时应使用目标目录");
                prop_assert!(marker.exists(), "成功时应写入幂等完成标记");
                prop_assert!(!tmp.exists(), "成功后临时目录应被原子重命名消费，不残留");

                // 源目录的每个文件都应在目标目录中存在且内容一致（含 clipboard.db）。
                for (name, content) in &tree {
                    let dst = target.join(name);
                    prop_assert!(dst.is_file(), "目标缺少源文件 {}", name);
                    prop_assert_eq!(
                        &std::fs::read(&dst).unwrap(),
                        content,
                        "目标文件 {} 内容应与源一致",
                        name
                    );
                }
                prop_assert_eq!(
                    std::fs::read(target.join("clipboard.db")).unwrap(),
                    db_bytes.clone(),
                    "目标 clipboard.db 内容应与源一致"
                );
                // legacy 始终保留不动作为安全网。
                prop_assert_eq!(
                    snapshot_files(&legacy),
                    legacy_before,
                    "成功时 legacy（com.tiez）仍应原样保留"
                );

                // 幂等：已写 marker 后再次调用 perform_migration_v040 应短路返回 UseTarget，
                // 且目标内容不发生任何变化（marker 短路在读取环境变量之前，不依赖外部环境）。
                let target_after_success = snapshot_files(&target);
                let again = perform_migration_v040(&target);
                prop_assert_eq!(again, MigrationOutcome::UseTarget, "已完成迁移后再次执行应幂等返回 UseTarget");
                prop_assert_eq!(
                    snapshot_files(&target),
                    target_after_success,
                    "再次执行迁移不应改变已迁移目标目录内容（幂等）"
                );
            }

            let _ = std::fs::remove_dir_all(&root);
        }
    }
}

// Feature: magpie-v0-4-1, Property 7: 半迁移状态检测
#[cfg(test)]
mod property_7_half_migration {
    use super::{is_half_migrated, tmp_dir_for};
    use proptest::prelude::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn unique_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "magpie_mig7_{}_{}_{}",
            std::process::id(),
            nanos,
            seq
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// 创建一个含/不含 settings 表的 SQLite 数据库文件，模拟「关键配置」存在与缺失。
    fn make_db(path: &std::path::Path, with_settings: bool) {
        let conn = rusqlite::Connection::open(path).unwrap();
        // 始终建一张无关表，确保是个合法可打开的 DB
        conn.execute_batch("CREATE TABLE clipboard_history(id INTEGER PRIMARY KEY);")
            .unwrap();
        if with_settings {
            conn.execute_batch(
                "CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT);",
            )
            .unwrap();
        }
        // 关闭连接，释放文件句柄
        drop(conn);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(120))]
        #[test]
        fn 半迁移状态检测(
            tmp_present in any::<bool>(),
            db_present in any::<bool>(),
            db_has_settings in any::<bool>(),
        ) {
            let root = unique_root();
            let target = root.join("app.magpie");
            std::fs::create_dir_all(&target).unwrap();
            let tmp = tmp_dir_for(&target);

            if tmp_present {
                std::fs::create_dir_all(&tmp).unwrap();
            }
            if db_present {
                make_db(&target.join("clipboard.db"), db_has_settings);
            }

            // 期望：存在 tmp 残留，或（存在 db 但缺 settings 关键配置）→ 半迁移；其余为否。
            let expected = tmp_present || (db_present && !db_has_settings);
            let actual = is_half_migrated(&target, &tmp);

            prop_assert_eq!(
                actual,
                expected,
                "半迁移检测真值表不符：tmp={} db={} settings={}",
                tmp_present,
                db_present,
                db_has_settings
            );

            let _ = std::fs::remove_dir_all(&root);
        }
    }
}

#[cfg(test)]
mod migration_log_and_tmp_tests {
    use super::{run_atomic_migration, tmp_dir_for, MigrationFault};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn unique_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "magpie_migunit_{}_{}_{}",
            std::process::id(),
            nanos,
            seq
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    // Requirements 2.1, 2.2：迁移日志行应包含 [MIGRATION v040] / from / to / db_size 字段。
    #[test]
    fn 迁移日志包含必需字段() {
        let root = unique_root();
        let legacy = root.join("com.tiez");
        let target = root.join("app.magpie");
        let marker = target.join("migration_v040.done");
        let tmp = tmp_dir_for(&target);

        std::fs::create_dir_all(&legacy).unwrap();
        // 写一个已知大小的 clipboard.db，便于校验 db_size 出现在日志中
        let db_bytes = vec![0u8; 123];
        std::fs::write(legacy.join("clipboard.db"), &db_bytes).unwrap();

        let outcome = run_atomic_migration(&target, &legacy, &marker, &tmp, MigrationFault::None);
        assert!(matches!(
            outcome,
            super::MigrationOutcome::UseTarget
        ));

        let log = std::fs::read_to_string(target.join("tiez.log")).expect("应写入 tiez.log");
        assert!(log.contains("[MIGRATION v040]"), "日志应含前缀 [MIGRATION v040]：{}", log);
        assert!(log.contains("from="), "日志应含 from 字段");
        assert!(log.contains("to="), "日志应含 to 字段");
        assert!(log.contains("db_size="), "日志应含 db_size 字段");
        // db_size 应为源数据库实际大小
        assert!(log.contains("db_size=123"), "db_size 应为源数据库字节数 123：{}", log);

        let _ = std::fs::remove_dir_all(&root);
    }

    // Requirements 6.2：tmp 建在 %APPDATA% 同卷（与目标同父目录），保证后续为同卷原子重命名。
    #[test]
    fn tmp_目录与目标同父同卷() {
        // 路径按平台构造：`\` 在 Unix 上是普通字符，硬编码 Windows 字面量会让整串被当成
        // 单个文件名，file_name() 取不到 "app.magpie"，断言随之失败。
        let target: PathBuf = if cfg!(windows) {
            PathBuf::from(r"C:\Users\tester\AppData\Roaming\app.magpie")
        } else {
            PathBuf::from("/home/tester/.local/share/app.magpie")
        };
        let tmp = tmp_dir_for(&target);
        assert_eq!(
            tmp.parent(),
            target.parent(),
            "tmp 目录必须与目标目录同父（同卷），以保证 fs::rename 为同卷原子重命名"
        );
        assert_eq!(
            tmp.file_name().unwrap().to_string_lossy(),
            "app.magpie.tmp",
            "tmp 目录名应为 <目标名>.tmp"
        );
    }
}
