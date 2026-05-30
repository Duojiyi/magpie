// Feature: magpie-theme-redesign, Property 1: 迁移完备性
import { describe, it, expect } from "vitest";
import fc from "fast-check";
import { normalizeThemeId } from "../themes";

/**
 * Property 1: 迁移完备性
 * **Validates: Requirements 3.1, 3.2, 3.3**
 *
 * 对任意旧主题值、store-* 值、空串/纯空白/null/未知输入，断言：
 * 1. normalizeThemeId 的结果恒 ∈ {ink, paper, mist, dusk}（Requirement 3.3）
 * 2. 结果与设计文档表 4.1 完全一致（Requirement 3.1、3.2）
 *
 * 期望值在本测试内独立维护（表 4.1 权威对照），不复用被测实现，避免自证。
 */

/** 合法新主题集合（结果论域） */
const VALID_THEMES = new Set(["ink", "paper", "mist", "dusk"]);

/**
 * 设计文档表 4.1：旧主题值 → 新主题值（精确字符串匹配，区分大小写）。
 * 此处独立写出权威对照，不引用 themes.ts 的 LEGACY_THEME_MAP。
 */
const TABLE_4_1: Record<string, string> = {
  mica: "mist",
  acrylic: "dusk",
  sakura: "mist",
  retro: "ink",
  "sticky-note": "paper",
  paper: "paper"
};

/** 全部旧主题值（表 4.1 的键） */
const LEGACY_THEME_VALUES = Object.keys(TABLE_4_1);

/** 纯空白字符生成器（空格 / 制表 / 换行 / 回车 / 换页 / 垂直制表） */
const whitespaceArb = fc.string({
  unit: fc.constantFrom(" ", "\t", "\n", "\r", "\f", "\v"),
  minLength: 1,
  maxLength: 8
});

describe("Property 1: 迁移完备性（需求 3.1/3.2/3.3）", () => {
  it("全部旧主题值映射结果与表 4.1 完全一致且 ∈ 新主题集合", () => {
    fc.assert(
      fc.property(fc.constantFrom(...LEGACY_THEME_VALUES), (legacy) => {
        const result = normalizeThemeId(legacy);
        // 与表 4.1 精确一致
        expect(result).toBe(TABLE_4_1[legacy]);
        // 结果属于合法新主题集合
        expect(VALID_THEMES.has(result)).toBe(true);
      }),
      { numRuns: 200 }
    );
  });

  it("store-* 前缀（任意后缀）一律归一为 ink", () => {
    fc.assert(
      fc.property(fc.string({ maxLength: 32 }), (suffix) => {
        const result = normalizeThemeId(`store-${suffix}`);
        expect(result).toBe("ink");
        expect(VALID_THEMES.has(result)).toBe(true);
      }),
      { numRuns: 300 }
    );
  });

  it("空串与纯空白字符串一律归一为 ink", () => {
    fc.assert(
      fc.property(fc.oneof(fc.constant(""), whitespaceArb), (blank) => {
        const result = normalizeThemeId(blank);
        expect(result).toBe("ink");
        expect(VALID_THEMES.has(result)).toBe(true);
      }),
      { numRuns: 200 }
    );
  });

  it("null / undefined / 非字符串输入一律归一为 ink", () => {
    const nonStringArb = fc.oneof(
      fc.constant(null),
      fc.constant(undefined),
      fc.integer(),
      fc.double(),
      fc.boolean(),
      fc.object(),
      fc.array(fc.anything())
    );
    fc.assert(
      fc.property(nonStringArb, (value) => {
        const result = normalizeThemeId(value as unknown);
        expect(result).toBe("ink");
        expect(VALID_THEMES.has(result)).toBe(true);
      }),
      { numRuns: 300 }
    );
  });

  it("任意未知字符串（非旧值/非新值/非 store-*）一律归一为 ink", () => {
    const unknownArb = fc
      .string({ maxLength: 32 })
      .filter(
        (s) =>
          !(s in TABLE_4_1) &&
          !VALID_THEMES.has(s) &&
          !s.startsWith("store-") &&
          s.trim().length > 0
      );
    fc.assert(
      fc.property(unknownArb, (unknown) => {
        const result = normalizeThemeId(unknown);
        expect(result).toBe("ink");
        expect(VALID_THEMES.has(result)).toBe(true);
      }),
      { numRuns: 300 }
    );
  });

  it("对全部输入类别的并集，结果恒 ∈ {ink, paper, mist, dusk}", () => {
    const anyInputArb = fc.oneof(
      fc.constantFrom(...LEGACY_THEME_VALUES),
      fc.string({ maxLength: 32 }).map((s) => `store-${s}`),
      fc.constant(""),
      whitespaceArb,
      fc.constant(null),
      fc.constant(undefined),
      fc.integer(),
      fc.boolean(),
      fc.string({ maxLength: 32 })
    );
    fc.assert(
      fc.property(anyInputArb, (input) => {
        const result = normalizeThemeId(input as unknown);
        expect(VALID_THEMES.has(result)).toBe(true);
      }),
      { numRuns: 500 }
    );
  });
});
