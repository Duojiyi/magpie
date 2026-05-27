# Magpie 中长期 Roadmap

> 最后更新：v0.4.0 发布后，根据 v0.4.1 计划合并后调整
> 本文档用于规划 v0.4.1 之后的版本主题。每个版本上线前会从这里挑题，单独写 vX.Y.Z-plan.md。

---

## 版本节奏

```
v0.4.0  ✅ 已发布   - 重大改名 Magpie + 数据迁移 + 全套 icon 替换
v0.4.1  📋 已规划   - Stability + UX：性能内功 / Win+V 修复 / 上游 5 bug + 4 个 UX 增强
v0.5.0  📋 本文档   - 大功能版本：Snippets + UI 升级
v1.0.0  💭 本文档   - 自带表情包库 + 跨平台体验对齐 + 第一个稳定主版本
v0.6+   💭 远期     - OCR / Smart Actions / 本地 AI（待评估）
```

---

## v0.5.0 — 大功能版本（Snippets + UI 升级）

**周期**：v0.4.1 发布后 4~6 周开发，目标 1.5 个月内发版。
**主题**：引入第一个会改变用户使用范式的新能力（Snippets），并对前端做一次有方向感的视觉升级。

### 1. Snippets（自定义文本片段）

参考 macOS Paste / Alfred Snippets，但保持隐私优先。

#### 数据模型

```sql
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY,
    keyword TEXT NOT NULL,        -- 触发关键词，如 ":eml"
    body TEXT NOT NULL,           -- 展开内容（纯文本/富文本/图片 base64）
    body_type TEXT NOT NULL,      -- 'text' | 'rich' | 'image'
    tag_id INTEGER,               -- 可选标签关联
    use_count INTEGER DEFAULT 0,  -- 使用频次（用于排序）
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);
CREATE UNIQUE INDEX idx_snippets_keyword ON snippets(keyword);
```

#### 唤起方式

**A. 主面板搜索匹配**（必做）：
- 用户在主面板搜索框输入 `:` 时切换到"snippet 搜索模式"，列表只显示 keyword 匹配的 snippet
- 双击或回车 → 直接粘贴 body

**B. 独立"Snippet 库"标签页**（必做）：
- 主面板顶栏除"全部历史"外加"Snippet"tab，纯托盘项
- 增删改查独立面板

**C. 全局打字时自动展开**（**不做**）：
- 需要全局键盘 hook，与杀毒软件冲突高
- 留到后续看用户呼声

#### UI 工作量

- 新增 `src/features/snippets/` 模块（components / hooks / api）
- TagManager 已有的标签面板可复用
- 设置面板加 Snippet 入口

### 2. UI 设计升级

**v0.4.x 一直没做 UI 重设计**。趁 v0.5.0 引入新模块（snippets / 表情包预热）的契机，做一次有目标的视觉刷新。

#### 范围

- **主面板视觉密度优化**：当前条目卡片信息略冗，调整间距/字号/分隔
- **主题统一**：mica / acrylic / 复古主题之间的视觉一致性梳理
- **关键交互动效**：复制成功 / 粘贴成功 / 标签切换的微动画（< 200ms，不影响性能）
- **空状态设计**：搜索无结果、Snippet 库为空、表情包为空时给友好的引导
- **设置面板分组**：当前 SettingsGroup 嵌套较深，重新分类减少层级

#### 不做

- 不重设计图标（Magpie icon 已 v0.4.0 定型）
- 不引入新主题色，仅在现有主题上做视觉整理
- 不动布局核心（保持紧凑窗口风格）

### 3. 设计参考

- macOS Paste / Raycast 的卡片密度
- Windows Fluent Design 的微妙阴影 / 半透明
- Linear 的设置面板分组方式

### 验收

- Snippet 列表可承载 1000+ 条，搜索 < 100 ms
- UI 升级后用户主观调研：≥ 80% 的现有用户认为"更好看了或没变差"
- 性能基线不退化（启动时间、滚动 fps、内存）

---

## v1.0.0 — 第一个稳定主版本

**周期**：v0.5.0 发布后 2~3 个月。**真正达到"日常生产力工具"标准时**才上 v1.0。
**主题**：自带表情包库 + 体验对齐高端付费工具。

### 1. 自带表情包库

> 用户原话："我们软件可以自带一些表情包，同时支持用户自己添加。这样设计有几个好处：方便跨平台使用；即使多占用一点体积也没关系。"

#### 设计

- **内置表情包**：Magpie 安装包内附 ~50~100 张精选表情包（GIF/PNG/WebP 混合）
  - 来源：开源表情包资源（如 Twemoji、OpenMoji），附带 LICENSE 注明
  - 不嵌入版权风险图（不含微信/QQ/twitter 平台特定表情）
- **用户表情**：v0.4.1 已经做过的「图片快速添加到表情包」（F4）的延续。用户从剪贴板加进来的图片
- **统一面板**：复用现有 `EmojiPanel`，分两个 tab："内置"和"我的"
- **快捷调用**：按 `Win+E`（可配置）调出独立的表情包面板，独立于主剪贴板面板

#### 体积影响

- 内置 100 张精选表情包估计 ~5~8 MB
- v0.4.0 NSIS 安装包 4.7 MB → v1.0 估计 10~12 MB
- 用户认可这个体积换取功能

#### 数据存储

- 内置表情：`<exe-dir>/emojis-builtin/`（只读，跟随 exe 升级）
- 用户表情：`%APPDATA%\app.magpie\emojis-user\`（自由增删）

#### 跨平台考虑

- 当前只发 Windows，但表情包是平台无关数据
- v1.0 同时考虑：是否要在便携版里同时打包内置表情包（增加 zip 大小但完整）

### 2. 体验对齐 Mac 付费工具

参考之前与 Paste / Raycast / Maccy 的对比，挑剩下的几项做：

- **Pinned items（置顶）**：v0.4.x 有"持久化"但没"显式置顶"
- **多剪贴板（multiple clipboards）**：复用顺序粘贴的基础概念扩展
- **历史导出**：JSON / CSV 导出用户历史

### 3. 文档完善

- 用户使用手册（中英对照）
- 内置教程引导（首次启动）
- 详细的 FAQ

### 4. 性能 / 稳定性兜底

- 1.0 前必须达成的指标：
  - 24 小时不崩溃
  - 100 万条历史不卡
  - 不会因为某个图片条目导致整个应用挂起
  - 全部命令的错误信息中英对照清晰

### 5. 主版本号承诺

v1.0 = "我可以推荐给非技术朋友"的版本。**v1.0 之前不积极推广**。

---

## v0.6+ — 远期方向（不承诺时间表）

下面这些不绑定具体版本，仅记录候选方向。每一项都需要 v1.0 后单独再评估。

### A. OCR / 图片转文字

- 选型：本地优先用 Windows.Media.Ocr（系统自带）或 `tesseract-rs`
- 触发：图片条目右键 → "OCR 提取文字" → 自动追加到剪贴板
- 拒绝外部 API（违背隐私优先）

### B. Smart Actions

- 检测到 URL → 显示"打开"按钮
- 检测到代码块 → 显示"复制为代码 / Markdown" 按钮
- 检测到颜色码 → 显示色卡和复制其他格式

### C. 本地翻译 / 摘要

- 仅当桌面端有可调用的本地小模型时启用（如 Windows Copilot Runtime）
- 不引入外部 LLM API

### D. 插件系统

- 暴露剪贴板事件总线
- 用户可写 JS 脚本扩展 Smart Actions

---

## v0.4.x 期间不变的承诺

- ❌ 不动 HTML marker `<!--TIEZ_RICH_IMAGE:`
- ❌ 不动 WebDAV 默认路径 `tiez-sync`
- ❌ 不动日志文件名 `tiez.log`
- ❌ 不动 localStorage 前缀 `tiez_xxx`
- ❌ 不动 MQTT topic `tiez/tiez_xxx`、client_id 前缀 `tiez_pc_xxx`

这些是 v0.4.0 立下的兼容承诺。**v0.5.0 minor bump 时**可以一次性配套迁移这些内部标识符，但需要在 CHANGELOG 显式声明并出迁移代码（参考 v0.4.0 的 `perform_migration_v040`）。

---

## 上游 jimuzhe/tiez-clipboard 同步策略

- 持续监控上游 issue 和 PR
- 与 Magpie 相关、价值高的 bug 修复 → cherry-pick + 在 CHANGELOG 标注作者
- 与 Magpie 不相关的（macOS 专用、Linux 等）→ 跳过
- 重大架构改动 → 不跟，因为我们已经独立维护

---

最后更新：v0.4.0 发布后，结合用户对 v0.4.1 加重 + 表情包 + UI 升级方向的反馈调整。
