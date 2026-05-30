/**
 * 平台 vibrancy（玻璃模糊）能力查询 + 缓存。
 *
 * 背景：v0.4.5 起玻璃主题（mist/dusk）在 Win11 用 mica，Win10 降级为不透明实色。
 * 前端必须在首帧前就知道结果，否则玻璃主题首次应用会出现「透明窗口直透桌面」的闪烁
 * （applyThemeClasses 同步先执行 → invoke 异步后返回挂 no-vibrancy → 中间帧透明）。
 *
 * 解法：模块级内存缓存 + localStorage 持久化。
 * - 启动时同步读 localStorage 兜底（同一台机器 OS 版本不变，缓存恒有效）
 * - 异步 prefetch 用 invoke 拉权威值并刷新缓存
 * - 全程不阻塞首帧渲染
 */

import { invoke } from "@tauri-apps/api/core";

const STORAGE_KEY = "tiez_vibrancy_capability_v1";

// 模块级内存缓存：null = 未知（首次启动且无 localStorage 缓存），true/false = 已知
let memCache: boolean | null = null;
let prefetchInflight: Promise<boolean> | null = null;

/** 启动时同步读 localStorage 兜底缓存（应用首帧前调用） */
export const initVibrancyCapabilityCache = (): void => {
  if (memCache !== null) return;
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "true") memCache = true;
    else if (v === "false") memCache = false;
    // 其他值（含 null）保持 memCache=null，调用方走"保守挂 no-vibrancy"兜底
  } catch {
    // localStorage 不可用，沿用 null
  }
};

/**
 * 同步读已缓存的 vibrancy 能力值。
 * - true：支持玻璃模糊（Win11 / macOS）
 * - false：不支持（Win10）
 * - null：尚未查询（首次启动且无 localStorage 缓存）——调用方应保守挂 no-vibrancy
 */
export const getCachedVibrancyCapability = (): boolean | null => memCache;

/**
 * 异步查询权威值并刷新缓存。多次调用复用同一个 in-flight Promise，避免并发查询。
 * 失败时不抛错，沿用现有缓存（如果有）；首次失败且无缓存时返回 false（保守降级）。
 */
export const prefetchVibrancyCapability = (): Promise<boolean> => {
  if (prefetchInflight) return prefetchInflight;
  prefetchInflight = invoke<boolean>("get_vibrancy_capability")
    .then((supports) => {
      memCache = supports;
      try {
        localStorage.setItem(STORAGE_KEY, supports ? "true" : "false");
      } catch {
        // localStorage 写入失败，仅依赖内存缓存
      }
      return supports;
    })
    .catch(() => {
      // 查询失败：若已有缓存沿用，否则保守降级为 false（挂 no-vibrancy）
      const fallback = memCache ?? false;
      if (memCache === null) memCache = fallback;
      return fallback;
    })
    .finally(() => {
      prefetchInflight = null;
    });
  return prefetchInflight;
};

/**
 * 把 no-vibrancy class 应用到指定元素：能力为 false 时挂上，true 时移除，null 时保守挂上。
 * 设计取舍：null 时（首次启动无缓存）保守挂 no-vibrancy，宁愿首启动看不到玻璃模糊，
 * 也比 Win10 用户看到桌面透出强；prefetch 完成后会被 effect 重新调度修正为权威值。
 */
export const applyVibrancyClass = (
  ...targets: Array<Element | null | undefined>
): void => {
  const supports = memCache;
  for (const el of targets) {
    if (!el) continue;
    if (supports === true) {
      el.classList.remove("no-vibrancy");
    } else {
      // false 或 null 都挂 no-vibrancy（null 是保守兜底）
      el.classList.add("no-vibrancy");
    }
  }
};
