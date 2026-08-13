# 更新日志

本仓库 fork 自 [`jimuzhe/tiez-clipboard`](https://github.com/jimuzhe/tiez-clipboard)，依据 GPL-3.0 协议二次分发。仅记录本仓库相对于上游的变更。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [0.7.1] - 2026-08-13

> 修复 v0.7.0 的两处缺陷，其中一处影响 Windows。建议所有平台升级。

### 修复

- **临时粘贴后文件条目被重复记入历史（影响 Windows）**：v0.7.0 改进了多文件剪贴板的还原（此前多个文件会被拼成一条无效路径），但还原时漏了「这是我们自己写的」标记，导致还原动作被剪贴板监控当成用户的新一次复制——该文件条目会被重新推到历史顶部或产生重复条目。现已补回该标记。
- **固定窗口模式下粘贴可能落到 Magpie 自己窗口（macOS / Linux）**：粘贴按键是异步投递给「投递那一刻的最前台应用」的，而窗口在按键送出后立刻被显示回来，重新抢走了焦点（macOS 上取消隐藏应用本身就会激活它）。现在会等待按键送达后再恢复窗口，并且只在确实隐藏过时才恢复——用快捷键从别的应用触发粘贴时，不再抢走你正在输入的窗口的焦点。
- **macOS 点击 Dock 图标后没有窗口**：粘贴后应用被隐藏，点 Dock 图标虽会取消隐藏，但主窗口仍处于隐藏状态。现已正确恢复窗口。
- **Linux 降级轮询的两处浪费**：显示服务不可用而降级为文本轮询时，每轮都会重建剪贴板连接、并把上次内容哈希清零（导致每轮首次读取都被误判为「有变化」而空跑一次捕获）。两者均已修正。

## [0.7.0] - 2026-08-13

> 主题：**macOS 与 Linux 正式支持**。此前这两个平台「能装能启动」，但剪贴板核心是一组空实现——复制图片、复制文件、粘贴富文本都会报成功却什么都没发生。本版把它们换成了真实现，补齐了来源识别、静态加密与动图保真，并首次提供 Linux 安装包。Windows 行为不变。

### 新增

- **Linux 支持**：首次提供 `.deb` 与 `.AppImage` 安装包（x86_64）。
- **macOS / Linux 剪贴板真实现**：文本、HTML 富文本、图片、文件列表的读写全部接通（macOS 走 NSPasteboard，Linux 走 X11），替换掉原先返回「成功」却无任何行为的空实现。GIF 保留动画（此前会被重编码成静态首帧），应用私有格式在同平台内往返保留。
- **来源应用识别**：macOS 通过 NSWorkspace、Linux 通过 X11 活动窗口 + 进程信息识别复制来源。此前每条历史的来源都是占位值，导致来源图标、按应用清理规则、以及判断内容是否来自 Office / 表格 / 截图工具的启发式全部失效。
- **敏感数据静态加密**：macOS / Linux 没有 DPAPI 对应物，此前 API 密钥、MQTT 凭据、云同步口令与敏感标签条目都是明文落库。现改为 XChaCha20-Poly1305 加密，密钥以 0600 权限存放在数据目录内并随数据目录一起迁移。
- **事件驱动的剪贴板监听**：macOS 用 NSPasteboard 变更计数、Linux 用 X11 XFixes 选区事件。此前是每 500 毫秒轮询一次纯文本，只复制图片或文件时**完全不会进历史**。若显示服务尚未就绪（开机自启常见），会临时降级为文本轮询并持续重试升级回事件驱动。
- **模拟粘贴**：改用跨平台输入注入。此前 Linux 上执行的是 macOS 专用的 `osascript`，**必然失败且没有任何提示**；macOS 上未授予辅助功能权限时也是静默失败。现在失败会记录明确原因与操作指引。

### 修复

- **富文本粘贴在 macOS / Linux 上连纯文本都粘不出来**（只要历史条目是富文本，点击后就毫无反应）。
- **窗口无法输入**：非 Windows 上窗口被设为不可聚焦（该设置只在 Windows 上与 `WS_EX_NOACTIVATE` 配套使用），导致搜索框打不进字、方向键无效。
- **粘贴落到自己窗口**：补上了粘贴前的焦点交还。macOS 采用应用级隐藏——该平台的激活是按应用而非按窗口的。
- **Linux 上「打开文件夹 / 打开文件 / 在文件管理器中显示」点了没反应**：这三个命令原本只有 Windows 与 macOS 分支，Linux 下函数体等价于直接返回成功。
- **Linux 被误判为 Windows**：平台判断改由后端提供（UA 字符串无法区分二者），此前 Linux 会显示 Win+V 接管等一批在该平台无效的开关。
- **Linux 玻璃主题透出桌面**：该平台不支持 vibrancy，现在正确降级为不透明背景。
- **跨平台数据显示为乱码**：Windows 上用 DPAPI 加密的值在 macOS / Linux 上会被原样当作明文展示（形如 `dpapi:AQAAA...`），现改为如实报告「无法解密」。
- **上传文件名消毒依赖 Windows 路径语义**：`\` 在 Unix 上不是路径分隔符，Windows 风格的文件名在 macOS / Linux 上不会被规约成末段。
- **临时粘贴会破坏剪贴板**：快照逻辑原本只在 Windows 上捕获图片与文件，还原时会把用户原本的图片剪贴板覆盖成文本。

### 工程

- CI 对 **macOS 与 Linux 均改为阻断式**编译检查并运行测试。此前 macOS 检查是非阻断的，导致 v0.6.1 在 CI 报绿的情况下实际 macOS 编译已损坏。
- 跨平台剪贴板与输入后端**在 Windows 上也参与编译**，使唯一可本地构建的平台能对其做类型检查——这是前两次发布被「空实现签名漂移」搞挂后的针对性措施。

### 平台差异（macOS / Linux）

这些能力依赖 Windows 专有机制，在其它平台上按下述方式工作：

- **键盘导航与数字快捷粘贴**：面板呼出时会获得焦点，方向键选择、Enter 粘贴、Esc 关闭与 `Ctrl`/`Cmd`+数字快捷粘贴均由面板自身处理；Windows 上那种「面板隐藏时也能响应」依赖低级键盘钩子，其它平台不提供。
- **Wayland**：模拟粘贴需要 X11 或 XWayland；纯 Wayland 会话下内容仍会写入剪贴板，手动按 `Ctrl+V` 粘贴即可。
- **边缘停靠**、**表情拖放**、**「用应用打开」的应用列表与图标**：仍为 Windows 专属。

### 兼容性

- Windows 行为与 v0.6.2 完全一致。
- 数据库、云同步协议、端到端加密格式均未变更，可与旧版本共存同步。
- macOS / Linux 首次运行会在数据目录生成 `local.key`（敏感数据的本机加密密钥）。**更换数据目录时它会随数据一并迁移**；若目标目录已属于另一份加密数据，迁移会在移动任何文件之前中止并提示，不会造成数据不可读。

## [0.6.2] - 2026-08-12

> 修复 v0.6.1 的 macOS 发布构建失败。功能与 v0.6.1 完全相同，Windows 用户无需升级。

### 修复

- **macOS 构建**：v0.6.1 给 `get_named_clipboard_formats` 增加了「取数据前先按格式名过滤」的参数（WPS 崩溃修复的核心），但只改了 Windows 实现，非 Windows 桩函数仍是旧签名，导致 macOS 编译失败、v0.6.1 只产出了 Windows 安装包。
- **工程**：CI 的 macOS 编译检查此前是非阻断的，这类「桩函数与真实实现签名漂移」的问题只能等到发布时才暴露（已连续两次）。现改为阻断，PR 阶段即可拦截。

## [0.6.1] - 2026-08-12

> 主题：**修复一批从上游继承的用户报告问题**，外加两条会导致数据不可恢复的本地缺陷。上游（`jimuzhe/tiez-clipboard`）自 v0.3.4 起已基本停止更新，其 issue 区累积的多个 bug 在本仓库仍然存在，本版逐一核查并修复。

### 修复（数据安全）

- **端到端加密口令可能被静默清空**：口令在本机以 DPAPI 加密存储，但解密失败时读取层会把它当作空字符串返回。重装系统、更换 Windows 账户或迁移配置目录后，界面会显示「未设置口令」并呈现空输入框，用户重新输入新口令即导致云端已加密数据永久不可读。现在能区分「未设置」与「本机无法读取」，给出明确提示并引导输入原口令；口令输入框也不再会用空值覆盖已有密文。
- **崩溃后无法启动、只能卸载重装**：release 构建使用 `panic = "abort"`，崩溃时不执行任何析构，SQLite 可能留下无法打开的文件；此前数据库打开失败会让启动直接终止，既没有托盘也没有窗口。现在会识别真正的文件损坏（`SQLITE_CORRUPT` / `SQLITE_NOTADB`），把损坏文件改名保留后以新库启动（绝不删除，便于事后取回）；迁移失败、磁盘满等非损坏原因则原样报错，不会误动数据。
- **托盘构建失败导致进程退出**：托盘菜单与图标的构建改为失败即降级，不再中止启动。
- **剪贴板可能被本进程独占**：`set_clipboard_files` 在内存分配失败时会跳过 `CloseClipboard`，使全系统复制粘贴失效直到进程退出。

### 修复（上游用户报告问题）

- **[上游 #147] 开启「捕获带格式的文本」后 WPS 崩溃**：捕获时会对枚举到的**每一个**剪贴板格式请求数据，其中包括 `Embed Source`、`Object Descriptor` 等 OLE 延迟渲染格式——请求它们会迫使 WPS 同步序列化嵌入文档并崩溃。虽然早有格式黑名单，但过滤发生在取数据**之后**。现在过滤前置到请求数据之前。同时 WPS 文字（`wps.exe`）不再被误判为电子表格（此前会触发三轮全量枚举并保留其全部私有格式），`OpenClipboard` 也增加了短重试。
- **[上游 #122] 从 Excel 复制整列，粘贴后每行之间多出空行**：HTML 转纯文本时对块级标签的开标签与闭标签都插入换行，导致每个单元格后多一个空行。现在改为在 HTML 路径统一移除空行；同时单元格以制表符分隔（此前多列会被拆成多行），连续空单元格的列位也得以保留。
- **[上游 #150] 富文本末尾空格丢失**：纯文本路径此前已修复，但富文本路径仍会逐行 trim，而该结果既入库也用于回写剪贴板。现在当 HTML 派生文本与原始纯文本仅有空白差异时，以原始纯文本为准。
- **[上游 #128 等] WebDAV「用一两天就失败，必须换同步文件夹」**：定位到四条独立成因并逐一修复——整设备快照此前绕过 blob 外置，把所有图片以 base64 内联进单个 JSON，随历史增长直到上传无法在超时内完成；HTTP 改为分别设置连接与读取超时，不再用一个总时限卡住大文件；`head.json` 损坏后此前只有 404 才触发重建（换文件夹恰好制造 404，这正是那个变通办法奏效的原因），现在损坏即重建；单个损坏的操作文件或缺失附件此前会中断整轮同步且游标永不前进，现在会跳过、推进并在同步状态中报告。
- **[上游 #146 / #140 / #126] 多设备同步不完整、另一台设备收不到数据**：同步游标此前不区分账户，切换 WebDAV 账户或目录后旧游标会压制新账户的数据；设备重装后机器码不变但序号归零，对端会永久跳过它发布的全部内容。现在游标按账户隔离，并引入安装标识以识别重装并重新同步。
- **[上游 #1] 鼠标滚轮穿透到下层应用**：此前只在固定窗口模式下转发滚轮，而非固定模式同样不获取焦点、成因相同。现在两种模式都处理，并补齐水平滚轮与 Ctrl / Shift 修饰键。
- **[上游 #143] 在 Windows 终端中方向键未被拦截**：每次按键都会做跨进程输入法状态查询，前台应用繁忙时可能超过系统对低级钩子的时间限制，导致拦截整体失效。该查询结果现在按前台窗口做短时缓存。
- **[上游 #114] AI 助手连接失败只显示「error」**：后端早已返回具体原因，但前端将其丢弃。现在会展示具体错误。

### 安全

- 修复 AI 服务 API Key 与 MQTT 账号密码经「设置同步」明文上传的问题（见 v0.6.0 说明，本版补充：新增的每设备安装标识同样排除在设置同步之外）。

### 兼容性

- 全部改动向后兼容，同步协议未变更；旧版本客户端与本版可共存同步。
- 升级后同步游标会重置一次（改为按账户隔离所致），首次同步可能多传输一些数据。

## [0.6.0] - 2026-08-12

> 主题：**云同步端到端加密（可选，默认关闭）**。开启后剪贴板内容在离开本机前即被加密，WebDAV 服务端只能看到密文。默认关闭，不开启则行为与 v0.5.1 完全一致。

### 新增

- **端到端加密（E2E）**：设置 → 云同步中新增「端到端加密」开关与「加密口令」。开启后，条目正文、HTML 富文本、预览文本以及外置的图片 / 长文本 blob 均在本机加密后再上传。
  - **密钥派生**：Argon2id（m=64 MiB, t=3, p=1）从口令派生主密钥，再经 HKDF-SHA256 分离出内容 / nonce / 校验三把子密钥；口令只保存在本机（Windows 上经 DPAPI 加密落盘），**绝不上传**，也不参与设置同步。
  - **加密算法**：XChaCha20-Poly1305。附加认证数据绑定「字段角色 + 条目身份」，服务端无法把某条目的预览密文搬到正文槽位，也无法在条目之间搬运密文。
  - **多设备**：首台开启的设备会在 WebDAV 上发布 `e2e.json`（仅含公开的 salt 与口令校验值），其它设备填入相同口令即可自动派生出同一密钥；口令不匹配会明确报错而不会写入乱码。
  - **失败即中止**：开启 E2E 但口令为空、口令与账户不匹配、或云端加密配置读取异常时，本次同步直接中止，**绝不降级为明文上传**。未开启 E2E 的设备若检测到该账户已加密，同样拒绝上传明文。
  - **去重保持**：相同内容加密后得到相同密文，blob 以 `sha256(密文)` 命名，因此图片 / 长文本的秒传与缓存机制不受影响，也不会因为重复复制而反复上传。
  - **无法解密时**：跳过该条目并在同步状态中提示数量，不会中断整轮同步，也不会把密文写进本地历史。

### 安全

- **修复：AI 与 MQTT 凭据曾被明文上传**：`ai_profiles`（含各家 AI 服务的 API Key）、`mqtt_username`、`mqtt_password` 虽然在本地是加密存储的，但设置同步在上传前会读取解密值，导致这些凭据以明文形式写入 WebDAV 的设置快照。现已将三者排除出设置同步范围。
  - ⚠️ **建议操作**：如果你此前使用过「设置同步」，这些凭据可能仍以明文留存在你的 WebDAV 服务器上。建议登录 WebDAV 删除旧的设置快照文件，并**轮换相关 API Key / MQTT 密码**。
  - 行为变更：升级后 AI 配置与 MQTT 账号密码不再跨设备同步，需要在各设备分别填写。

### 工程

- 移除 7 个未被实际引用的前端依赖（`react-window`、`aedes`、`express`、`form-data`、`mqtt`、`multer`、`node-fetch`），缩小产物体积与依赖面。
- 加密模块含 12 项单元测试，覆盖 nonce 派生、AAD 绑定、跨字段 / 跨条目搬运、信封格式边界与去重稳定性；封装 / 解封链路补充端到端回归测试。

### 兼容性

- 默认关闭，升级即用，不改变既有同步协议。
- 未开启 E2E 的历史明文数据可继续正常读取（加密与非加密内容可共存）。
- 开启 E2E 后**请务必牢记口令**：口令仅存于本机，一旦遗忘，云端已加密的数据将无法恢复。

## [0.5.1] - 2026-08-12

> 修复 v0.5.0 的 macOS 发布构建回归。功能与 v0.5.0 相同。

### 修复

- **macOS 构建**：为非 Windows 平台补上 `set_clipboard_rich_content` 桩函数。v0.5.0 引入的"单事务富文本写入"只在 Windows 的 `win_clipboard` 模块中定义了该函数，非 Windows 桩模块漏补，导致 macOS `cargo check` 报 `E0425` 失败、发布工作流的 macOS bundle 未能产出（Windows 包不受影响）。非 Windows 平台行为与此前一致（富文本写入在该平台为 no-op）。

## [0.5.0] - 2026-08-12

> 主题：**安全加固与稳定性专项**。基于一轮完整的前端 + 后端 + 网络面审计，修复了一批崩溃、死锁、局域网/剪贴板安全与数据完整性问题；全部改动向后兼容，未改变云同步 / 设备配对协议。

### 安全

- **局域网文件传输**：请求体大小改为有上限（`/upload` 流式 4 GiB、其余路由 16 MiB，此前为完全不限制的 DoS 面）；新增上传会话数上限与空闲超时淘汰；`/download` token 改为进程内随机、不可枚举；上传文件名与分片临时文件名分别消毒/哈希，防目录穿越与临时文件碰撞交错；下载响应 `Content-Disposition` 文件名消毒。
- **剪贴板 HTML 预览**：改用 DOMPurify 作最终净化网关，并收紧 CSP —— 移除 `script-src 'unsafe-inline'`，新增 `object-src 'none'`、`frame-src 'none'`、`base-uri 'self'`；文件传输 Web UI 的图片 `onclick` 注入面收敛。
- **MQTT 云同步**：新增入站载荷大小上限；改为有界的应用线程（此前每条消息 `spawn` 一个线程，可被刷屏拖垮）。
- **系统集成**：Windows「用应用打开」对传入参数做 PowerShell 转义，防命令注入；收窄 webview 权限，移除前端并未使用的 `fs` / `sql` 授权；默认 AI Key 不再内联进前端产物。

### 修复

- **死锁**：修复「给会话条目打标签」与「置顶 / 捕获管线」之间的 `conn`↔`session` 锁顺序反转（AB-BA），统一为全局一致的加锁次序。
- **崩溃（release `panic = "abort"`）**：清除远端可触达与 UI 热路径上的多处 `SystemTime … unwrap()`、路径 `to_str().unwrap()`、托盘 / 全局钩子安装 `expect`，改为优雅降级，避免时钟异常或环境受限时整进程被杀。
- **数据完整性**：修改数据目录前先执行 WAL checkpoint，避免最近的剪贴板条目在迁移中丢失。
- **类型筛选失效**：修正 `get_clipboard_history` 的 IPC 参数名（此前 snake_case 被后端静默忽略，导致按类型筛选失效、分页提前终止）。
- **整窗白屏**：非法 / 损坏的语言设置不再导致 `t()` 抛错白屏——新增语言白名单校验、`t()` 防御性兜底，并为所有窗口加上全局 ErrorBoundary。
- **状态残留与竞态**：窗口隐藏后重置筛选 / 搜索状态；修复标签切换与并发 AI 操作的请求竞态；补齐若干定时器 / 事件监听清理；`localStorage` 解析加健壮性防护。
- **原生剪贴板**：修复 Windows 剪贴板写入失败分支的内存泄漏；剪贴板监控锁中毒后自动恢复而非永久停摆。

### 工程

- CI 新增 macOS 编译检查任务，并加入非阻断的 `cargo fmt` / `npm audit` 信号；为安全关键函数（文件名消毒、下载 token、临时文件名、类型筛选参数契约、语言白名单、错误边界）补充回归测试。

### 兼容性

- 本次全部改动向后兼容：不改变数据库结构、云同步与设备配对协议，现有用户升级后配置与历史不受影响。
- 云同步端到端加密与 MQTT topic 提熵需前后端协同，留待后续版本单独实现。

## [0.4.6] - 2026-06-09

> 主题：**v11 液态玻璃界面 Beta + Mac Release 构建通道**。

### 新增

- **v11 液态玻璃界面 Beta**：引入全新的视觉层级、搜索区、内容卡片和底部状态栏设计，让剪贴板主界面更接近桌面软件质感。
- **Release Actions 增加 macOS universal 构建**：发布 workflow 现在会在打 tag 或手动运行时同时构建 Windows `nsis/msi` 和 macOS `app/dmg`，并通过 `tauri-action` 自动创建 GitHub Release、上传安装包与便携版、生成跨平台 `latest.json` 更新清单；Mac 包覆盖 Apple Silicon 与 Intel。

### 改进

- **macOS 构建兜底**：补齐非 Windows 平台的 Windows API stub，并将 Windows-only hook 清理、注册表、管理员重启、Win+V 接管等逻辑限制在 Windows 编译。
- **macOS 文件打开体验**：文件/文件夹打开命令在 macOS 下改用系统 `open`，文件定位使用 `open -R`。
- **macOS ad-hoc 签名兜底**：在没有 Apple Developer 证书的 GitHub Release 构建中使用 ad-hoc signing identity，降低下载包被 macOS 判定为损坏的概率。

### 兼容性

- 发布 workflow 已将 Actions token 权限恢复为 `contents: write`，并改回 `tauri-action` 自动发布：构建产物现在会作为 Release 资产对外提供，Windows 与 macOS 的 updater `latest.json` 也会自动生成并上传，自动更新通道恢复可用。
- macOS 包已进入 CI 构建通道，但剪贴板监听、系统热键与窗口焦点等平台行为仍需要真机 QA 后再标为稳定。

## [0.4.5] - 2026-05-31

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

## [0.4.4] - 2026-05-31

> 主题：**拖动卡顿真正修复 + 表情包体验优化 + 托盘文案修正**。

### 修复

- **窗口拖动卡顿（真正解决）**：v0.4.3 尝试通过移除 CSS `backdrop-filter` 修复拖动卡顿，但实测后所有主题（含扁平的 ink/paper）仍卡，证明根因不在玻璃模糊。重新定位后发现整个 header 顶栏使用 CSS `-webkit-app-region: drag` 拖动——这在 Tauri + WebView2 透明窗口下会走 WebView 命中测试 + 跨进程 IPC，是已知的拖动延迟源。本次改用原生 `data-tauri-drag-region`：在 header 顶行铺一层透明拖动层（自身带原生拖动属性），按钮 / 标题 / 搜索框以更高 z-index 浮于其上，点击空白即触发系统级 `WM_NCLBUTTONDOWN`，跟手零延迟。
- **表情面板「人物」组拆字显示**：v0.4.2 的 `emoji-data.json` 「人物」组使用了大量 ZWJ（零宽连接符）组合 emoji（如 `🧑‍💻 🧑‍🔧 👨‍🚀` 等职业组合），这些 ZWJ 序列在部分 Windows emoji 字体上不被完整支持，被拆开渲染成「人 + 物件」两个独立图标，并出现空白格。本次将「人物」组全部替换为单码位、各 Windows 版本均稳定显示的基础人物 emoji（`👶🧒👦👧🧑👮👷💂🕵️🤴👸🥷🧙🧚🧛🧜🧝🧞🧟` 等），「表情」组的 `😵‍💫` 也替换为同义单字符 `🥴`，彻底消除拆字与空白。
- **系统托盘菜单遗留文案**：托盘右键菜单仍显示「退出 贴汁」（上游旧名），改为「退出 喜鹊」。

### 改进

- **表情包默认开启**：将 `emoji_panel_enabled` 默认值从关改为开（前端 `useState(true)` + 后端 `seed_defaults` 写入 `'true'`，使用 `INSERT OR IGNORE` 仅对未设置过的用户生效，主动关过的老用户选择继续被尊重）。新装用户首次启动即可在顶栏看到表情包入口。

### 兼容性

- 仅 Windows。无数据变更，从 v0.4.3 升级不影响任何用户数据与设置。

## [0.4.3] - 2026-05-31

> 主题：**性能修复尝试**。本次尝试通过移除 `#root` 上的 CSS `backdrop-filter` 修复 v0.4.2 引入的玻璃主题拖动卡顿，但实测后所有主题仍卡，**未真正解决问题**。真正的拖动卡顿修复见 v0.4.4。

### 修复（部分有效）

- 移除 `#root` 上冗余的 CSS `backdrop-filter`：玻璃主题（mist / dusk）的模糊不再由前端再叠加一层，统一交给后端 DWM acrylic 渲染。这一改动本身合理（消除了一处冗余 GPU 开销），但并非拖动卡顿的真正根因——实测后所有主题（含扁平的 ink / paper）仍卡，故 v0.4.4 改用原生 `data-tauri-drag-region` 才彻底解决。

### 兼容性

- 仅 Windows。无数据变更，从 v0.4.2 升级不影响任何用户数据与设置。

## [0.4.2] - 2026-05-30

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

[0.4.6]: https://github.com/Duojiyi/magpie/releases/tag/v0.4.6
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
