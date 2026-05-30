import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  applyThemeClasses,
  isGlassTheme,
  normalizeThemeId,
  THEMES,
  type ThemeId
} from "./themes";

// ---------------------------------------------------------------------------
// DOM 桩：vitest 全局环境为 node，未安装 jsdom/happy-dom。
// 这里实现一个忠实于 DOMTokenList 契约的最小 classList（用 Set 支撑，保持插入顺序），
// 仅覆盖被测函数实际用到的 API：add / remove / contains 以及可迭代（[...classList]）。
// applyThemeClasses / clearThemeClasses 只使用这些标准 classList 方法，故此桩对其逻辑无失真。
// ---------------------------------------------------------------------------
class ClassListStub {
  private readonly tokens = new Set<string>();

  add(...names: string[]): void {
    for (const name of names) this.tokens.add(name);
  }

  remove(...names: string[]): void {
    for (const name of names) this.tokens.delete(name);
  }

  contains(name: string): boolean {
    return this.tokens.has(name);
  }

  [Symbol.iterator](): IterableIterator<string> {
    return this.tokens[Symbol.iterator]();
  }
}

/** 创建一个带 classList 的元素桩，必要时预置若干初始 class。 */
const createElement = (...initialClasses: string[]): Element => {
  const classList = new ClassListStub();
  classList.add(...initialClasses);
  // 仅 classList 字段被被测函数访问，按结构类型断言为 Element。
  return { classList } as unknown as Element;
};

/** 取元素上全部 theme- 前缀 class。 */
const themePrefixed = (el: Element): string[] =>
  [...(el.classList as unknown as Iterable<string>)].filter((c) =>
    c.startsWith("theme-")
  );

const NEW_THEME_IDS: ThemeId[] = THEMES.map((t) => t.id);
const LEGACY_VALUES = ["mica", "acrylic", "sakura", "retro", "sticky-note", "paper"];

/**
 * 智能生成器：覆盖 applyThemeClasses 的完整输入空间。
 * 包含新主题值、旧别名、store-* 前缀、空串/纯空白、任意字符串及非字符串值（null/undefined/数字/布尔）。
 */
const themeInputArb: fc.Arbitrary<unknown> = fc.oneof(
  fc.constantFrom(...NEW_THEME_IDS),
  fc.constantFrom(...LEGACY_VALUES),
  fc.string().map((s) => `store-${s}`),
  fc.constantFrom("", "   ", "\t", "Unknown", "INK", "Mist"),
  fc.string(),
  fc.constantFrom(null, undefined, 0, 1, true, false, NaN)
);

describe("Property 4: class 与玻璃判定对齐", () => {
  // 子属性一：theme-glass 存在 ⟺ isGlassTheme(t) 为真（Requirements 5.1, 5.2, 5.5）
  it("applyThemeClasses 后 DOM 含 theme-glass 当且仅当 isGlassTheme(t) 为真", () => {
    fc.assert(
      fc.property(themeInputArb, (t) => {
        const el = createElement();
        applyThemeClasses(t, el);
        expect(el.classList.contains("theme-glass")).toBe(isGlassTheme(t));
      })
    );
  });

  // 子属性二：应用后恰好保留一个 theme-{id}，且为归一后的目标主题（Requirements 5.4）
  it("applyThemeClasses 后恰好保留一个 theme-{id} 且等于归一目标", () => {
    fc.assert(
      fc.property(themeInputArb, (t) => {
        const el = createElement();
        applyThemeClasses(t, el);

        const expectedId = normalizeThemeId(t);
        const idClasses = NEW_THEME_IDS.map((id) => `theme-${id}`).filter((c) =>
          el.classList.contains(c)
        );

        // 恰好一个 theme-{id}，且为目标主题
        expect(idClasses).toEqual([`theme-${expectedId}`]);
        // theme- 前缀 class 总数 = 1 个 theme-{id} + 玻璃主题时的 theme-glass
        expect(themePrefixed(el).length).toBe(isGlassTheme(t) ? 2 : 1);
      })
    );
  });

  // 子属性三：切换主题 t1→t2 后旧 theme-*/theme-glass 被清理，仅保留 t2 的恰好一个 theme-{id}（Requirements 5.3, 5.4）
  it("切换主题后旧 theme-*/theme-glass 被清理，仅保留新主题对应 class", () => {
    fc.assert(
      fc.property(themeInputArb, themeInputArb, (t1, t2) => {
        const el = createElement();
        applyThemeClasses(t1, el);
        applyThemeClasses(t2, el);

        const expectedId = normalizeThemeId(t2);
        // 仅保留 t2 对应的 theme-{id}，旧主题 id 不残留
        const idClasses = NEW_THEME_IDS.map((id) => `theme-${id}`).filter((c) =>
          el.classList.contains(c)
        );
        expect(idClasses).toEqual([`theme-${expectedId}`]);
        // theme-glass 与 t2 的玻璃判定对齐（旧值若为玻璃也已被清理）
        expect(el.classList.contains("theme-glass")).toBe(isGlassTheme(t2));
        expect(themePrefixed(el).length).toBe(isGlassTheme(t2) ? 2 : 1);
      })
    );
  });

  // 子属性四：clearThemeClasses 只清理 theme- 前缀，切换不影响无关 class（Requirements 5.3）
  it("切换主题不影响 theme- 前缀以外的无关 class", () => {
    fc.assert(
      fc.property(themeInputArb, (t) => {
        const el = createElement("custom-bg", "density-compact", "dark-mode");
        applyThemeClasses(t, el);
        expect(el.classList.contains("custom-bg")).toBe(true);
        expect(el.classList.contains("density-compact")).toBe(true);
        expect(el.classList.contains("dark-mode")).toBe(true);
      })
    );
  });
});

describe("applyThemeClasses 具体示例（单元测试）", () => {
  it("玻璃主题 mist 追加 theme-glass", () => {
    const el = createElement();
    applyThemeClasses("mist", el);
    expect(el.classList.contains("theme-mist")).toBe(true);
    expect(el.classList.contains("theme-glass")).toBe(true);
  });

  it("扁平主题 ink 不含 theme-glass", () => {
    const el = createElement();
    applyThemeClasses("ink", el);
    expect(el.classList.contains("theme-ink")).toBe(true);
    expect(el.classList.contains("theme-glass")).toBe(false);
  });

  it("从玻璃主题 dusk 切换到扁平主题 paper：theme-glass 与旧 id 被清理", () => {
    const el = createElement();
    applyThemeClasses("dusk", el);
    applyThemeClasses("paper", el);
    expect(el.classList.contains("theme-dusk")).toBe(false);
    expect(el.classList.contains("theme-glass")).toBe(false);
    expect(el.classList.contains("theme-paper")).toBe(true);
    expect(themePrefixed(el)).toEqual(["theme-paper"]);
  });

  it("旧别名 acrylic 归一为 dusk 并标记 theme-glass", () => {
    const el = createElement();
    applyThemeClasses("acrylic", el);
    expect(el.classList.contains("theme-dusk")).toBe(true);
    expect(el.classList.contains("theme-glass")).toBe(true);
  });

  it("非法输入归一为默认 ink（扁平，无 theme-glass）", () => {
    const el = createElement();
    applyThemeClasses(null, el);
    expect(el.classList.contains("theme-ink")).toBe(true);
    expect(el.classList.contains("theme-glass")).toBe(false);
  });

  it("多个目标元素（html/body 模拟）同时应用一致", () => {
    const html = createElement();
    const body = createElement();
    applyThemeClasses("mist", html, body);
    for (const el of [html, body]) {
      expect(el.classList.contains("theme-mist")).toBe(true);
      expect(el.classList.contains("theme-glass")).toBe(true);
    }
  });
});
