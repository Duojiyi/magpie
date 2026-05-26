# 更新日志

本仓库 fork 自 [`jimuzhe/tiez-clipboard`](https://github.com/jimuzhe/tiez-clipboard)，依据 GPL-3.0 协议二次分发。仅记录本仓库相对于上游的变更。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [0.3.6] - 2026-05-26

### 变更

- **检查更新**改为指向本仓库 GitHub Releases（静态 `latest.json`），不再请求上游官网域名 `tiez.name666.top`。
  - 应用内"检查更新"按钮拉取 `https://github.com/Duojiyi/tiez-clipboard/releases/latest/download/latest.json`。
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

[0.3.6]: https://github.com/Duojiyi/tiez-clipboard/releases/tag/v0.3.6
[0.3.5]: https://github.com/Duojiyi/tiez-clipboard/releases/tag/v0.3.5
