import { describe, it, expect } from "vitest";
import fc from "fast-check";
import {
  isGlassTheme,
  supportsCustomBackground,
  supportsSurfaceOpacity
} from "./themes";

/**
 * Property 5: 能力收敛
 *
 * 断言对任意主题输入 t，三个能力判定恒等：
 *   supportsCustomBackground(t) === isGlassTheme(t)
 *   supportsSurfaceOpacity(t)   === isGlassTheme(t)
 * 即玻璃主题（mist/dusk）三者均为 true，其余（扁平主题及一切异常输入）三者均为 false。
 *
 * Validates: Requirements 7.1
 */
describe("Property 5: 能力收敛", () => {
  // 智能生成器：覆盖归一器的全部输入分区
  // —— 新值、旧别名、store-* 前缀、空串/纯空白、任意未知字符串、非字符串（null 等）
  const themeInputArb = fc.oneof(
    // 新主题值（含玻璃 mist/dusk 与扁平 ink/paper）
    fc.constantFrom("ink", "paper", "mist", "dusk"),
    // 旧主题别名
    fc.constantFrom("mica", "acrylic", "sakura", "retro", "sticky-note", "paper"),
    // store-* 前缀（已移除的主题商店主题）
    fc.string().map((s) => `store-${s}`),
    // 空串与纯空白字符串
    fc.constantFrom("", " ", "   ", "\t", "\n  "),
    // 任意未知字符串
    fc.string(),
    // 非字符串输入：null / undefined / 数字 / 布尔 / 对象 / 数组
    fc.constantFrom(null, undefined, 0, 123, true, false, {}, [])
  );

  it("supportsCustomBackground 与 supportsSurfaceOpacity 与 isGlassTheme 恒等", () => {
    fc.assert(
      fc.property(themeInputArb, (t) => {
        const glass = isGlassTheme(t);
        expect(supportsCustomBackground(t)).toBe(glass);
        expect(supportsSurfaceOpacity(t)).toBe(glass);
      })
    );
  });

  // 针对性断言：玻璃主题三者恒为 true，扁平主题恒为 false（锁定收敛方向）
  it("玻璃主题（mist/dusk）三能力均为 true", () => {
    for (const t of ["mist", "dusk"]) {
      expect(isGlassTheme(t)).toBe(true);
      expect(supportsCustomBackground(t)).toBe(true);
      expect(supportsSurfaceOpacity(t)).toBe(true);
    }
  });

  it("扁平主题（ink/paper）三能力均为 false", () => {
    for (const t of ["ink", "paper"]) {
      expect(isGlassTheme(t)).toBe(false);
      expect(supportsCustomBackground(t)).toBe(false);
      expect(supportsSurfaceOpacity(t)).toBe(false);
    }
  });
});
