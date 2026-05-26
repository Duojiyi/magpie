# 更新日志

本仓库 fork 自 [`jimuzhe/tiez-clipboard`](https://github.com/jimuzhe/tiez-clipboard)，依据 GPL-3.0 协议二次分发。仅记录本仓库相对于上游的变更。

格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

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

[0.3.5]: https://github.com/Duojiyi/tiez-clipboard/releases/tag/v0.3.5
