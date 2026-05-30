// Feature: magpie-theme-redesign, Property 2: 迁移幂等性
import { describe, it, expect } from "vitest";
import fc from "fast-check";
import { normalizeThemeId, DEFAULT_THEME, type ThemeId } from "./themes";

/**
 * Property 2: 迁移幂等性
 * **Validates: Requirements 3.4, 3.5**
 *
 * - 幂等性（需求 3.5）：对任意输入 t，normalizeThemeId(normalizeThemeId(t)) === normalizeThemeId(t)
 * - 新值原样返回（需求 3.4）：对新主题集合 {ink,paper,mist,dusk}，normalizeThemeId(id) === id
 *
 * 输入空间智能覆盖：新值、旧值、store-* 前缀、空串、纯空白、未知字符串、null/非字符串。
 */

/** 新主题集合（需求 3.4 原样返回的对象） */
const NEW_THEME_IDS: ThemeId[] = ["ink", "paper", "mist", "dusk"];

/** 旧主题值（表 4.1 中的别名） */
const LEGACY_THEME_IDS = [
  "mica",
  "acrylic",
  "sakura",
  "retro",
  "sticky-note",
  "paper"
];

/**
 * 覆盖全部输入类别的生成器：
 * - 新值 / 旧值：直接命中归一分支
 * - store-* 前缀：商店主题已移除，应落默认
 * - 空串 / 纯空白：边界字符串
 * - 任意字符串：未知值
 * - null / undefined / 数字 / 布尔 / 对象：非字符串
 */
const anyThemeInputArb: fc.Arbitrary<unknown> = fc.oneof(
  fc.constantFrom(...NEW_THEME_IDS),
  fc.constantFrom(...LEGACY_THEME_IDS),
  fc.string().map((s) => `store-${s}`),
  fc.constantFrom("", " ", "\t", "  \n "),
  fc.string(),
  fc.constantFrom(null, undefined, 0, 1, NaN, true, false, {}, [])
);

describe("Property 2: 迁移幂等性（需求 3.4、3.5）", () => {
  it("对任意输入，二次归一与一次归一结果相等（幂等）", () => {
    fc.assert(
      fc.property(anyThemeInputArb, (t) => {
        const once = normalizeThemeId(t);
        const twice = normalizeThemeId(once);
        // 二次归一不再改变结果
        expect(twice).toBe(once);
        // 归一结果必落在新主题集合内
        expect(NEW_THEME_IDS).toContain(once);
      }),
      { numRuns: 300 }
    );
  });

  it("对新主题集合中的值，归一原样返回该值", () => {
    fc.assert(
      fc.property(fc.constantFrom(...NEW_THEME_IDS), (id) => {
        // 需求 3.4：已是新值，原样返回
        expect(normalizeThemeId(id)).toBe(id);
      }),
      { numRuns: 50 }
    );
  });

  it("默认主题 ink 自身归一恒等（幂等不动点）", () => {
    expect(normalizeThemeId(DEFAULT_THEME)).toBe(DEFAULT_THEME);
  });
});
