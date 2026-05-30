// Feature: magpie-theme-redesign, Property 3: 前后端判定一致（前端侧）
import { describe, it, expect } from "vitest";
import fc from "fast-check";
import { isGlassTheme, normalizeThemeId } from "./themes";
import glassThemeVectors from "./__fixtures__/glassThemeVectors.json";

/**
 * Property 3: 前后端判定一致
 * **Validates: Requirements 4.1, 4.2, 4.3**
 *
 * 本文件为前端侧对照：读取共享测试向量表（前后端唯一权威输入源
 * src/shared/config/__fixtures__/glassThemeVectors.json），逐向量断言：
 *   1. 前端 isGlassTheme(input) === 向量期望 expectedGlass
 *   2. expectedGlass 仅在归一后为 mist/dusk 时为真（与后端 is_glass_theme 同口径）
 *
 * 后端 Rust 侧 (src-tauri/src/app/commands/ui_cmd.rs) 以 include_str! 嵌入同一份 JSON 做对照，
 * 两侧对每个向量结论一致即证明前后端玻璃判定一致。
 */

/** 共享向量条目结构 */
interface GlassVector {
  category: string;
  input: string;
  expectedGlass: boolean;
}

/** 归一后属于玻璃主题的集合（与后端 is_glass_theme 口径一致） */
const GLASS_IDS = new Set(["mist", "dusk"]);

const vectors = glassThemeVectors.vectors as GlassVector[];

describe("Property 3: 前后端玻璃判定一致（前端对照共享向量表）", () => {
  it("共享向量表覆盖合法值/旧别名/空串/未知值四类输入", () => {
    const categories = new Set(vectors.map((v) => v.category));
    expect(categories.has("legal")).toBe(true);
    expect(categories.has("legacy")).toBe(true);
    expect(categories.has("empty")).toBe(true);
    expect(categories.has("unknown")).toBe(true);
  });

  it("每个向量：前端 isGlassTheme(input) 等于期望 expectedGlass", () => {
    for (const { input, expectedGlass } of vectors) {
      expect(isGlassTheme(input)).toBe(expectedGlass);
    }
  });

  it("每个向量：expectedGlass 当且仅当归一后为 mist/dusk", () => {
    for (const { input, expectedGlass } of vectors) {
      expect(expectedGlass).toBe(GLASS_IDS.has(normalizeThemeId(input)));
    }
  });

  // 以共享向量集合驱动 fast-check：随机抽取向量，断言前端结果恒与期望表一致，
  // 且仅归一后 mist/dusk 为真（双向锁定，避免实现漂移）。
  it("fast-check 驱动向量集合：前端结果恒等于期望且仅 mist/dusk 为真", () => {
    fc.assert(
      fc.property(fc.constantFrom(...vectors), (vector) => {
        const actual = isGlassTheme(vector.input);
        // 与共享向量期望一致
        expect(actual).toBe(vector.expectedGlass);
        // 仅归一后 mist/dusk 返回真
        expect(actual).toBe(GLASS_IDS.has(normalizeThemeId(vector.input)));
      }),
      { numRuns: 200 }
    );
  });
});
