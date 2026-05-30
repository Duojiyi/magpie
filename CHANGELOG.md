# 更新日志

本仓库 fork 自 [`jimuzhe/tiez-clipboard`](https://github.com/jimuzhe/tiez-clipboard)，依据 GPL-3.0 协议二次分发。仅记录本仓库相对于上游的变更。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [0.4.5] - 2026-06-13

> 主题：**玻璃主题拖动卡顿真正解决 + 跨平台兜底 + 体验细节修正**。
>
> 0.4.4 通过原生 `data-tauri-drag-region` 修复了扁平主题（ink/paper）的拖动卡顿，
> 但玻璃主题（mist/dusk）实测仍卡——根因不在拖动层而在玻璃成像方式：
> 0.4.2 起 mist/dusk 走 DWM `apply_acrylic`，DWM 每帧实时高斯模糊「窗口背后的桌面 +
> 其他窗口」，高速拖动时 GPU 与桌面合成器持续高负载。本次改用 `apply_mica`
> （DWM 仅在窗口显示时采样桌面壁纸一次，拖动期间零重采样），与 0.4.1 mica 主题
> 同等的流畅手感由此恢复。

### 修复

- **玻璃主题（mist/dusk）拖动卡顿真正解决**：将 Win11 玻璃主题的 DWM vibrancy 从
  `apply_acrylic` 切换为 `apply_mica`。技术差异——
  - acrylic：DWM 实时高斯模糊「当前窗口背后的桌面 + 其他窗口」，每帧重采样，开销
    随显卡驱动浮动，是 0.4.2~0.4.4 玻璃主题拖动卡顿的根因。
  - mica：DWM 仅在窗口显示时**采样桌面壁纸一次**，窗口拖动期间不重新采样。代价是
    mica 不接受 tint，颜色调由前端 CSS 半透明表面层（mist 雾绿 / dusk 黄铜紫）提供。
  - `set_theme` 与 `window_manager` 重新应用 vibrancy 的两条路径都走 mica。
- **Win10 玻璃主题降级兜底**：mica 仅 Windows 11（build ≥ 22000）支持。Win10 上玻璃
  主题（mist/dusk）改为不透明实色背景；前端通过 `get_vibrancy_capability` 查询并
  挂 `no-vibrancy` class，触发 `mist.css` / `dusk.css` 中的实色 fallback 规则。
  能力查询走模块级缓存 + localStorage 兜底，**首帧前同步可得**，避免 Win10 用户
  切换玻璃主题时出现「透明窗口直透桌面」的闪烁。
- **`is_dark` 在 set_theme 与 re-apply 路径口径一致**：原 `window_manager` 重新
  应用 vibrancy 时仅读系统主题，与 `set_theme` 优先读 `app.color_mode` 不一致。
  用户在设置里强制覆盖系统色模式（如 light + 系统暗色）时，隐藏-再显示窗口会
  出现 mica 浅暗变体闪烁；现统一为 `set_theme` 的优先级。
- **表情面板 fallback 同步**：`EmojiPanel.tsx` 的 `FALLBACK_GROUPS`（`fetch
  /emoji-data.json` 失败时的兜底）此前仍含 0.4.4 已替换的 ZWJ 组合 emoji 与
  「键盘」中文字面量，与正式 JSON 不一致。现完全同步，并加注释要求未来同步修改。

### 改进

- **`is_win11 = build >= 22000` 阈值收敛**：抽 `supports_mica()` 函数到 `ui_cmd.rs`
  作为唯一权威定义点，`set_theme` / `window_manager` re-apply / `get_vibrancy_capability`
  三处共用，避免阈值散落与未来漏改。
- **代码精简**：删除 `glass_tint` 死函数（acrylic 路径已弃用，mica 不接受 tint，无
  消费者）；`window_manager` re-apply 的 `lock().unwrap()` 改为 `if let Ok` 兜底，
  与全仓持锁风格一致，消除锁中毒时二次 panic 风险。
- **`backdrop-filter` 上限统一**：`file-transfer.css` 的 `.wt-fullscreen-editor`
  （30px → 16px）与 `.wt-context-menu`（20px → 16px）模糊半径下调到 ≤ 16px，与
  玻璃主题表面约束保持一致，对低端 GPU 更友好。
- **CHANGELOG 链接补全**：底部 reference link 区段补齐 v0.4.1~v0.4.4，确保历史
  版本的标题链接均可点击跳转 GitHub Release。

### 兼容性

- **Windows 11**（build ≥ 22000）：玻璃主题（mist/dusk）使用 mica，拖动跟手零延迟。
- **Windows 10**：玻璃主题降级为不透明实色背景（CSS fallback token 已就绪），
  仍可读、仍流畅；如需 acrylic 的实时桌面透出效果需停留在 Win11 平台。该取舍
  换来的是 0.4.1 同等流畅的拖动手感。
- 无数据变更，从 v0.4.4 升级不影响任何用户数据与设置。

### 已知限制

- mica 不接受 tint，mist 的雾绿与 dusk 的黄铜紫色调完全由前端 CSS 半透明表面层
  提供。在某些壁纸下两个主题的 DWM 底层可能视觉接近，但 CSS 表面色仍可清晰区分。

## [0.4.4] - 2026-06-13

> 主题：**拖动卡顿真正修复 + 表情包体验优化 + 托盘文案修正**。

### 修复

- **窗口拖动卡顿（真正解决）**：v0.4.3 尝试通过移除 CSS `backdrop-filter` 修复拖动卡顿，但实测后所有主题（含扁平的 ink/paper）仍卡，证明根因不在玻璃模糊。重新定位后发现整个 header 顶栏使用 CSS `-webkit-app-region: drag` 拖动——这在 Tauri + WebView2 透明窗口下会走 WebView 命中测试 + 跨进程 IPC，是已知的拖动延迟源。本次改用原生 `data-tauri-drag-region`：在 header 顶行铺一层透明拖动层（自身带原生拖动属性），按钮 / 标题 / 搜索框以更高 z-index 浮于其上，点击空白即触发系统级 `WM_NCLBUTTONDOWN`，跟手零延迟。
- **表情面板「人物」组拆字显示**：v0.4.2 的 `emoji-data.json` 「人物」组使用了大量 ZWJ（零宽连接符）组合 emoji（如 `🧑‍💻 🧑‍🔧 👨‍🚀` 等职业组合），这些 ZWJ 序列在部分 Windows emoji 字体上不被完整支持，被拆开渲染成「人 + 物件」两个独立图标，并出现空白格。本次将「人物」组全部替换为单码位、各 Windows 版本均稳定显示的基础人物 emoji（`👶🧒👦👧🧑👮👷💂🕵️🤴👸🥷🧙🧚🧛🧜🧝🧞🧟` 等），「表情」组的 `😵‍💫` 也替换为同义单字符 `🥴`，彻底消除拆字与空白。
- **系统托盘菜单遗留文案**：托盘右键菜单仍显示「退出 贴汁」（上游旧名），改为「退出 喜鹊」。

### 改进

- **表情包默认开启**：将 `emoji_panel_enabled` 默认值从关改为开（前端 `useState(true)` + 后端 `seed_defaults` 写入 `'true'`，使用 `INSERT OR IGNORE` 仅对未设置过的用户生效，主动关过的老用户选择继续被尊重）。新装用户首次启动即可在顶栏看到表情包入口。

### 兼容性

- 仅 Windows。无数据变更，从 v0.4.3 升级不影响任何用户数据与设置。

## [0.4.3] - 2026-06-13

> 主题：**性能修复尝试**。本次尝试通过移除 `#root` 上的 CSS `backdrop-filter` 修复 v0.4.2 引入的玻璃主题拖动卡顿，但实测后所有主题仍卡，**未真正解决问题**。真正的拖动卡顿修复见 v0.4.4。

### 修复（部分有效）

- 移除 `#root` 上冗余的 CSS `backdrop-filter`：玻璃主题（mist / dusk）的模糊不再由前端再叠加一层，统一交给后端 DWM acrylic 渲染。这一改动本身合理（消除了一处冗余 GPU 开销），但并非拖动卡顿的真正根因——实测后所有主题（含扁平的 ink / paper）仍卡，故 v0.4.4 改用原生 `data-tauri-drag-region` 才彻底解决。

### 兼容性

- 仅 Windows。无数据变更，从 v0.4.2 升级不影响任何用户数据与设置。

## [0.4.2] - 2026-06-13

> 主题：**主题系统重构**。将原有 6 套主题与整套「主题商店」重构为 4 套全新主题（ink 墨玉 / paper 宣纸 / mist 晨雾 / dusk 暮山），统一低饱和暖偏移配色，彻底避开「AI 蓝」。
>
> 数据兼容：老用户旧主题值在启动时按权威映射表自动无缝迁移（不白屏、不重置其他数据）。

### 新增

- **四套全新主题**：`ink`（墨玉·默认·扁平，玉石 petrol 绿）、`paper`（宣纸·扁平，陶土赤陶）、`mist`（晨雾·玻璃·浅，雾绿）、`dusk`（暮山·玻璃·深，黄铜）。每套主题在浅 / 暗两种模式下均提供完整变量族与 `--accent-soft` / `--accent-glow` / `--danger` / `--success` / `--sensitive` / `--sensitive-soft` 语义状态色。
- **统一选中签名元素**：左缘 3px 强调色光脊、操作图标徽章点亮、选中辉光（`--card-selected-shadow`），四套主题观感一致。
- **`theme-glass` 语义 class**：玻璃主题（mist / dusk）统一标记，CSS 与组件不再硬编码具体主题名。

### 改进

- **默认主题统一为 ink**：前端 `DEFAULT_THEME` 与后端启动兜底均为 `ink`，消除启动主题不一致与闪烁。
- **玻璃主题成像**：mist / dusk 经 DWM acrylic + CSS `backdrop-filter`（≤16px）叠加成像；在系统「减少透明度」时降级为不透明实色背景。
- **能力按主题收敛**：自定义背景与表面透明度控件仅对玻璃主题（mist / dusk）开放，扁平主题（ink / paper）不再渲染相关控件。
- **首帧不白屏**：迁移过程首帧即在根节点应用默认 `theme-ink` class，保证根容器始终持有恰好一个有效主题 class。

### 移除

- **主题商店（theme-store）**：彻底移除整套主题商店 feature（组件 / hooks / API / 面板 CSS）及全部残留引用（含 `web_ui.rs` 内嵌 CSS、`VITE_API_BASE_URL` 入口、`tiez_store_css_*` 缓存、i18n 文案）。
- **5 套旧主题**：`retro` / `sticky-note` / `mica` / `acrylic` / `sakura` 不再可见可选；其旧主题值在启动时按权威映射表归一为新主题（`mica`/`sakura`→`mist`、`acrylic`→`dusk`、`retro`→`ink`、`sticky-note`→`paper`、`store-*`/未知→`ink`）。

### 稳定性与测试

- 为主题迁移完备性 / 幂等性、class 与玻璃判定对齐、能力收敛、前后端玻璃判定一致、默认主题一致、无残留扫描等核心正确性属性建立 **7 条可执行属性测试**（前端 fast-check + 后端 Rust），并新增主题 CSS 语义变量完备性、启动迁移写回行为等单元测试。
- 修复 `normalizeThemeId` 经原型链访问 `LEGACY_THEME_MAP` 的缺陷（`valueOf` / `toString` 等键误判），改用 `hasOwnProperty` 守卫，仅匹配自有属性。

### 兼容性

- 仅 Windows。沿用既有内部兼容标识符（localStorage `tiez_` 前缀等）保持不变，确保 v0.4.1 数据继续可用。

## [0.4.1] - 2026-05-30

> 主题：**稳定性 + 体验 + 界面升级**。在 v0.4.0 改名迁移的基础上，巩固迁移与自启动链路，新增多项剪贴板使用增强，统一界面观感，并建立属性测试与 CI 测试体系。
>
> 数据 100% 无损：从 v0.4.0 升级不重置任何用户数据，既有快捷键行为保持不变。

### 新增

- **条目快速打标签**：选中一个或多个条目按 `T`，在条目上叠加浮动输入框即时打标签，无需打开标签管理面板。空内容或纯空白会被忽略，已有同名标签不重复添加。
- **数字快捷粘贴 `Ctrl+1~9`**：主面板可见时按 `Ctrl+数字` 直接粘贴当前可见列表的第 N 个条目（按搜索/过滤后的顺序计），粘贴后自动隐藏面板。主面板隐藏时按键透传给前台应用、不拦截。可在设置中开关。
- **敏感内容快速标记**：选中条目按 `S` 一键打上保留标签 `__sensitive__`，列表中以色块与图标视觉强调；支持自定义触发键。
- **Win+V 默认唤起 Magpie**：开箱即用按 `Win+V` 即可呼出 Magpie 剪贴板面板（默认接管系统剪贴板历史快捷键，与默认主快捷键 `Alt+C` 并存）。无论系统剪贴板历史是否开启均可用；可在「设置 → 剪贴板」关闭接管以恢复系统 `Win+V`（恢复需重启资源管理器）。接管被其他应用（PowerToys / Ditto）占用时会给出中文提示并指明来源。
- **复制诊断信息**：设置「反馈」旁新增「复制诊断信息」按钮，一键复制最近日志、系统信息与设置摘要到剪贴板（自动脱敏密码 / token / URL 参数，全程不联网）。
- **图片加入表情包**：图片条目右键可「添加到表情包」存入用户表情库。表情库默认为空，完全由用户自行添加（不随包预置内置表情）。可在「设置 → 常用 → 表情包开关」开启顶部表情入口。
- **快捷键作用域分离**：每个快捷键可设为「全局 / 仅应用内 / 仅后台」，应用内快捷键不再污染全局；缺省按「全局」兜底，老用户行为零回归。
- **卡片密度切换**：列表支持「紧凑 / 标准 / 宽松」三档密度。
- **云同步教程**：内置 MQTT 与 WebDAV 同步教程（`docs/` 下，便携包亦随附），「查看教程」改指向本仓库 GitHub，移除失效的飞书链接。

### 改进

- **设置面板重组**：归并为「常用 / 同步 / 高级」三大分组并支持 tab 切换，所有设置项 ID 保持不变；首次升级弹一次性说明。
- **空状态与 Toast 统一**：搜索无结果 / 历史为空 / 标签下无条目均配中英文文案与图标；复制成功 / 失败 / 网络错误统一走同一 Toast 组件。
- **设置分组图标**统一为 lucide 风格，不再混用 emoji。
- **更新检查错误中文化**：DNS / TLS / 通用错误分类为中文提示，不再抛出英文异常原文。
- **启动速度优化**：schema 检查异步化、窗口骨架先行显示、后台服务并行启动。
- **README 升级**：中英文同步，定位从「剪贴板工具」过渡到「轻量信息中枢」。

### 修复

- **复制以空格 / Tab 缩进开头的内容不再变空**：捕获判空仅依据原始长度，纯空白与带缩进的代码片段完整保留。
- **重复内容合并不再丢标签**：再次复制已打标签的内容时，标签取并集保留，使用次数累加，置顶状态保留。
- **便携版开机自启动失效修复**：自启动统一交由 `tauri-plugin-autostart` 管理（移除注册表直写），便携版移动目录后仍能开机启动；缺 `data` 目录时降级为标准模式。
- **迁移更稳健**：`com.tiez → app.magpie` 迁移改为「临时目录 + 同卷原子重命名」，失败自动回滚并降级使用旧数据启动、可下次重试，迁移过程写入可见日志。
- **固定窗口模式滚轮穿透修复**：悬停在 Magpie 窗口上滚动时作用于自身列表，不再穿透到下层应用。
- **启动时不再闪现不透明方框**：透明窗口改为创建时隐藏、待毛玻璃 / 透明效果应用完成后再显示，消除便携版启动瞬间一闪而过的白色方框。
- **高级设置「搜索应用」输入框显示不全修复**：该侧栏搜索框误用了为放大镜图标预留 36px 左内边距的全局样式，在窄侧栏下把占位文字「搜索应用」截断为「搜索应」；现已恢复正常内边距、加宽侧栏默认宽度并限制拖拽最小宽度。
- **多屏显示位置与图层**复测修复；`datapath.txt` 指向的盘符不存在时回退默认目录并记录原因。
- **卸载体验**：卸载时若 Magpie 仍在运行，交互卸载提示先关闭、静默卸载走优雅关闭再兜底结束。
- **Panic 兜底**：全局 panic 写入日志，主线程崩溃尝试数据库落盘。

### 稳定性与测试

- 为剪贴板捕获 / 去重、标签合并、迁移、快捷键作用域、Win+V、自启动、诊断脱敏等核心逻辑建立 **12 条可执行的正确性属性测试**（Rust proptest + 前端 fast-check，各 ≥100 次随机迭代）。
- 新增前端单元测试、Playwright 端到端用例（含数据迁移），以及 criterion 基准测试套件与大列表实测脚手架。
- CI 接入 `Swatinem/rust-cache` 加速，并运行单元测试、端到端测试与属性测试。
- `richTextSnapshot` 缓存加 LRU 上限、`sensitive_align` 队列收口，减少长时间运行的内存增长。

### 兼容性

- 仅 Windows。沿用 v0.4.0 的内部兼容标识符（`tiez.log`、`<!--TIEZ_RICH_IMAGE:`、`tiez-sync`、localStorage `tiez_` 前缀、MQTT `tiez/tiez_` 前缀）保持不变，确保 v0.4.0 数据继续可用。
- 构建未启用 `opt-level = "z"`，保持运行性能。

### 已知限制

- 新版主题截图待重新截取。

## [0.4.0] - 2026-05-27

> ⚠️ **重大变更**：v0.4.0 是改名版本。原 TieZ 本仓库自此以 **Magpie** 名义独立维护。
>
> 老用户首次升级 v0.4.0 时数据会**自动迁移**（`%APPDATA%\com.tiez\` → `%APPDATA%\app.magpie\`），旧目录保留作为安全网，确认新版本工作正常后可手动删除。

### 重大变更：项目改名为 Magpie

- **项目正式更名为 Magpie**（喜鹊）。原名 TieZ 来自上游 jimuzhe/tiez-clipboard，本仓库自 v0.4.0 起以 Magpie 名义独立维护。
- **GitHub 仓库**从 `Duojiyi/tiez-clipboard` 重命名为 `Duojiyi/magpie`，老 URL 由 GitHub 自动 301 重定向。
- **GitHub 仓库**已脱离 fork 关系，作为独立项目维护。
- **应用 identifier** 从 `com.tiez` 改为 `app.magpie`。这意味着默认数据目录从 `%APPDATA%\com.tiez\` 变为 `%APPDATA%\app.magpie\`。
- **数据自动迁移**：首次启动 v0.4.0 时，旧目录 `com.tiez` 中的数据库、日志、设置会被自动复制到新目录 `app.magpie`。旧目录保留作为安全网。
- **自启动注册表项**从 `TieZ` 切换到 `Magpie`，旧值在切换时自动清理。
- **可执行文件名**从 `tiez-app.exe` 改为 `magpie.exe`；安装包名从 `TieZ_x.x.x_x64-setup.exe` 改为 `Magpie_x.x.x_x64-setup.exe`。
- **NSIS 卸载脚本**保留对旧名 (`TieZ` / `tiez-app` / `tie-z`) 的兼容清理，确保从老版本卸载升级链路无损。

### 内部不变

为保证用户已有数据可用，下列内部标识符**保持不变**：
- 数据库内 HTML 富文本回退 marker (`<!--TIEZ_RICH_IMAGE:` 等)
- WebDAV 同步路径默认值 `tiez-sync`
- 日志文件名 `tiez.log`
- localStorage 前缀 `tiez_xxx`
- MQTT topic 默认前缀 `tiez/tiez_xxx`、client_id 默认前缀 `tiez_pc_xxx`

如需彻底清理这些内部标识符，可在更未来的版本中做配套迁移。

### 0.3.x 累积变更（基线说明）

- **检查更新**指向本仓库 GitHub Releases（静态 `latest.json`），不再请求上游官网域名 `tiez.name666.top`。
- **公告/心跳** (`useAnnouncements`) 已禁用，不再向上游域名发送启动 ping。
- **主题商店**：默认 API 基址置空，未配置 `VITE_API_BASE_URL` 时不向任何域名发请求。商店入口在外观设置组中条件渲染（默认隐藏）。商店面板加中文友好「暂未启用」提示。
- **启动期主题处理**：用户旧设置中的 `theme: store-xxx` 在商店未启用时静默回退到默认主题 `mica`。
- **新增「启动时检查更新」开关**：默认开启，关闭后启动期不再向 GitHub 发请求；版本号旁的按钮始终可用，用于手动检查。
- **设置面板「官网」按钮** 改为打开本仓库 Releases 页面。
- **设置面板「反馈」卡片** 改为打开 GitHub Issues 页面，不再复制邮箱到剪贴板。
- **Tauri opener 白名单** 调整为本仓库相关地址（移除 `tiez.name666.top` 与 `jimuzhe/tie-z`）。
- **检查更新失败** 时按钮上显示错误详情（前 120 字符），便于无 devtools 的便携版定位问题。
- **Issue 模板 `config.yml`**：移除上游官网/赞助链接，新增 Latest Release 与 Upstream Project 入口。
- **便携版构建脚本** `scripts/build-portable.ps1` 与 `npm run build:portable`。
- **GitHub Actions** `release.yml` 重写：tag push 后一次性出 nsis、msi、portable zip 与 `latest.json`。
- 移除 6 处来自上游的 `[THEME DEBUG]` 调试 `console.log`。

### 包含的上游 PR 修复（自 v0.3.5 起）

- **PR [#87](https://github.com/jimuzhe/tiez-clipboard/pull/87)** 修复"固定窗口模式下点击标签管理后无法粘贴"。来自 [@Gao-Qian-Long](https://github.com/Gao-Qian-Long)。
- **PR [#103](https://github.com/jimuzhe/tiez-clipboard/pull/103)** 修复"窗口隐藏时 GPU 仍持续占用约 5%"。来自 [@Roxy-0304](https://github.com/Roxy-0304)。

## [0.3.8] - 2026-05-27

### 改进

- **新增「启动时检查更新」开关**：默认开启（与历史行为一致）。关闭后应用启动不再向 GitHub 发更新请求；版本号旁的按钮始终可用，用于手动检查。
- **主题商店面板**：未配置 `VITE_API_BASE_URL` 时显示中文友好提示「主题商店暂未启用」，不再是冷冰冰的空列表/加载失败。
- **主题商店入口**：未启用时不在外观设置组中渲染按钮，避免误点。
- **启动期 store-theme 处理**：用户保存的主题为 `store-xxx` 但商店未启用时，静默回退到默认主题（`mica`），避免应用启动时反复尝试拉取已下线的主题资源。

### 修复

- 移除 6 处来自上游的 `[THEME DEBUG]` 调试 `console.log`（涉及 `useSettingsInit.ts`、`AppearanceSettingsGroup.tsx`、`App.tsx`），减少 Tauri 内核日志噪声。

## [0.3.7] - 2026-05-27

### 改进

- **检查更新失败**时按钮上会显示错误详情（前 120 字符），便于无 devtools 的便携版/release 版定位问题。错误提示自动 8 秒后清除。

## [0.3.6] - 2026-05-26

### 变更

- **检查更新**改为指向本仓库 GitHub Releases（静态 `latest.json`），不再请求上游官网域名 `tiez.name666.top`。
  - 应用内"检查更新"按钮拉取 `https://github.com/Duojiyi/magpie/releases/latest/download/latest.json`。
  - 配套替换了 Tauri updater 公钥（私钥仅用于发布签名，不入库）。
- **设置面板"官网"按钮**改为打开本仓库的 Releases 页面。
- **设置面板"反馈"卡片**改为打开 GitHub Issues 页面，不再复制邮箱到剪贴板。
- **公告/心跳**（`useAnnouncements`）已禁用，不再向上游域名发送启动 ping。
- **主题商店**：默认 API 基址置空，未通过 `VITE_API_BASE_URL` 配置时不向任何域名发请求；功能保留代码，可在自部署后端时启用。
- **Tauri 配置 `opener` 白名单**调整为本仓库相关地址（移除 `tiez.name666.top` 与 `jimuzhe/tie-z`）。
- **Issue 模板 `config.yml`**：移除上游官网/赞助链接，新增 Latest Release 与 Upstream Project 入口。

### 新增

- **便携版构建脚本** `scripts/build-portable.ps1` 与 `npm run build:portable`，产物 `artifacts/portable/TieZ_<version>_x64_portable.zip`，包含 `TieZ.exe`、`data/`（触发运行时便携模式）、`LICENSE.txt`、`README*.md` 与使用说明。
- **GitHub Actions `release.yml` 重写**：tag push 后一次性出 nsis、msi、portable zip 与 `latest.json`（用于 updater）。

### 协议合规

- README/CHANGELOG 保留对上游 `jimuzhe/tiez-clipboard` 与 GPL-3.0 的署名与变更说明。

## [0.3.5] - 2026-05-26

基线版本：上游 `jimuzhe/tiez-clipboard@v0.3.4` (`ddf4060`)。

### 修复

- **修复"固定窗口"模式下点击标签管理后鼠标点击无法粘贴的问题。**
  - 原因：`TagManager` 根容器上的 `onMouseDown` 调用 `activate_window_focus`，固定窗口模式下会与全局焦点管理冲突，导致后续点击无法触发粘贴。
  - 修复：移除该 `onMouseDown` handler。
  - 来源：上游 PR [#87](https://github.com/jimuzhe/tiez-clipboard/pull/87) — 作者 [@Gao-Qian-Long](https://github.com/Gao-Qian-Long)。
  - 影响文件：`src/features/tag/components/TagManager.tsx`

- **修复窗口隐藏时 GPU 仍持续占用约 5% 的问题。**
  - 原因：窗口隐藏后 Mica/Acrylic vibrancy 效果未被清理，DWM 持续合成空透明窗口产生无谓 GPU 渲染。
  - 修复：在所有隐藏路径（关闭按钮、blur、`toggle_window`、`hide_window_cmd`）触发前调用 `window_vibrancy::clear_vibrancy`；在窗口重新显示时根据当前主题重新 `apply_mica` / `apply_acrylic`。仅作用于 Windows。
  - 来源：上游 PR [#103](https://github.com/jimuzhe/tiez-clipboard/pull/103) — 作者 [@Roxy-0304](https://github.com/Roxy-0304)。
  - 影响文件：`src-tauri/src/app/setup.rs`、`src-tauri/src/app/window_manager.rs`

### 其他变更

- README 调整：更新仓库链接指向本 fork，移除上游的赞助和社区入口，新增 fork 与协议合规说明。
- 补充 `vitest` 开发依赖以让 `tsc` 顺利通过对仓库内 `*.test.ts` 文件的类型检查。

[0.4.5]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.5
[0.4.4]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.4
[0.4.3]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.3
[0.4.2]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.2
[0.4.1]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.1
[0.4.0]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.0
[0.3.8]: https://github.com/Duojiyi/magpie/releases/tag/v0.3.8
[0.3.7]: https://github.com/Duojiyi/magpie/releases/tag/v0.3.7
[0.3.6]: https://github.com/Duojiyi/magpie/releases/tag/v0.3.6
[0.3.5]: https://github.com/Duojiyi/magpie/releases/tag/v0.3.5
