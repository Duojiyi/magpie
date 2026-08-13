import { invoke } from "@tauri-apps/api/core";

export type PlatformId = "windows" | "macos" | "linux";

/**
 * Platform reported by the backend, cached after the first successful call.
 *
 * User-agent sniffing (the previous approach) cannot distinguish Linux from Windows, so
 * `!isMacPlatform()` silently classified Linux as Windows and showed it Windows-only
 * settings — a Win+V takeover toggle, "run as administrator", game-mode paste — all of
 * which invoke commands that are no-ops there. The backend already knows the answer via
 * `get_platform_info`; this asks it.
 */
let cachedPlatform: PlatformId | null = null;

/** Synchronous best guess, used only until the backend answers. */
const guessPlatform = (): PlatformId => {
  if (typeof navigator === "undefined") return "windows";
  const ua = navigator.userAgent;
  if (/Mac|iPhone|iPad|iPod/i.test(ua) || /Mac/i.test(navigator.platform)) return "macos";
  if (/Linux|X11|CrOS/i.test(ua) && !/Android/i.test(ua)) return "linux";
  return "windows";
};

export const detectPlatform = async (): Promise<PlatformId> => {
  if (cachedPlatform) return cachedPlatform;
  try {
    const info = await invoke<{ platform: string }>("get_platform_info");
    cachedPlatform =
      info.platform === "macos" ? "macos" : info.platform === "windows" ? "windows" : "linux";
  } catch {
    cachedPlatform = guessPlatform();
  }
  return cachedPlatform;
};

/** Last known platform without awaiting; falls back to the user-agent guess. */
export const currentPlatform = (): PlatformId => cachedPlatform ?? guessPlatform();

export const isMacPlatform = (): boolean => currentPlatform() === "macos";

export const isWindowsPlatform = (): boolean => currentPlatform() === "windows";

export const isLinuxPlatform = (): boolean => currentPlatform() === "linux";
