<p align="left">
  <img src="docs/images/logo.png" width="32" vertical-align="middle" />
  <b>A lightweight information hub — making fragmented information flow effortlessly.</b>
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

## About Magpie

**Magpie** is a local-first, privacy-respecting **lightweight information hub** for Windows, built on a fast clipboard core.

It started as a clipboard manager and is gradually growing into a quiet place where the fragments you copy, sync, tag, and revisit all flow together. Capturing text and images is just the entry point — Magpie keeps your scattered bits organized and within reach, so the right snippet is always one shortcut away.

This repository is based on [`jimuzhe/tiez-clipboard`](https://github.com/jimuzhe/tiez-clipboard) under the GPL-3.0 license, and has been independently maintained as **Magpie** since v0.4.0. It includes upstream-pending bug fixes, privacy hardening, and UX improvements. See [CHANGELOG](./CHANGELOG.md) for details.

> The magpie is folklorically associated with collecting shiny things — a fitting metaphor for an information hub that quietly keeps the bits and pieces you'll want later.

---

<div align="center">

## Theme Gallery

  <sub>Design preview of the new Magpie v0.4.x visual system (calm low-saturation palette + liquid-glass variants). These are design mockups; live screenshots will follow once the refreshed UI ships.</sub>

  <table>
    <tr>
      <td align="center"><b>Ink Jade</b><br><sub>flat · default</sub><br><img src="docs/images/theme-ink-jade.png" width="220" /></td>
      <td align="center"><b>Paper</b><br><sub>warm editorial</sub><br><img src="docs/images/theme-paper.png" width="220" /></td>
    </tr>
    <tr>
      <td align="center"><b>Mist</b><br><sub>liquid glass · light</sub><br><img src="docs/images/theme-mist.png" width="220" /></td>
      <td align="center"><b>Dusk</b><br><sub>liquid glass · dark</sub><br><img src="docs/images/theme-dusk.png" width="220" /></td>
    </tr>
  </table>

</div>

---

## Why Magpie

| Performance | Practicality | Privacy | Sync |
| :--- | :--- | :--- | :--- |
| **Instant Access**<br>Native listeners and Rust core ensure absolute speed. | **Power Workflows**<br>Rich text, tags, and AI-assisted actions. | **Local & Private**<br>Local-first storage with smart masking for sensitive data in previews. | **Cloud Fluent**<br>Seamless WebDAV and MQTT cross-device sync. |

---

## Key Features

### Core Experience
- **Native Efficiency**: Built with Tauri 2 and Rust for minimum memory footprint.
- **Smart Capture**: Automatically collects text, rich text (HTML), images, and file paths.
- **Modern UI**: Supports Mica/Acrylic effects and Dark/Light modes with multiple polished themes.
- **Edge Docking**: Automatically hides at the screen edge to stay out of your way.

### Management & Enhancements
- **Tag System**: Organize your history with custom multi-color tags.
- **Emoji Library**: Comprehensive built-in emoji management for quick access.
- **Advanced Settings**: Granular control over cleanup rules and app behavior.
- **Privacy Masking**: Auto-masks sensitive info like IDs and phone numbers in previews.

### Networking & Transport
- **WebDAV Sync**: Your data, your cloud. Complete cross-device history.
- **LAN File Transfer**: Seamlessly move items between devices on the same network.
- **Verification Code Sync**: Instant transfer of OTP codes to your active device.
- **MQTT Connectivity**: Optimized for real-time synchronization between devices.

### Productivity Tools
- **External Collaboration**: Open items in external editors with auto-sync back.
- **Global Search**: Find anything by content, source app, or date.
- **Sequential Paste**: Optimized workflow for high-frequency copy-paste tasks.

---

## Installation

| Platform | Requirement | Output |
| :--- | :--- | :--- |
| **Windows** | Windows 10/11 (x64) | `.exe` / `.msi` / `.zip` (Portable) |

[**Download the Latest Release →**](https://github.com/Duojiyi/magpie/releases)

---

## Known Limitations

### Win+V cannot be used as the main hotkey

`Win+V` is reserved by Windows for the built-in Clipboard History feature. Selecting `Win+V` in **Settings → Main Hotkey** will show "hotkey unavailable".

**Workaround**: pick `Alt+V`, `Ctrl+Shift+V`, `` Alt+` `` or any other combination — the experience is otherwise identical.

A proper fix is scheduled for v0.4.1: when `Win+V` is chosen, Magpie will offer to take over the hotkey via the `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\DisabledHotkeys` registry value, with a clear opt-in switch in the settings panel. See [v0.4.1 plan](./docs/v0.4.1-plan.md) for details.

---

## Upgrading from TieZ

If you previously used `jimuzhe/tiez-clipboard` or this repository under the `TieZ` name, installing Magpie v0.4.0 will automatically migrate your data on first launch:
- Old data folder `%APPDATA%\com.tiez\` is copied into the new folder `%APPDATA%\app.magpie\`
- The old folder is kept as a safety net; you may delete it manually once you've confirmed the new build works for you
- The autostart registry entry is migrated from `TieZ` to `Magpie`; old entries are cleaned up

---

## License

This project is licensed under the [GNU GPL-3.0](./LICENSE).

- All original copyright belongs to the authors and contributors of **jimuzhe/tiez-clipboard**.
- This repository is a derivative work redistributed under GPL-3.0. As required by section 5, the original copyright notice, license text, and change descriptions are preserved.
- Any further redistribution based on this repository must remain under GPL-3.0 with full corresponding source code.
