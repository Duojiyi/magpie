//! 诊断信息收集命令（A9 / 需求 7）
//!
//! 提供 `copy_diagnostics` 命令：收集 `tiez.log` 末尾若干行、系统信息与活跃设置摘要，
//! 经脱敏后返回字符串供前端写入剪贴板。全程不做任何网络上传。

use crate::app_state::AppDataDir;
use crate::database::{is_sensitive_key, DbState};
use crate::error::{AppError, AppResult};
use crate::infrastructure::repository::settings_repo::SettingsRepository;
use regex::Regex;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, State};

/// 诊断信息中保留的日志行数（tiez.log 末尾）
const LOG_TAIL_LINES: usize = 200;

/// 对诊断文本脱敏：掩码 password / token / secret 等敏感字段的值，以及 URL 的 query string。
///
/// 纯函数：无副作用、无网络调用，便于后续属性测试覆盖。
fn redact(input: &str) -> String {
    // 敏感键值对：`key<分隔符>value` → `key<分隔符>***`
    // 覆盖 `key=value`、`key: value`、JSON 风格 `"key":"value"` 等常见形态
    static KV_RE: OnceLock<Regex> = OnceLock::new();
    let kv_re = KV_RE.get_or_init(|| {
        Regex::new(
            // 值部分匹配两种形态：
            // 1. 双引号包裹的字符串（可含空格，匹配到闭合引号前），覆盖 `key="my secret value"`；
            // 2. 无引号 token（遇空白/分隔符/结构字符即止），覆盖 `key=value`。
            // 分隔符组保留尾随可选开引号 `"?`，故引号形态下值组以引号内首字符起算。
            r#"(?i)(password|passwd|pwd|token|secret|api[_-]?key)(\s*[:=]\s*"|"\s*:\s*"|\s*[:=]\s*)([^"\r\n]*"|[^\s,;"}\]]+)"#,
        )
        .expect("脱敏键值正则应当合法")
    });
    let masked_kv = kv_re.replace_all(input, "${1}${2}***");

    // URL query string：`scheme://host/path?query` → `scheme://host/path?<redacted>`
    static URL_RE: OnceLock<Regex> = OnceLock::new();
    let url_re = URL_RE
        .get_or_init(|| Regex::new(r#"(https?://[^\s?]+)\?[^\s]*"#).expect("URL 脱敏正则应当合法"));
    let masked = url_re.replace_all(&masked_kv, "${1}?<redacted>");

    masked.into_owned()
}

/// 取文本最后 `n` 行
fn tail_lines(content: &str, n: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// 组装诊断报告文本（纯函数，无任何 IO / 网络）。
///
/// 抽离自 `copy_diagnostics` 的报告拼装逻辑，便于单元测试验证「含日志末 N 行、系统信息、
/// 设置摘要」的组成，且因不触达任何 IO 而天然「无网络调用」。返回未脱敏的原始报告，
/// 脱敏在调用方统一通过 `redact` 兜底。
fn build_diagnostics_report(
    app_version: &str,
    os_version: &str,
    run_mode: &str,
    data_path: &str,
    settings_summary: &str,
    log_tail: &str,
    tail_n: usize,
) -> String {
    format!(
        "==== Magpie 诊断信息 ====\n\
         应用版本: {app_version}\n\
         系统版本: {os_version}\n\
         运行模式: {run_mode}\n\
         数据路径: {data_path}\n\
         \n\
         ==== 活跃设置摘要 ====\n\
         {settings_summary}\n\
         ==== 日志（tiez.log 末 {tail_n} 行）====\n\
         {log_tail}\n"
    )
}

/// 当前是否以便携模式运行：exe 同目录存在 data 文件夹（与 setup::resolve_data_dir 判定一致）
fn is_portable_mode() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("data")))
        .map(|data| data.is_dir())
        .unwrap_or(false)
}

/// 当前操作系统版本描述
fn os_version_string() -> String {
    #[cfg(target_os = "windows")]
    {
        let v = windows_version::OsVersion::current();
        format!("Windows {}.{}.{}", v.major, v.minor, v.build)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::consts::OS.to_string()
    }
}

/// 收集诊断信息并返回脱敏后的文本（不做任何网络上传）
#[tauri::command]
pub fn copy_diagnostics(app: AppHandle, state: State<'_, AppDataDir>) -> AppResult<String> {
    let data_dir = state.0.lock().unwrap().clone();

    // 1) 系统信息：应用版本 / 系统版本 / 是否便携 / 数据路径
    let app_version = app.package_info().version.to_string();
    let os_version = os_version_string();
    let run_mode = if is_portable_mode() {
        "便携版"
    } else {
        "标准版"
    };
    let data_path = data_dir.to_string_lossy().to_string();

    // 2) 活跃设置摘要：读取全部设置，跳过敏感键，整体在末尾再做一次脱敏兜底
    let db = app.state::<DbState>();
    let settings = db
        .settings_repo
        .get_all()
        .map_err(|e| AppError::Database(e.to_string()))?;
    // Beyond the encrypted secrets: the E2E salt + verifier together allow offline guessing of
    // the passphrase, and account names / server URLs (which may embed `user:pass@`) identify
    // the user. Diagnostics get pasted into public issues.
    let is_private_for_report = |k: &str| {
        k.starts_with("cloud_sync_e2e_")
            || k == "file_server_access_key"
            // On a shared broker the topic alone lets anyone push into this clipboard.
            || k == "mqtt_topic"
            || k == "mqtt_client_id"
            || k == "app.anon_id"
            || k.ends_with("_username")
            || k == "cloud_sync_webdav_url"
            || k == "cloud_sync_server"
            || k == "mqtt_server"
    };
    let mut keys: Vec<&String> = settings
        .keys()
        .filter(|k| !is_sensitive_key(k) && !is_private_for_report(k))
        .collect();
    keys.sort();
    let mut settings_summary = String::new();
    for k in keys {
        if let Some(v) = settings.get(k) {
            settings_summary.push_str(&format!("{} = {}\n", k, v));
        }
    }

    // 3) 日志：tiez.log 末 200 行（tiez.log 为内部兼容字段，仅读取不修改）
    let log_path = data_dir.join("tiez.log");
    let log_tail = match std::fs::read_to_string(&log_path) {
        Ok(content) => tail_lines(&content, LOG_TAIL_LINES),
        Err(e) => format!("(无法读取日志 {}: {})", log_path.to_string_lossy(), e),
    };

    // 4) 组装报告并脱敏
    let tail_n = LOG_TAIL_LINES;
    let report = build_diagnostics_report(
        &app_version,
        &os_version,
        run_mode,
        &data_path,
        &settings_summary,
        &log_tail,
        tail_n,
    );

    Ok(redact(&report))
}

// Feature: magpie-v0-4-1, Property 12: 诊断信息脱敏不泄露
#[cfg(test)]
mod property_12_redact {
    use super::redact;
    use proptest::prelude::*;

    /// 敏感字段键名（覆盖 redact 正则识别的全部键）。
    fn sensitive_key() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("password"),
            Just("passwd"),
            Just("pwd"),
            Just("token"),
            Just("secret"),
            Just("api_key"),
            Just("apikey"),
            Just("api-key"),
            // 大小写混合：redact 使用 (?i) 不区分大小写
            Just("Password"),
            Just("TOKEN"),
            Just("ApiKey"),
        ]
        .prop_map(|s| s.to_string())
    }

    /// 生成带唯一前缀 `Zq9` 的敏感明文值：
    /// - 字符集限定为字母数字，落在 redact 值正则 `[^\s,;"}\]]+` 内，确保会被掩码；
    /// - `Zq9` 前缀保证该值不会与任何键名、分隔符、掩码（`***`/`<redacted>`）或固定文案重叠，
    ///   从而「输出中不含该值」的断言不会因巧合子串而误判。
    fn secret_value() -> impl Strategy<Value = String> {
        "[A-Za-z0-9]{8,24}".prop_map(|s| format!("Zq9{}", s))
    }

    /// 键值分隔符的若干常见形态（均能被 redact 的键值正则匹配）。
    fn separator() -> impl Strategy<Value = String> {
        prop_oneof![Just("="), Just(": "), Just(" = "), Just(":")].prop_map(|s| s.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]
        #[test]
        fn 诊断信息脱敏不泄露(
            key in sensitive_key(),
            sep in separator(),
            value in secret_value(),
            shape in 0u8..3,
        ) {
            // 按三类形态构造含敏感字段的诊断输入：键值对 / JSON 风格 / URL query string
            let input = match shape {
                0 => format!("{}{}{}", key, sep, value),                 // key=value
                1 => format!("\"{}\":\"{}\"", key, value),               // "key":"value"
                _ => format!("https://example.com/api?{}={}", key, value), // URL query
            };

            let output = redact(&input);

            // 核心属性：脱敏输出不得包含敏感字段的原始明文值
            prop_assert!(
                !output.contains(&value),
                "脱敏输出不得泄露原始明文值：shape={} input={:?} output={:?}",
                shape,
                input,
                output
            );
            // 脱敏确实发生了变化（掩码 *** 或 URL <redacted>）
            prop_assert_ne!(
                &output,
                &input,
                "含敏感字段的输入经脱敏后应发生变化：input={:?}",
                input
            );
        }
    }

    #[test]
    fn 非敏感文本不被改动() {
        // 守门单测：不含敏感字段的普通文本应原样返回，避免过度脱敏
        let plain = "这是一行普通日志 level=info module=clipboard count=42";
        assert_eq!(redact(plain), plain);
    }

    #[test]
    fn 引号包裹含空格的敏感值被完整掩码() {
        // 回归：值被双引号包裹且含空格时，旧正则在空格处截断会残留尾部明文。
        // 改进后应掩码整个引号内容，输出不得包含原始明文片段。
        let input = r#"password = "my secret value""#;
        let out = redact(input);
        assert!(!out.contains("my secret value"), "引号内含空格的明文不得残留：{}", out);
        assert!(!out.contains("secret value"), "不得残留任何明文尾部：{}", out);
        assert!(out.contains("***"), "应已掩码：{}", out);
    }
}

#[cfg(test)]
mod diagnostics_report_tests {
    use super::{build_diagnostics_report, tail_lines};

    // Requirements 7.2, 7.3：诊断内容含日志末 200 行、系统信息、设置摘要，且无网络调用。
    #[test]
    fn 日志仅保留末_200_行() {
        // 构造 250 行日志（line1 ~ line250）
        let content: String = (1..=250).map(|i| format!("line{}\n", i)).collect();
        let tail = tail_lines(&content, 200);
        let lines: Vec<&str> = tail.lines().collect();
        assert_eq!(lines.len(), 200, "应恰好保留末 200 行");
        assert_eq!(lines.first(), Some(&"line51"), "末 200 行应从 line51 开始");
        assert_eq!(lines.last(), Some(&"line250"), "末行应为 line250");
        assert!(!tail.contains("line50"), "第 50 行（及更早）应被截断");
    }

    #[test]
    fn 行数不足时全部保留() {
        let content = "only-one-line";
        assert_eq!(tail_lines(content, 200), "only-one-line");
    }

    #[test]
    fn 报告包含系统信息_设置摘要与日志末行() {
        let settings_summary = "app.card_density = standard\napp.theme = dark\n";
        let log_tail = "line51\nline52\nline250";
        let report = build_diagnostics_report(
            "0.4.1",
            "Windows 10.0.26100",
            "便携版",
            r"D:\portable\data",
            settings_summary,
            log_tail,
            200,
        );

        // 系统信息：应用版本 / 系统版本 / 运行模式 / 数据路径
        assert!(report.contains("应用版本: 0.4.1"), "报告应含应用版本");
        assert!(report.contains("系统版本: Windows 10.0.26100"), "报告应含系统版本");
        assert!(report.contains("运行模式: 便携版"), "报告应含运行模式（是否便携）");
        assert!(report.contains(r"数据路径: D:\portable\data"), "报告应含数据路径");
        // 设置摘要
        assert!(report.contains("活跃设置摘要"), "报告应含设置摘要分节标题");
        assert!(report.contains("app.card_density = standard"), "报告应含设置项");
        // 日志末 200 行分节
        assert!(report.contains("tiez.log 末 200 行"), "报告应含日志分节标题");
        assert!(report.contains("line250"), "报告应含日志末行内容");
    }

    #[test]
    fn 报告组装为纯函数无外部依赖() {
        // 无网络调用的可验证代理：报告完全由入参决定，相同入参两次组装结果一致（确定性）。
        let a = build_diagnostics_report("v", "os", "标准版", "path", "s=1\n", "log", 200);
        let b = build_diagnostics_report("v", "os", "标准版", "path", "s=1\n", "log", 200);
        assert_eq!(a, b, "报告组装应为确定性纯函数，不依赖任何外部 IO / 网络");
    }
}
