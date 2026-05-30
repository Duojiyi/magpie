// Feature: magpie-theme-redesign, 任务 2.6：主题 CSS 语义变量完备性
// 单元测试：解析 4 套主题 CSS，断言每套在浅/暗两轨均定义 6 个语义变量且取值为非空合法 CSS 颜色。
// _Requirements: 9.5, 1.9_
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** 4 套新主题标识（与 themes.ts 一致）。 */
const THEME_IDS = ["ink", "paper", "mist", "dusk"] as const;
type ThemeId = (typeof THEME_IDS)[number];

/**
 * 需求 9.5 列举的 6 个语义变量。
 * 每套主题在浅色与暗色两种模式下均须为这些变量提供非空有效 CSS 颜色值。
 */
const SEMANTIC_VARS = [
  "--accent-soft",
  "--accent-glow",
  "--danger",
  "--success",
  "--sensitive",
  "--sensitive-soft",
] as const;

/** 测试文件与 4 套主题 CSS 同目录。 */
const THEMES_DIR = dirname(fileURLToPath(import.meta.url));

/** CSS 规则块：选择器、声明体，以及外层 @-规则前置语句栈（用于识别 prefers-color-scheme: dark）。 */
interface CssRule {
  selector: string;
  declarations: string;
  atRules: string[];
}

/**
 * 大括号匹配 + @-规则上下文栈的极简 CSS 解析器。
 * 能正确处理嵌套 @media（如 reduced-transparency 内再套 prefers-color-scheme: dark）。
 */
function parseCssRules(css: string): CssRule[] {
  // 先剥离注释，避免注释中的花括号/分号干扰解析
  const src = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules: CssRule[] = [];
  const atStack: string[] = [];
  let buffer = "";
  let i = 0;

  while (i < src.length) {
    const ch = src[i];
    if (ch === "{") {
      const prelude = buffer.trim();
      buffer = "";
      i++;
      if (prelude.startsWith("@")) {
        // 带块的 @-规则（如 @media）：压栈，其内部规则继续解析
        atStack.push(prelude);
      } else {
        // 样式规则：捕获声明体直到配对的 }
        let decl = "";
        let depth = 1;
        while (i < src.length && depth > 0) {
          const c = src[i];
          if (c === "{") depth++;
          else if (c === "}") {
            depth--;
            if (depth === 0) break;
          }
          decl += c;
          i++;
        }
        i++; // 跳过收尾的 }
        rules.push({ selector: prelude, declarations: decl, atRules: [...atStack] });
      }
    } else if (ch === "}") {
      // 收尾一个 @-规则块
      atStack.pop();
      buffer = "";
      i++;
    } else {
      buffer += ch;
      i++;
    }
  }
  return rules;
}

/** 判断规则栈是否处于暗色模式媒体查询（prefers-color-scheme: dark）内。 */
function inDarkMedia(rule: CssRule): boolean {
  return rule.atRules.some((a) => /prefers-color-scheme\s*:\s*dark/.test(a));
}

/** 聚合某主题「浅轨」声明：选择器命中 .theme-{id}、不含 dark-mode、且不在暗色媒体查询内。 */
function collectLightDeclarations(rules: CssRule[], id: ThemeId): string {
  return rules
    .filter(
      (r) =>
        r.selector.includes(`.theme-${id}`) &&
        !r.selector.includes("dark-mode") &&
        !inDarkMedia(r)
    )
    .map((r) => r.declarations)
    .join("\n");
}

/** 聚合某主题「暗轨」声明：选择器命中 .theme-{id}，且（含 dark-mode 类 或 处于暗色媒体查询内）。 */
function collectDarkDeclarations(rules: CssRule[], id: ThemeId): string {
  return rules
    .filter(
      (r) =>
        r.selector.includes(`.theme-${id}`) &&
        (r.selector.includes("dark-mode") || inDarkMedia(r))
    )
    .map((r) => r.declarations)
    .join("\n");
}

/**
 * 从声明体中取出指定 CSS 变量的最后一个赋值（后定义覆盖前者）。
 * 变量名以 ':' 收尾界定，故 `--sensitive` 不会误匹配 `--sensitive-soft` / `--sensitive-accent`。
 */
function readVarValue(declarations: string, varName: string): string | null {
  const re = new RegExp(`${varName}\\s*:\\s*([^;]+);`, "g");
  let match: RegExpExecArray | null;
  let last: string | null = null;
  while ((match = re.exec(declarations)) !== null) {
    last = match[1].trim();
  }
  return last;
}

/** 校验是否为非空合法 CSS 颜色值（hex / rgb(a) / hsl(a)）。 */
function isValidColor(value: string | null): boolean {
  if (!value) return false;
  const v = value.trim();
  if (v.length === 0) return false;
  if (/^#[0-9a-fA-F]{3,8}$/.test(v) && [4, 5, 7, 9].includes(v.length)) return true; // #rgb/#rgba/#rrggbb/#rrggbbaa
  if (/^rgba?\([^)]+\)$/i.test(v)) return true;
  if (/^hsla?\([^)]+\)$/i.test(v)) return true;
  return false;
}

// 一次性读取并解析 4 套主题 CSS
const parsedThemes = new Map<ThemeId, CssRule[]>(
  THEME_IDS.map((id) => {
    const css = readFileSync(join(THEMES_DIR, `${id}.css`), "utf-8");
    return [id, parseCssRules(css)] as const;
  })
);

describe("任务 2.6：主题 CSS 语义变量完备性（需求 9.5、1.9）", () => {
  describe.each(THEME_IDS)("主题 %s", (id) => {
    const rules = parsedThemes.get(id)!;
    const lightDecls = collectLightDeclarations(rules, id);
    const darkDecls = collectDarkDeclarations(rules, id);

    it("浅轨与暗轨均存在主题块", () => {
      expect(lightDecls.length).toBeGreaterThan(0);
      expect(darkDecls.length).toBeGreaterThan(0);
    });

    it.each(SEMANTIC_VARS)("浅轨定义 %s 且为非空合法颜色", (varName) => {
      const value = readVarValue(lightDecls, varName);
      expect(value, `${id} 浅轨缺失 ${varName}`).not.toBeNull();
      expect(isValidColor(value), `${id} 浅轨 ${varName} 值非法: ${value}`).toBe(true);
    });

    it.each(SEMANTIC_VARS)("暗轨定义 %s 且为非空合法颜色", (varName) => {
      const value = readVarValue(darkDecls, varName);
      expect(value, `${id} 暗轨缺失 ${varName}`).not.toBeNull();
      expect(isValidColor(value), `${id} 暗轨 ${varName} 值非法: ${value}`).toBe(true);
    });
  });
});
