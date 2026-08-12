//! Cross-platform keystroke simulation for the non-Windows paste path.
//!
//! Windows keeps its elaborate `SendInput` implementation (scan codes, IME handling, game
//! mode) untouched. macOS and Linux previously fired an `osascript` command and threw away
//! the result — which meant "paste does nothing, silently" on every Linux box and on any Mac
//! without the Accessibility permission. This module uses `enigo` (CGEvent on macOS, XTEST
//! via x11rb on Linux) and reports failure so callers can tell the user *why*.
//!
//! Compiled on every platform so the Windows build we can verify locally type-checks it too;
//! only non-Windows code calls into it.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

fn connection() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| format!("input backend unavailable: {}", e))
}

/// The platform's canonical paste chord: Cmd+V on macOS, Ctrl+V elsewhere.
pub fn paste_combo() -> Result<(), String> {
    let mut enigo = connection()?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| format!("press modifier failed: {}", e))?;
    let click = enigo.key(Key::Unicode('v'), Direction::Click);
    // Always release the modifier, even when the click failed: leaving a phantom held-down
    // Ctrl/Cmd behind would corrupt every subsequent real keystroke the user types.
    let release = enigo.key(modifier, Direction::Release);
    click.map_err(|e| format!("send V failed: {}", e))?;
    release.map_err(|e| format!("release modifier failed: {}", e))?;
    Ok(())
}

/// Type the text directly instead of going through the clipboard chord — the non-Windows
/// equivalent of the Windows "game mode" paste method.
pub fn type_text(text: &str) -> Result<(), String> {
    let mut enigo = connection()?;
    enigo
        .text(text)
        .map_err(|e| format!("type text failed: {}", e))
}

/// Actionable hint appended to paste failures, because the dominant cause differs by OS and
/// neither is discoverable from the raw error.
pub fn paste_failure_hint() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macOS: grant Accessibility permission to Magpie under System Settings → Privacy & Security → Accessibility, then retry"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Linux: simulated input requires X11 or XWayland; on pure Wayland sessions paste manually with Ctrl+V"
    }
}
