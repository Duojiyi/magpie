import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyThemeClasses, normalizeThemeId, supportsSurfaceOpacity } from "../config/themes";
import type { CardDensity } from "../../features/app/types";

interface UseSettingsApplyOptions {
  theme: string;
  colorMode: string;

  compactMode: boolean;
  cardDensity: CardDensity;
  settingsLoaded: boolean;
  clipboardItemFontSize: number;
  clipboardTagFontSize: number;
  surfaceOpacity: number;
  showAppBorder: boolean;
}

export const useSettingsApply = ({
  theme,
  colorMode,

  compactMode,
  cardDensity,
  settingsLoaded,
  clipboardItemFontSize,
  clipboardTagFontSize,
  surfaceOpacity,
  showAppBorder: _showAppBorder
}: UseSettingsApplyOptions) => {
  useEffect(() => {
    if (!settingsLoaded) return;

    const root = document.documentElement;
    const body = document.body;

    let disposed = false;
    const normalizedTheme = normalizeThemeId(theme);

    const applyExplicitMode = (mode: "light" | "dark") => {
      if (disposed) return;
      root.classList.remove("light-mode", "dark-mode");
      body.classList.remove("light-mode", "dark-mode");
      if (mode === "dark") {
        root.classList.add("dark-mode");
        body.classList.add("dark-mode");
      } else {
        root.classList.add("light-mode");
        body.classList.add("light-mode");
      }
    };

    const applySystemMode = async () => {
      try {
        const current = await getCurrentWindow().theme();
        if (disposed) return;
        applyExplicitMode(current === "dark" ? "dark" : "light");
      } catch {
        if (disposed) return;
        const isDark =
          window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
        applyExplicitMode(isDark ? "dark" : "light");
      }
    };

    applyThemeClasses(normalizedTheme, root, body);

    if (compactMode) {
      body.classList.add("compact-mode");
    } else {
      body.classList.remove("compact-mode");
    }

    // 卡片密度三档（V5 / 需求 32）：在 body 上挂 density-* 类，由 compact-mode.css 调整高度与间距。
    // 与 compactMode 协调——紧凑模式叠加密度类时仍以更密集布局为基础。
    body.classList.remove("density-compact", "density-standard", "density-loose");
    body.classList.add(`density-${cardDensity}`);

    if (colorMode === "light") {
      applyExplicitMode("light");
    } else if (colorMode === "dark") {
      applyExplicitMode("dark");
    } else {
      applySystemMode();
    }

    invoke("set_theme", {
      theme: normalizedTheme,
      color_mode: colorMode,
    }).catch(console.error);

    let unlisten: (() => void) | null = null;
    let cleanupMedia: (() => void) | null = null;

    if (colorMode === "system") {
      getCurrentWindow()
        .onThemeChanged((event) => {
          if (disposed) return;
          const next = event?.payload === "dark" ? "dark" : "light";
          applyExplicitMode(next);
          invoke("set_theme", {
            theme: normalizedTheme,
            color_mode: "system",
          }).catch(console.error);
        })
        .then((f) => {
          if (disposed) {
            f();
            return;
          }
          unlisten = f;
        });

      if (window.matchMedia) {
        const media = window.matchMedia("(prefers-color-scheme: dark)");
        const onChange = () => applyExplicitMode(media.matches ? "dark" : "light");
        if (media.addEventListener) {
          media.addEventListener("change", onChange);
          cleanupMedia = () => media.removeEventListener("change", onChange);
        } else {
          media.addListener(onChange);
          cleanupMedia = () => media.removeListener(onChange);
        }
      }
    }

    return () => {
      disposed = true;
      if (unlisten) unlisten();
      if (cleanupMedia) cleanupMedia();
    };
  }, [theme, colorMode, settingsLoaded, compactMode, cardDensity]);

  useEffect(() => {
    if (!settingsLoaded) return;
    const root = document.documentElement;
    root.style.setProperty("--clipboard-item-font-size", `${clipboardItemFontSize}px`);
    root.style.setProperty("--clipboard-tag-font-size", `${clipboardTagFontSize}px`);
    // --surface-opacity-scale 仅对玻璃主题（mist/dusk）有意义：
    // 玻璃主题下按 surfaceOpacity 计算缩放并写入；扁平主题（ink/paper）移除该变量，避免残留影响实色表面。
    if (supportsSurfaceOpacity(theme)) {
      const scale = Math.min(2, Math.max(0, surfaceOpacity / 50));
      root.style.setProperty("--surface-opacity-scale", scale.toString());
    } else {
      root.style.removeProperty("--surface-opacity-scale");
    }
  }, [clipboardItemFontSize, clipboardTagFontSize, surfaceOpacity, settingsLoaded, theme]);
};
