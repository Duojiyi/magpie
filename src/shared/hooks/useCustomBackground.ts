import { useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { supportsCustomBackground } from "../config/themes";

interface UseCustomBackgroundOptions {
  customBackground: string;
  customBackgroundOpacity: number;
  theme: string;
}

export const useCustomBackground = ({
  customBackground,
  customBackgroundOpacity,
  theme
}: UseCustomBackgroundOptions) => {
  useEffect(() => {
    const root = document.documentElement;
    const body = document.body;
    root.style.setProperty("--custom-bg-opacity", (customBackgroundOpacity / 100).toString());
    // 仅玻璃主题（mist/dusk）支持自定义背景：supportsCustomBackground 与 isGlassTheme 收敛一致。
    if (customBackground && supportsCustomBackground(theme)) {
      // 玻璃主题且已设背景图：挂载 has-custom-bg 并写入 --custom-bg-image。
      root.style.setProperty("--custom-bg-image", `url("${convertFileSrc(customBackground)}")`);
      body.classList.add("has-custom-bg");
    } else {
      // 扁平主题（ink/paper）不挂 has-custom-bg、不设 --custom-bg-image；
      // theme 在依赖项中，玻璃→扁平切换时此分支会移除 has-custom-bg 并清除 --custom-bg-image。
      root.style.removeProperty("--custom-bg-image");
      body.classList.remove("has-custom-bg");
    }
  }, [customBackground, theme, customBackgroundOpacity]);
};
