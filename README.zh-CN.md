<p align="left">
  <img src="docs/images/logo.png" width="32" vertical-align="middle" />
  <b>轻量信息中枢 —— 让碎片化信息轻松流转</b>
</p>

---

<div align="center">
  <img src="docs/images/logo.png" alt="Magpie Hero Logo" width="300" />

  ### **STAY FAST. STAY SYNCED.**

  | VERSION | LICENSE | PLATFORM |
  | :--- | :--- | :--- |
  | [![Version](https://img.shields.io/github/v/release/Duojiyi/magpie?label=VERSION&style=for-the-badge&color=2196F3)](https://github.com/Duojiyi/magpie/releases) | [![License](https://img.shields.io/badge/LICENSE-GPL--3.0-FF9800?style=for-the-badge)](https://www.gnu.org/licenses/gpl-3.0) | [![Platform](https://img.shields.io/badge/PLATFORM-WINDOWS-f44336?style=for-the-badge)](https://github.com/Duojiyi/magpie/releases) |

  [English](./README.md) | [简体中文](./README.zh-CN.md)
</div>

---

## 关于本项目

**Magpie**（中文：喜鹊）是一款本地优先、注重隐私的 Windows **轻量信息中枢**，以高速剪贴板内核为基础。

它从剪贴板工具起步，正逐步成长为一个让碎片信息汇聚流转的安静角落——你复制、同步、打标签、再回看的内容都在这里交汇。采集文字和图片只是入口，Magpie 把你零散的内容收纳整齐、随手可取，让需要的那一条内容始终只差一个快捷键。

本仓库基于 [`jimuzhe/tiez-clipboard`](https://github.com/jimuzhe/tiez-clipboard) 在 GPL-3.0 协议下二次开发，自 v0.4.0 起以 Magpie 命名独立维护。包含若干上游未合并的 Bug 修复、隐私改进与体验优化，详见 [CHANGELOG](./CHANGELOG.md)。

> 喜鹊有"收集闪亮东西"的文化形象，恰好对应一个信息中枢的本质——把你需要的零散内容收起来，需要时随手取出。

---

<div align="center">

## 主题展示

  <sub>新版 Magpie v0.4.x 视觉系统设计预览（低饱和静谧配色 + 液态玻璃变体）。以下为设计稿，待新版界面落地后将替换为真实截图。</sub>

  <table>
    <tr>
      <td align="center"><b>墨玉 Ink Jade</b><br><sub>扁平 · 默认</sub><br><img src="docs/images/theme-ink-jade.png" width="220" /></td>
      <td align="center"><b>宣纸 Paper</b><br><sub>暖编辑</sub><br><img src="docs/images/theme-paper.png" width="220" /></td>
    </tr>
    <tr>
      <td align="center"><b>晨雾 Mist</b><br><sub>液态玻璃 · 浅</sub><br><img src="docs/images/theme-mist.png" width="220" /></td>
      <td align="center"><b>暮山 Dusk</b><br><sub>液态玻璃 · 深</sub><br><img src="docs/images/theme-dusk.png" width="220" /></td>
    </tr>
  </table>

</div>

---

## 为什么选择 Magpie

| 极速性能 | 深度工作流 | 本地隐私 | 云端流畅 |
| :--- | :--- | :--- | :--- |
| **瞬间响应**<br>Rust 核心层与原生监听器，毫秒级响应。 | **全能管理**<br>支持富文本、多色标签及 AI 协作。 | **本地优先**<br>数据完全本地化存储，敏感信息预览自动脱敏。 | **多端同步**<br>基于 WebDAV 与 MQTT 协议，剪贴板在设备间流动。 |

---

## 核心功能

### 基础体验
- **原生效率**：Tauri 2 + Rust，体积小、内存占用低。
- **智能采集**：自动记录文字、富文本 (HTML)、图片、文件和目录路径。
- **现代美学**：完整支持 Mica/Acrylic 背景效果及暗黑模式，多套主题可切。
- **贴边收纳**：自动停靠屏幕边缘，节省桌面空间且随时呼出。

### 管理与增强
- **标签系统**：自定义多色标签分类与整理。
- **表情管理**：内置 Emoji 表情库，快捷搜索与输入。
- **高级设置**：精细化控制清理规则、全局快捷键映射等。
- **隐私脱敏**：自动识别身份证、手机号、邮箱等敏感信息，预览时遮罩。

### 网络与传输
- **WebDAV 同步**：用你自己的 NAS / 坚果云，跨设备历史同步。
- **局域网传输**：同网内极速传输文件和内容。
- **秒传验证码**：手机端短信验证码瞬间同步至当前设备。
- **MQTT 协议**：轻量协议同步方案，多网络环境下高实时性。

### 效率提速
- **外部协作**：调用外部编辑器修改内容，存盘后自动写回。
- **全局搜索**：按内容、所属应用、标签或日期检索。
- **顺序粘贴**：高频办公场景设计的顺序拷贝/粘贴流程。

---

## 系统要求

| 平台 | 运行环境 | 获取格式 |
| :--- | :--- | :--- |
| **Windows** | Windows 10/11 (x64) | `.exe` / `.msi` / `.zip` (便携版) |

[**前往 Releases 下载最新版本 →**](https://github.com/Duojiyi/magpie/releases)

---

## 已知限制

### Win+V 不能直接作为主快捷键

`Win+V` 是 Windows 系统内置的「剪贴板历史」快捷键，被系统级占用。在「设置 → 主快捷键」处选择 `Win+V` 会提示「快捷键不可用」。

**临时方案**：选用 `Alt+V` / `Ctrl+Shift+V` / `Alt+\`` 等组合键作为主快捷键，体验完全等价。

**根本修复**计划在 v0.4.1 完成：检测到 Win+V 时自动通过注册表（`HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\DisabledHotkeys`）让 Magpie 接管 Win+V，并在设置面板暴露相应开关。详见 [v0.4.1 计划](./docs/v0.4.1-plan.md)。

---

## 从 TieZ 升级

如果你之前使用过 jimuzhe/tiez-clipboard 或它的 fork，安装 Magpie v0.4.0 后首次启动会自动迁移你的数据：
- 旧数据目录 `%APPDATA%\com.tiez\` 会被复制到新目录 `%APPDATA%\app.magpie\`
- 旧目录会保留作为安全网，确认新版本工作正常后可以手动删除
- 自启动注册表项会从 `TieZ` 切换到 `Magpie`，旧值会被清理

---

## 开源协议

本项目基于 [GNU GPL-3.0](./LICENSE) 协议开源。

- 原始版权归 **jimuzhe/tiez-clipboard** 项目作者及全体贡献者所有。
- 本仓库为基于上游的二次开发版本，依 GPL-3.0 第 5 条要求保留原始版权声明、协议文本与变更说明。
- 任何基于本仓库的再分发同样必须以 GPL-3.0 协议开源全部对应源码。
