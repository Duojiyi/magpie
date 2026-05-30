import { useEffect, useLayoutEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DEFAULT_THEME, normalizeThemeId, applyThemeClasses } from "../config/themes";
import type { Locale } from "../types";
import { isTauriRuntime } from "../lib/tauriRuntime";

interface UseSettingsInitOptions {
  setAppSettings: (settings: Record<string, string>) => void;
  setHotkey: (val: string) => void;
  setTheme: (val: string) => void;
  setColorMode: (val: string) => void;
  setCompactMode: (val: boolean) => void;
  setLanguage: (val: Locale) => void;
}

export const useSettingsInit = ({
  setAppSettings,
  setHotkey,
  setTheme,
  setColorMode,
  setCompactMode,
  setLanguage
}: UseSettingsInitOptions) => {
  const [settings, setSettings] = useState<Record<string, string> | null>(null);

  // 首帧渲染：在浏览器绘制前于 DOM 根节点应用默认 theme-ink class（见 Requirement 3.9、3.10）。
  // 保证 get_settings 归一完成前根容器即持有恰好一个有效 theme class，迁移过程不白屏。
  useLayoutEffect(() => {
    applyThemeClasses(DEFAULT_THEME, document.documentElement, document.body);
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;

    const loadSettings = () => {
      invoke<Record<string, string>>("get_settings")
        .then((result) => {
          if (disposed) return;

          setAppSettings(result);
          if (result["app.hotkey"]) setHotkey(result["app.hotkey"]);

          // 读取原始主题值并归一；归一后必为 {ink, paper, mist, dusk} 之一
          const rawTheme = result["app.theme"];
          const normalizedTheme = normalizeThemeId(rawTheme);
          const loadedColorMode = result["app.color_mode"] || "system";

          // 迁移写回：仅当归一值与原始值不同（发生迁移）时，写回 DB 与 localStorage 各恰好一次；
          // 已是新主题值（未迁移）则不写回（见 Requirement 3.6、3.7）。
          if (normalizedTheme !== rawTheme) {
            // 写回 DB；失败时忽略错误、不中断启动，继续用内存归一值渲染，下次启动重试（见 Requirement 3.8）
            invoke("save_setting", { key: "app.theme", value: normalizedTheme }).catch(() => {});
            try {
              localStorage.setItem("tiez_theme", normalizedTheme);
            } catch {
              // 忽略 localStorage 写入失败，下次启动重试
            }
          }

          setTheme(normalizedTheme);
          setColorMode(loadedColorMode);
          setCompactMode(result["app.compact_mode"] === "true");

          // 颜色模式 / 紧凑模式的防启动闪烁缓存（与主题迁移无关，保持原有行为）
          try {
            localStorage.setItem("tiez_color_mode", loadedColorMode);
            localStorage.setItem(
              "tiez_compact_mode",
              result["app.compact_mode"] === "true" ? "true" : "false"
            );
          } catch {
            // 忽略 localStorage 写入失败
          }

          if (result["app.language"]) {
            setLanguage(result["app.language"] as Locale);
          }

          setSettings(result);
        })
        .catch(console.error);
    };

    loadSettings();

    const unlisten = listen("settings-changed", () => {
      loadSettings();
    });

    return () => {
      disposed = true;
      unlisten.then((off) => off());
    };
  }, [setAppSettings, setHotkey, setTheme, setColorMode, setCompactMode, setLanguage]);

  return settings;
};
