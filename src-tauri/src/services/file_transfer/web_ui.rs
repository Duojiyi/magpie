fn file_transfer_theme_variants(theme: &str) -> (&'static str, &'static str, &'static str) {
    match theme {
        "paper" => (
            r#"
            --bg-body: #f4ecd8;
            --bg-window: #fdf6e3;
            --bg-panel: rgba(255, 253, 247, 0.78);
            --bg-input: #ffffff;
            --bg-button: rgba(139, 90, 43, 0.08);
            --border-dark: #d5c4a1;
            --text-primary: #3c3836;
            --text-secondary: #7c6f64;
            --accent-color: #8b5a2b;
            --shadow-color: rgba(60, 56, 54, 0.12);
            --font-mono: "Courier New", Courier, monospace;
            --content-font-family: "Georgia", "STSong", "SimSun", "Songti SC", serif;
            --radius: 2px;
            --bubble-received-bg: #fffdf7;
            --panel-border: 1px solid #d5c4a1;
            --panel-radius: 2px;
            --panel-shadow: 0 2px 10px rgba(0, 0, 0, 0.03);
            --input-border: 1px solid #d5c4a1;
            --input-radius: 2px;
            --input-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.03);
            --button-border: 1px solid rgba(139, 90, 43, 0.3);
            --button-radius: 4px;
            --send-button-background: rgba(139, 90, 43, 0.12);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px solid rgba(139, 90, 43, 0.28);
            --send-button-shadow: 0 4px 10px rgba(139, 90, 43, 0.12);
            "#,
            r#"
            --bg-body: #282828;
            --bg-panel: rgba(50, 48, 47, 0.82);
            --bg-input: #1d2021;
            --bg-button: rgba(213, 196, 161, 0.1);
            --border-dark: #504945;
            --text-primary: #ebdbb2;
            --text-secondary: #a89984;
            --accent-color: #d79921;
            --shadow-color: rgba(0, 0, 0, 0.3);
            --bubble-received-bg: #32302f;
            --panel-border: 1px solid #504945;
            --input-border: 1px solid #504945;
            --send-button-background: rgba(215, 153, 33, 0.16);
            --send-button-color: var(--text-primary);
            --send-button-border: 1px solid rgba(215, 153, 33, 0.26);
            --send-button-shadow: 0 4px 10px rgba(0, 0, 0, 0.18);
            "#,
            r#"
            body.theme-paper {{
                background-image: linear-gradient(rgba(139, 90, 43, 0.06) 1px, transparent 1px);
                background-size: 100% 1.65em;
            }}
            body.theme-paper::before {{
                content: "";
                position: fixed;
                inset: 0;
                background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E");
                opacity: 0.04;
                pointer-events: none;
                z-index: 9999;
            }}
            @media (prefers-color-scheme: dark) {{
                body.theme-paper:not(.light-mode) {{
                    background-image: linear-gradient(rgba(213, 196, 161, 0.04) 1px, transparent 1px);
                }}
            }}
            "#,
        ),
        // 晨雾 mist（液态玻璃·浅）：雾绿强调色，玻璃质感由 .theme-glass 选择器统一驱动
        "mist" => (
            r#"
            --bg-body: #eef3f1;
            --bg-panel: rgba(255, 255, 255, 0.48);
            --bg-input: rgba(255, 255, 255, 0.74);
            --bg-button: rgba(255, 255, 255, 0.46);
            --border-dark: rgba(78, 140, 125, 0.18);
            --text-primary: #1f2a27;
            --text-secondary: #5a6c66;
            --accent-color: #4e8c7d;
            --shadow-color: rgba(31, 42, 39, 0.1);
            --font-mono: system-ui, -apple-system, sans-serif;
            --radius: 12px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
            --panel-border: 1px solid rgba(255, 255, 255, 0.3);
            --send-button-background: #4e8c7d;
            --send-button-color: #ffffff;
            "#,
            r#"
            --bg-body: #1a1f1d;
            --bg-panel: rgba(36, 42, 40, 0.52);
            --bg-input: rgba(255, 255, 255, 0.12);
            --bg-element: rgba(45, 52, 50, 0.65);
            --bg-button: rgba(255, 255, 255, 0.05);
            --text-primary: #e8eeec;
            --text-secondary: #a8b2af;
            --accent-color: #6fa897;
            --border-dark: rgba(255, 255, 255, 0.08);
            --bubble-received-bg: rgba(40, 46, 44, 0.9);
            --send-button-background: #6fa897;
            --send-button-color: #ffffff;
            "#,
            r#"
            .theme-glass header, .theme-glass footer {{
                backdrop-filter: blur(16px) saturate(180%);
                -webkit-backdrop-filter: blur(16px) saturate(180%);
            }}
            .theme-glass .message.received .bubble {{
                backdrop-filter: blur(10px);
                -webkit-backdrop-filter: blur(10px);
            }}
            .theme-glass .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(78, 140, 125, 0.3);
            }}
            "#,
        ),
        // 暮山 dusk（液态玻璃·深）：黄铜强调色，玻璃质感由 .theme-glass 选择器统一驱动
        "dusk" => (
            r#"
            --bg-body: #f1ede6;
            --bg-panel: rgba(255, 255, 255, 0.34);
            --bg-input: rgba(255, 255, 255, 0.52);
            --bg-button: rgba(255, 255, 255, 0.2);
            --border-dark: rgba(201, 162, 107, 0.32);
            --text-primary: #2a2420;
            --text-secondary: #6b6258;
            --accent-color: #c9a26b;
            --shadow-color: rgba(42, 36, 32, 0.08);
            --font-mono: system-ui, -apple-system, sans-serif;
            --radius: 10px;
            --bubble-received-bg: rgba(255, 255, 255, 0.4);
            --send-button-background: #c9a26b;
            --send-button-color: #2a2420;
            "#,
            r#"
            --bg-body: #1a1620;
            --bg-panel: rgba(28, 24, 32, 0.38);
            --bg-input: rgba(255, 255, 255, 0.12);
            --bg-button: rgba(255, 255, 255, 0.06);
            --border-dark: rgba(255, 255, 255, 0.12);
            --text-primary: #ece6dd;
            --text-secondary: #b2a89c;
            --accent-color: #c9a26b;
            --bubble-received-bg: rgba(45, 40, 50, 0.42);
            --send-button-background: #c9a26b;
            --send-button-color: #1a1620;
            "#,
            r#"
            .theme-glass header, .theme-glass footer {{
                backdrop-filter: blur(16px) saturate(145%);
                -webkit-backdrop-filter: blur(16px) saturate(145%);
            }}
            .theme-glass .message.received .bubble {{
                backdrop-filter: blur(15px);
                -webkit-backdrop-filter: blur(15px);
            }}
            .theme-glass .retro-btn.send-btn {{
                background: var(--send-button-background);
                color: var(--send-button-color);
                border: none;
                box-shadow: 0 4px 12px rgba(201, 162, 107, 0.3);
            }}
            "#,
        ),
        _ => (
            r#"
            --bg-body: #dcdcdc;
            --bg-panel: #f3f3f3;
            --bg-input: #ffffff;
            --bg-button: #e0e0e0;
            --border-dark: #373737;
            --text-primary: #373737;
            --text-secondary: #707070;
            --accent-color: #487bdb;
            --shadow-color: #373737;
            --font-mono: "Courier New", Courier, monospace;
            --content-font-family: var(--font-mono);
            --radius: 0px;
            --bubble-received-bg: #ffffff;
            --panel-border: 2px solid var(--border-dark);
            --panel-radius: 4px;
            --panel-shadow: 2px 2px 0 0 var(--shadow-color);
            --input-border: 3px solid var(--border-dark);
            --input-radius: 0;
            --input-shadow: inset 4px 4px 0 rgba(0, 0, 0, 0.1);
            --button-border: 2px solid var(--border-dark);
            --button-radius: 0;
            --button-shadow: 2px 2px 0 0 var(--shadow-color);
            --button-active-transform: translate(2px, 2px);
            --button-active-shadow: 0 0 0 0 var(--shadow-color);
            --button-active-filled-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.2);
            --send-button-background: var(--accent-color);
            --send-button-color: #ffffff;
            --send-button-border: 2px solid var(--border-dark);
            --send-button-shadow: inset 2px 2px 0 rgba(0, 0, 0, 0.2);
            "#,
            r#"
            --bg-body: #121212;
            --bg-panel: #1e1e1e;
            --bg-input: #202020;
            --bg-button: #333333;
            --border-dark: #000000;
            --text-primary: #e0e0e0;
            --text-secondary: #a0a0a0;
            --accent-color: #5a8dee;
            --shadow-color: #000000;
            --bubble-received-bg: #1e1e1e;
            --panel-border: 2px solid #000000;
            --panel-shadow: 2px 2px 0 0 #000000;
            --input-border: 2px solid #000000;
            --input-shadow: inset 2px 2px 0 0 rgba(0,0,0,0.5);
            --button-border: 2px solid #000000;
            --button-shadow: 2px 2px 0 0 #000000;
            --send-button-background: #000000;
            --send-button-color: #ffffff;
            --send-button-border: 2px solid #000000;
            --send-button-shadow: none;
            "#,
            "",
        ),
    }
}

/// 旧主题别名归一（语义同设计文档表 4.1）：
/// mica/sakura→mist、acrylic→dusk、retro→ink、sticky-note→paper，
/// ink/paper/mist/dusk 原样返回，store-* 与未知值一律落默认 ink。
fn normalize_theme_id(theme: &str) -> &str {
    match theme {
        "mica" | "sakura" => "mist",
        "acrylic" => "dusk",
        "retro" => "ink",
        "sticky-note" => "paper",
        "ink" | "paper" | "mist" | "dusk" => theme,
        t if t.starts_with("store-") => "ink",
        _ => "ink",
    }
}

/// 玻璃主题判定：仅归一后为 mist / dusk 时为真，用于追加 theme-glass class。
fn is_glass_theme(theme: &str) -> bool {
    matches!(normalize_theme_id(theme), "mist" | "dusk")
}

fn file_transfer_theme_css(theme: &str, color_mode: &str) -> String {
    let (light, dark, extra) = file_transfer_theme_variants(theme);
    let dark_css = match color_mode {
        "dark" => format!(":root {{{dark}}}"),
        "system" => format!("@media (prefers-color-scheme: dark) {{ :root {{{dark}}} }}"),
        _ => String::new(),
    };

    format!(":root {{{light}}}\n{dark_css}\n{extra}")
}

pub fn render_index(theme: &str, color_mode: &str, logo_base64: &str) -> String {
    // 先将原始（可能为旧别名）主题值归一为新主题集合，再据此生成 CSS 与 body class
    let theme = normalize_theme_id(theme);
    let theme_css = file_transfer_theme_css(theme, color_mode);
    let mode_class = match color_mode {
        "dark" => "dark-mode",
        "light" => "light-mode",
        _ => "",
    };
    // 玻璃主题（mist/dusk）追加 theme-glass 语义 class，驱动统一的玻璃样式分支
    let glass_class = if is_glass_theme(theme) { "theme-glass" } else { "" };

    format!(
        r#"
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover, interactive-widget=resizes-content">
    <title>Magpie 终端传输</title>
    <style>
        * {{ box-sizing: border-box; -webkit-tap-highlight-color: transparent; }}
        
        :root {{
            --bg-body: #dcdcdc;
            --bg-panel: #f3f3f3;
            --border-dark: #373737;
            --text-primary: #373737;
            --accent-color: #487bdb;
            --shadow-color: #373737;
            --font-mono: "Courier New", Courier, monospace;
            --radius: 0px;
            --bubble-received-bg: #ffffff;
            --app-height: 100vh;
        }}

        @media (prefers-color-scheme: dark) {{
            .theme-glass {{
                --bg-body: #101010;
                --bg-panel: #1c1c1c;
                --border-dark: rgba(255,255,255,0.08); /* 弱化边框 */
                --text-primary: #e0e0e0;
                --shadow-color: rgba(0,0,0,0.5);
            }}
            :root:not(.theme-glass) {{
                --bg-body: #121212; /* 略偏冷的黑 */
                --bg-panel: #1e1e1e;
                --border-dark: #2a2a2a; /* 用深灰替代纯黑做边框 */
                --text-primary: #e0e0e0; 
                --shadow-color: #000000;
                --bubble-received-bg: var(--bg-panel);
            }}
        }}

        /* 玻璃主题（mist / dusk）公共覆盖 */
        .theme-glass {{
            --bg-body: #f3f3f3;
            --bg-panel: #ffffff;
            --border-dark: rgba(0,0,0,0.1);
            --text-primary: #333;
            --shadow-color: rgba(0,0,0,0.1);
            --font-mono: "Segoe UI", system-ui, -apple-system, sans-serif;
            --radius: 12px;
            --bubble-received-bg: rgba(255, 255, 255, 0.9);
        }}

        @media (prefers-color-scheme: dark) {{
            .theme-glass {{
                --bg-body: #1a1a1a;
                --bg-panel: #2a2a2a;
                --text-primary: #e0e0e0;
                --bubble-received-bg: rgba(40, 40, 40, 0.9);
            }}
        }}

        html, body {{
            height: 100%;
            width: 100%;
            margin: 0;
            padding: 0;
            overflow: hidden;
            position: fixed;
            top: 0;
            left: 0;
        }}

        body {{
            background-color: var(--bg-body);
            color: var(--text-primary);
            font-family: var(--content-font-family, var(--font-mono));
            display: flex;
            flex-direction: column;
            height: var(--app-height);
            transition: background 0.3s;
        }}

        header {{
            height: 60px;
            background: var(--bg-panel);
            border-bottom: 2px solid var(--border-dark);
            display: flex; align-items: center; justify-content: center;
            padding: 0 16px; position: relative;
            flex-shrink: 0;
            z-index: 10;
        }}
        .theme-glass header {{
            background: rgba(255,255,255,0.7); backdrop-filter: blur(16px);
            border-bottom: 1px solid rgba(0,0,0,0.1);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-glass header {{ background: rgba(30,30,30,0.7); }}
        }}

        h1 {{ font-size: 18px; font-weight: 900; margin: 0; letter-spacing: -0.5px; }}
        
        .header-status {{
            position: absolute; left: 16px; top: 50%; transform: translateY(-50%);
            display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: bold;
        }}
        .status-dot {{ width: 8px; height: 8px; background: #4caf50; border-radius: 50%; box-shadow: 0 0 5px #4caf50; }}

        #chat-box {{
            flex: 1; overflow-y: auto; padding: 16px;
            display: flex; flex-direction: column; gap: 16px;
            scroll-behavior: smooth;
            padding-bottom: 40px;
        }}

        .timestamp {{
            font-size: 11px; text-align: center; opacity: 0.6;
            margin: 8px 0; font-weight: bold;
        }}

        .message {{ display: flex; gap: 10px; max-width: 90%; }}
        .message.received {{ align-self: flex-start; }}
        .message.sent {{ align-self: flex-end; flex-direction: row-reverse; }}

        .avatar {{
            width: 36px; height: 36px; background: #fff;
            border: 2px solid var(--border-dark);
            display: flex; align-items: center; justify-content: center;
            font-weight: bold; font-size: 18px; flex-shrink: 0;
            box-shadow: 2px 2px 0 var(--shadow-color);
            border-radius: var(--radius); overflow: hidden;
        }}
        .avatar img {{ width: 100%; height: 100%; object-fit: cover; }}
        
        .theme-glass .avatar {{
            box-shadow: none; border-width: 1px;
        }}

        .bubble {{
            padding: 10px 14px;
            background: #fff;
            border: 2px solid var(--border-dark);
            box-shadow: 3px 3px 0 var(--shadow-color);
            font-size: 14px; line-height: 1.5;
            word-break: break-all;
            position: relative;
            border-radius: var(--radius);
        }}
        .message.received .bubble {{ background: var(--bubble-received-bg); }}
        .message.sent .bubble {{ background: var(--accent-color); color: #fff; border-color: var(--border-dark); }}
        
        /* 玻璃主题气泡样式 */
        .theme-glass .bubble {{
            box-shadow: 0 2px 10px var(--shadow-color);
            border: 1px solid var(--border-dark);
            border-radius: 12px;
        }}
        .theme-glass .message.received .bubble {{
             background: var(--bubble-received-bg);
             backdrop-filter: blur(10px);
        }}

        /* 三角气泡尾仅扁平主题使用 */
        :root:not(.theme-glass) .message.received .bubble::after {{
            content: ''; position: absolute; left: -10px; top: 10px;
            width: 0; height: 0; border: 5px solid transparent;
            border-right-color: var(--bubble-received-bg);
        }}
        :root:not(.theme-glass) .message.received .bubble::before {{
            content: ''; position: absolute; left: -13px; top: 9px;
            width: 0; height: 0; border: 6px solid transparent;
            border-right-color: var(--border-dark);
        }}
        :root:not(.theme-glass) .message.sent .bubble::after {{
            content: ''; position: absolute; right: -10px; top: 10px;
            width: 0; height: 0; border: 5px solid transparent;
            border-left-color: var(--accent-color);
        }}
        :root:not(.theme-glass) .message.sent .bubble::before {{
            content: ''; position: absolute; right: -13px; top: 9px;
            width: 0; height: 0; border: 6px solid transparent;
            border-left-color: var(--border-dark);
        }}
        
        /* 玻璃主题不使用三角尾 */
        .theme-glass .message.received .bubble::before {{ border-right-color: var(--border-dark); }}
        .theme-glass .message.received .bubble::after {{ border-right-color: var(--bubble-received-bg); }}
        
        @media (prefers-color-scheme: dark) {{
            .theme-glass .message.received .bubble {{ background: var(--bubble-received-bg); }}
        }}

        .theme-glass .message.sent .bubble::before {{ display: none; }}
        .theme-glass .message.sent .bubble::after {{ 
            border-left-color: var(--accent-color) !important; 
            right: -7px; 
            bottom: 10px;
        }}

        
        /* File Card */
        .file-card {{ display: flex; align-items: center; gap: 12px; }}
        .file-icon {{ font-size: 24px; flex-shrink: 0; }}
        .file-info {{ display: flex; flex-direction: column; min-width: 0; overflow: hidden; }}
        .file-name {{ font-weight: 700; font-family: var(--font-mono); font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 160px; }}
        .file-size {{ font-size: 11px; font-family: var(--font-mono); opacity: 0.8; margin-top: 2px; }}

        /* Image Preview */
        .img-preview {{
            max-width: 100%;
            width: auto;
            max-height: 400px;
            display: block;
            border: 1px solid rgba(0,0,0,0.1);
            margin-bottom: 6px;
            border-radius: 8px;
            object-fit: contain;
        }}
        .video-preview {{
            width: 100%;
            max-width: 100%;
            min-height: 120px;
            max-height: 400px;
            display: block;
            margin-bottom: 6px;
            border-radius: 8px;
            background: #000;
        }}

        .progress-wrapper {{ margin-top: 8px; border-top: 1px dashed rgba(255,255,255,0.3); padding-top: 4px; }}
        .progress-bar {{ width: 100%; height: 4px; background: rgba(0,0,0,0.1); border-radius: 2px; overflow: hidden; margin-top: 4px; }}
        .progress-inner {{ height: 100%; background: var(--accent-color); width: 0%; transition: width 0.2s; }}

        footer {{
            padding: 10px 16px;
            padding-bottom: calc(10px + env(safe-area-inset-bottom));
            background: var(--bg-panel);
            border-top: 2px solid var(--border-dark);
            display: flex; gap: 12px; align-items: flex-end;
            flex-shrink: 0;
            z-index: 10;
        }}
        .theme-glass footer {{
            background: rgba(255,255,255,0.7); backdrop-filter: blur(16px);
            border-top: 1px solid rgba(0,0,0,0.1);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-glass footer {{ background: rgba(30,30,30,0.7); }}
        }}
        
        .retro-btn {{
            background: var(--bg-button);
            border: var(--button-border, 2px solid var(--border-dark));
            box-shadow: var(--button-shadow, 2px 2px 0 0 var(--shadow-color));
            display: flex; align-items: center; justify-content: center;
            cursor: pointer; color: var(--text-primary);
            transition: all 0.1s;
            height: 40px;
            flex-shrink: 0;
            border-radius: var(--button-radius, var(--radius));
        }}
        .retro-btn:active {{
            transform: var(--button-active-transform, translate(2px, 2px));
            box-shadow: var(--button-active-shadow, 0 0 0 0 var(--shadow-color));
        }}
        
        .theme-glass .retro-btn {{
            box-shadow: none; border-width: 1px;
            background: rgba(255,255,255,0.5);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-glass .retro-btn {{ background: rgba(255,255,255,0.1); }}
        }}

        .add-btn {{ width: 40px; font-size: 24px; font-weight: 900; }}
        .send-btn {{
            min-width: 72px;
            padding: 0 16px;
            background: var(--send-button-background, var(--accent-color));
            color: var(--send-button-color, #ffffff);
            border: var(--send-button-border, var(--button-border, 2px solid var(--border-dark)));
            box-shadow: var(--send-button-shadow, var(--button-active-filled-shadow, 2px 2px 0 0 var(--shadow-color)));
        }}
        
        .text-input {{ 
            flex: 1; height: 40px; min-height: 40px; max-height: 100px; padding: 10px 12px;
            background: var(--bg-input); border: var(--input-border, 2px solid var(--border-dark));
            box-shadow: var(--input-shadow, inset 2px 2px 0 rgba(0,0,0,0.1));
            font-size: 14px; font-family: var(--content-font-family, var(--font-mono));
            color: var(--text-primary); outline: none;
            border-radius: var(--input-radius, var(--radius)); -webkit-appearance: none;
            resize: none; overflow-y: auto; line-height: 20px;
        }}
        .theme-glass .text-input {{ box-shadow: none; border-width: 1px; }}
        @media (prefers-color-scheme: dark) {{
            .theme-glass .text-input {{ background: rgba(255,255,255,0.05); color: #fff; }}

            /* 扁平主题暗色覆盖 */
            :root:not(.theme-glass) .retro-btn {{
                background: #333;
                border-color: #000;
                color: #e0e0e0;
                box-shadow: 2px 2px 0 0 #000;
            }}
            :root:not(.theme-glass) .retro-btn:active {{
                box-shadow: none;
                transform: translate(2px, 2px);
            }}
            :root:not(.theme-glass) .retro-btn.send-btn {{
                background: #000;
                color: #fff;
                border-color: #000;
            }}
            :root:not(.theme-glass) .text-input {{
                background: #202020;
                color: #e0e0e0;
                border-color: #000;
                box-shadow: inset 2px 2px 0 0 rgba(0,0,0,0.5);
            }}
        }}
        
        .expand-btn {{
            width: 30px; height: 30px; display: none; align-items: center; justify-content: center;
            position: absolute; right: 5px; bottom: 5px;
            background: var(--bg-body); border: 1px solid var(--border-dark);
            border-radius: 4px; cursor: pointer; z-index: 5;
            color: var(--text-primary);
        }}

        /* Fullscreen Editor */
        #fs-editor {{
            position: fixed; top: 0; left: 0; width: 100%; height: 100%;
            background: var(--bg-body); z-index: 1000;
            display: none; flex-direction: column;
            padding: 16px;
        }}
        .theme-glass #fs-editor {{
             background: rgba(255,255,255,0.95); backdrop-filter: blur(16px);
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-glass #fs-editor {{ background: rgba(20,20,20,0.95); }}
            
            /* 扁平主题全屏编辑器暗色覆盖 */
            :root:not(.theme-glass) #fs-textarea {{
                background: #202020;
                color: #e0e0e0;
                border: 2px solid #000;
            }}
        }}

        #fs-textarea {{
            flex: 1; width: 100%; border: 2px solid var(--border-dark);
            padding: 16px; font-size: 16px; font-family: var(--font-mono);
            background: #fff; color: var(--text-primary); margin-bottom: 16px;
            border-radius: var(--radius); resize: none; outline: none;
        }}
        .theme-glass #fs-textarea {{
            background: rgba(255,255,255,0.5); border-width: 1px; box-shadow: none;
        }}
        @media (prefers-color-scheme: dark) {{
            .theme-glass #fs-textarea {{ background: rgba(255,255,255,0.05); color: #fff; }}
        }}

        .fs-toolbar {{ display: flex; justify-content: flex-end; gap: 12px; }}
        {theme_css}
    </style>
</head>
<body class="theme-{theme} {glass_class} {mode_class}">
    <header>
        <div class="header-status">
            <div class="status-dot"></div>
            <span id="device-count">Linked</span>
        </div>
        <h1>Magpie 终端</h1>
    </header>

    <div id="chat-box">
        <div class="timestamp">SYS: <span id="time-now"></span></div>
        <div class="message received">
            <div class="avatar"><img src="{logo_base64}" onerror="this.innerText='T'"></div>
            <div class="bubble">
                <div style="font-weight:900; margin-bottom:4px">SYSTEM READY</div>
                发送文字、图片或文件到电脑。
            </div>
        </div>
    </div>
    
    <!-- Full Screen Editor Modal -->
    <div id="fs-editor">
        <div style="font-weight:bold; margin-bottom:8px; display:flex; justify-content:space-between; align-items:center">
            <span>FULL SCREEN EDIT</span>
            <span onclick="closeFullscreen()" style="cursor:pointer; padding:4px;">✕</span>
        </div>
        <textarea id="fs-textarea" placeholder="输入内容..."></textarea>
        <div class="fs-toolbar">
            <button class="retro-btn" onclick="closeFullscreen()">CANCEL</button>
            <button class="retro-btn send-btn" onclick="sendFullscreen()">SEND</button>
        </div>
    </div>

    <footer>
        <label for="file-input" class="retro-btn add-btn">+</label>
        <div style="position:relative; flex:1; display:flex;">
            <textarea class="text-input" id="text-input" placeholder="输入文字..." rows="1"></textarea>
            <div class="expand-btn" id="expand-btn" onclick="openFullscreen()">⤢</div>
        </div>
        <button class="retro-btn send-btn" id="send-btn">SEND</button>
        <input type="file" id="file-input" multiple style="display:none">
    </footer>

    <!-- Fullscreen Image Overlay -->
    <div id="img-overlay" style="display:none; position:fixed; top:0; left:0; width:100%; height:100%; background:rgba(0,0,0,0.9); backdrop-filter:blur(5px); z-index:9999; align-items:center; justify-content:center; flex-direction:column;">
        <div style="position:absolute; top:20px; right:20px; color:white; font-size:24px; cursor:pointer; padding:10px;" onclick="closeOverlay()">✕</div>
        <img id="overlay-img" style="max-width:95%; max-height:90%; object-fit:contain; border-radius:4px; box-shadow:0 0 20px rgba(0,0,0,0.5);">
    </div>

    <script>
        const fileInput = document.getElementById('file-input');
        const textInput = document.getElementById('text-input');
        const sendBtn = document.getElementById('send-btn');
        const chatBox = document.getElementById('chat-box');
        
        const now = new Date();
        document.getElementById('time-now').innerText = `${{now.getHours().toString().padStart(2,'0')}}:${{now.getMinutes().toString().padStart(2,'0')}}`;
        
        let lastId = 0;
        let isUploading = false;
        const deviceId = localStorage.getItem('tiez_device_id') || ('m-' + Math.random().toString(36).substr(2, 9));
        localStorage.setItem('tiez_device_id', deviceId);
        
        const deviceName = "Mobile";
        const MAGPIE_LOGO = "{logo_base64}";
        const pendingUploads = new Map(); // filename -> [elements]

        function scrollToBottom() {{
            chatBox.scrollTop = chatBox.scrollHeight;
        }}

        function syncViewportMetrics() {{
            const vv = window.visualViewport;
            if (!vv) {{
                document.documentElement.style.setProperty('--app-height', `${{window.innerHeight}}px`);
                return;
            }}

            // Pull the height from the visual viewport (excludes keyboard)
            document.documentElement.style.setProperty('--app-height', `${{vv.height}}px`);
            
            // Keep the fixed body aligned with the visual viewport's top/left
            document.body.style.transform = `translate(${{vv.offsetLeft}}px, ${{vv.offsetTop}}px)`;
            
            // Prevent the browser from scrolling the layout viewport away from origin
            if (window.scrollY !== 0 || window.scrollX !== 0) {{
                window.scrollTo(0, 0);
            }}
        }}

        if (window.visualViewport) {{
            window.visualViewport.addEventListener('resize', syncViewportMetrics);
            window.visualViewport.addEventListener('scroll', syncViewportMetrics);
        }}
        window.addEventListener('resize', syncViewportMetrics);
        // Also sync on focus/blur to handle various keyboard states
        document.addEventListener('focusin', () => setTimeout(syncViewportMetrics, 50));
        document.addEventListener('focusout', () => setTimeout(syncViewportMetrics, 50));
        syncViewportMetrics();

        function escapeHTML(str) {{
            if (!str) return '';
            return str.replace(/[&<>"']/g, function(m) {{
                return {{
                    '&': '&amp;',
                    '<': '&lt;',
                    '>': '&gt;',
                    '"': '&quot;',
                    "'": '&#39;'
                }}[m];
            }});
        }}

        function normalizeFileName(name) {{
            if (!name) return '';
            const base = name.split('/').pop().split('\\').pop();
            const m = base.match(/^\d{{8,}}_(.+)$/);
            return escapeHTML(m ? m[1] : base);
        }}
        function extractNameFromContent(content, file_path) {{
            if (content && content.startsWith('/download/') && content.includes('?name=')) {{
                const idx = content.indexOf('?name=');
                if (idx !== -1) {{
                    try {{ return decodeURIComponent(content.slice(idx + 6)); }} catch (e) {{ return content; }}
                }}
            }}
            return file_path || content || '';
        }}
        function addPendingUpload(fileName, el) {{
            const list = pendingUploads.get(fileName) || [];
            list.push(el);
            pendingUploads.set(fileName, list);
        }}
        function takePendingUpload(maybeName) {{
            for (const [name, list] of pendingUploads.entries()) {{
                if (maybeName.endsWith(name) && list.length > 0) {{
                    const el = list.shift();
                    if (list.length === 0) pendingUploads.delete(name);
                    return el;
                }}
            }}
            return null;
        }}
        function createMessageElement(direction, content, senderName, msgType, file_path) {{
            const div = document.createElement('div');
            div.className = `message ${{direction}}`;
            
            let bubbleContent = escapeHTML(content);
            if (msgType === 'image' || (content.match(/\.(jpg|jpeg|png|gif|webp)$/i) && file_path)) {{
                const useContent = content.startsWith('data:') || content.startsWith('/download/') || content.startsWith('http');
                const src = escapeHTML(useContent ? content : (file_path || content));
                bubbleContent = `<img src="${{src}}" class="img-preview" onclick="openOverlay('${{src}}')">`;
            }} else if (msgType === 'video') {{
                const useContent = content.startsWith('/download/') || content.startsWith('http');
                const src = escapeHTML(useContent ? content : (file_path || content));
                bubbleContent = `<video class="video-preview" controls src="${{src}}"></video>`;
            }} else if (msgType === 'file' || file_path) {{
                 const rawName = extractNameFromContent(content, file_path);
                 const fileName = normalizeFileName(rawName);
                 bubbleContent = `
                    <div class="file-card">
                        <div class="file-icon">📄</div>
                        <div class="file-info">
                            <span class="file-name">${{fileName}}</span>
                            <span class="file-size">DOWNLOAD</span>
                        </div>
                    </div>
                 `;
            }}

            const escapedSenderName = escapeHTML(senderName);
            div.innerHTML = `
                ${{direction === 'received' ? (() => {{
                    const name = (senderName || '').trim();
                    const lower = name.toLowerCase();
                    const isPc = name === '电脑' || name === 'PC' || lower === 'pc' || lower === 'tiez' || lower === 'magpie';
                    if (isPc) {{
                        return `<div class="avatar"><img src="${{MAGPIE_LOGO}}" alt="Magpie"></div>`;
                    }}
                    return `<div class="avatar">${{name ? escapeHTML(name[0]) : '?'}}</div>`;
                }})() : ''}}
                <div class="bubble">
                    ${{escapedSenderName && escapedSenderName !== 'System' ? `<div style="font-size:10px; opacity:0.6; margin-bottom:2px">${{escapedSenderName}}</div>` : ''}}
                    ${{bubbleContent}}
                </div>
            `;
            
            const downloadUrl = content.startsWith('/download/') ? content : file_path;
            if (downloadUrl && msgType !== 'image' && msgType !== 'video') {{
                div.querySelector('.bubble').style.cursor = 'pointer';
                div.querySelector('.bubble').onclick = () => window.location.href = downloadUrl;
            }}
            
            return div;
        }}

        function openOverlay(src) {{
            document.getElementById('overlay-img').src = src;
            document.getElementById('img-overlay').style.display = 'flex';
        }}
        function closeOverlay() {{
            document.getElementById('img-overlay').style.display = 'none';
        }}

        function openFullscreen() {{
            document.getElementById('fs-textarea').value = textInput.value;
            document.getElementById('fs-editor').style.display = 'flex';
            document.getElementById('fs-textarea').focus();
        }}
        function closeFullscreen() {{
            document.getElementById('fs-editor').style.display = 'none';
        }}
        function sendFullscreen() {{
            const val = document.getElementById('fs-textarea').value;
            if (val.trim()) {{
                textInput.value = val;
                sendBtn.click();
            }}
            closeFullscreen();
        }}

        // Adjust textarea height
        textInput.addEventListener('input', function() {{
            this.style.height = '40px';
            const newHeight = Math.min(this.scrollHeight, 100);
            this.style.height = newHeight + 'px';
            document.getElementById('expand-btn').style.display = newHeight > 50 ? 'flex' : 'none';
        }});

        textInput.addEventListener('focus', () => {{
            setTimeout(() => {{
                syncViewportMetrics();
                scrollToBottom();
            }}, 80);
        }});

        window.addEventListener('resize', syncViewportMetrics);
        window.addEventListener('orientationchange', syncViewportMetrics);
        if (window.visualViewport) {{
            window.visualViewport.addEventListener('resize', syncViewportMetrics);
            window.visualViewport.addEventListener('scroll', syncViewportMetrics);
        }}
        syncViewportMetrics();

        // WebSocket Setup
        let socket;
        function connectWS() {{
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            socket = new WebSocket(`${{protocol}}//${{window.location.host}}/ws`);
            
            socket.onopen = () => {{
                socket.send(JSON.stringify({{ type: 'identity', device_id: deviceId, device_name: deviceName }}));
                console.log('WS Connected');
            }};
            
            socket.onmessage = (e) => {{
                const msg = JSON.parse(e.data);
                if (msg.direction === 'in' && msg.sender_id === deviceId && (msg.msg_type === 'file' || msg.msg_type === 'image' || msg.msg_type === 'video')) {{
                    const rawName = extractNameFromContent(msg.content, msg.file_path);
                    const maybeName = normalizeFileName(rawName);
                    const pending = takePendingUpload(maybeName);
                    if (pending) {{
                        const replacement = createMessageElement('sent', msg.content, 'You', msg.msg_type, msg.file_path);
                        pending.replaceWith(replacement);
                        scrollToBottom();
                        return;
                    }}
                }}
                if (msg.direction === 'out') {{
                    const el = createMessageElement('received', msg.content, msg.sender_name, msg.msg_type, msg.file_path);
                    chatBox.appendChild(el);
                    scrollToBottom();
                }} else if (msg.direction === 'in') {{
                    const el = createMessageElement('sent', msg.content, 'You', msg.msg_type, msg.file_path);
                    chatBox.appendChild(el);
                    scrollToBottom();
                }}
            }};
            
            socket.onclose = () => setTimeout(connectWS, 3000);
        }}
        connectWS();

        sendBtn.onclick = async () => {{
            const text = textInput.value.trim();
            if (!text || isUploading) return;
            
            textInput.value = '';
            textInput.style.height = '40px';
            document.getElementById('expand-btn').style.display = 'none';

            try {{
                await fetch('/send-text', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ content: text, sender_id: deviceId, sender_name: deviceName }})
                }});
            }} catch(e) {{ alert('Send failed'); }}
        }};

        fileInput.onchange = async () => {{
            if (!fileInput.files.length || isUploading) return;
            const files = Array.from(fileInput.files);
            fileInput.value = '';
            
            for(const file of files) {{
                await uploadFile(file);
            }}
        }};

        async function uploadFile(file) {{
            isUploading = true;
            const isImage = file.type.startsWith('image/');
            const isVideo = file.type.startsWith('video/');
            const msgType = isVideo ? 'video' : (isImage ? 'image' : 'file');
            const previewUrl = (isImage || isVideo) ? URL.createObjectURL(file) : file.name;
            const el = createMessageElement('sent', previewUrl, 'You', msgType, undefined);
            el.dataset.fileName = file.name;
            el.dataset.pending = 'true';
            addPendingUpload(file.name, el);
            const progressWrapper = document.createElement('div');
            progressWrapper.className = 'progress-wrapper';
            progressWrapper.innerHTML = `<div style="font-size:10px">0%</div><div class="progress-bar"><div class="progress-inner"></div></div>`;
            el.querySelector('.bubble').appendChild(progressWrapper);
            chatBox.appendChild(el);
            scrollToBottom();

            const CHUNK_SIZE = 1024 * 512; // 512KB
            const totalChunks = Math.ceil(file.size / CHUNK_SIZE);
            const uploadId = Math.random().toString(36).substr(2, 9);

            for (let i = 0; i < totalChunks; i++) {{
                const start = i * CHUNK_SIZE;
                const end = Math.min(file.size, start + CHUNK_SIZE);
                const chunk = file.slice(start, end);

                const formData = new FormData();
                formData.append('file', chunk);
                formData.append('metadata', JSON.stringify({{
                    upload_id: uploadId,
                    chunk_index: i,
                    total_chunks: totalChunks,
                    file_name: file.name,
                    sender_id: deviceId,
                    sender_name: deviceName,
                    total_size: file.size,
                    content_type: file.type
                }}));

                try {{
                    const res = await fetch('/upload-chunk', {{ method: 'POST', body: formData }});
                    if (!res.ok) throw new Error('Chunk failed');
                    
                    const percent = Math.round(((i + 1) / totalChunks) * 100);
                    progressWrapper.querySelector('.progress-inner').style.width = percent + '%';
                    progressWrapper.querySelector('div').innerText = percent + '%';
                }} catch (e) {{
                    alert('Upload failed: ' + file.name);
                    // Remove pending marker on failure
                    el.dataset.pending = 'false';
                    const list = pendingUploads.get(file.name) || [];
                    const idx = list.indexOf(el);
                    if (idx >= 0) {{ list.splice(idx, 1); }}
                    if (list.length === 0) pendingUploads.delete(name);
                    break;
                }}
            }}
            
            progressWrapper.remove();
            el.querySelector('.bubble').innerHTML += ' <span style="color:#4caf50">✓</span>';
            isUploading = false;
        }}

        // Dragon-drop support
        document.addEventListener('dragover', e => e.preventDefault());
        document.addEventListener('drop', async e => {{
            e.preventDefault();
            const files = Array.from(e.dataTransfer.files);
            if (files.length) {{
                 for(const file of files) await uploadFile(file);
                 
                 // Small delay for UI and then notify PC
                 setTimeout(() => {{
                     const fileCount = files.length;
                     const replyEl = createMessageElement('received', `ACK: <b>${{fileCount}}</b> FILES SAVED.`, 'System', 'pc');
                     chatBox.appendChild(replyEl);
                     scrollToBottom();
                 }}, 800);
            }}
        }});

    </script>
</body>
</html>
    "#,
        theme = theme,
        mode_class = mode_class,
        glass_class = glass_class,
        theme_css = theme_css,
        logo_base64 = logo_base64
    )
}
