// Feature: magpie-theme-redesign, Property 7: 无残留
import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join, resolve, relative } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Property 7: 无残留
 * **Validates: Requirements 8.4**
 *
 * 全仓产物代码（前端 src 下 .ts/.tsx/.css + 后端 src-tauri/src 下 .rs，
 * 含 web_ui.rs 内嵌 CSS）断言以下主题商店/旧主题标识匹配数为 0：
 *   theme-mica / theme-acrylic / theme-store / VITE_API_BASE_URL / tiez_store_css
 *
 * 关于 `store-`：设计文档表 4.1 与需求 3.2 要求前后端归一器必须保留对旧
 * `store-*` 前缀的识别（`startsWith("store-")` / `t.starts_with("store-")`），
 * 这是合法的迁移归一逻辑而非残留。故 `store-` 采取「仅允许出现在归一识别代码行」
 * 的更精细断言：除该模式外不得有任何其他 `store-` 出现在产物代码（注释/测试除外）。
 *
 * 扫描范围与排除：
 * - 范围：src/**\/*.{ts,tsx,css} 与 src-tauri/src/**\/*.rs
 * - 排除：测试文件自身（*.test.ts/tsx、__tests__）与共享向量表（__fixtures__），
 *   它们属测试基础设施而非产物代码，且按设计天然包含上述字符串用于断言。
 */

/** 仓库根：从本测试文件（src/shared/config）回溯三级 */
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

/** 扫描根目录 */
const SCAN_ROOTS = [
  { dir: join(REPO_ROOT, "src"), exts: [".ts", ".tsx", ".css"] },
  { dir: join(REPO_ROOT, "src-tauri", "src"), exts: [".rs"] }
];

/** 严格 0 匹配的残留标识（任意出现即视为残留） */
const STRICT_PATTERNS = [
  "theme-mica",
  "theme-acrylic",
  "theme-sakura",
  "theme-retro",
  "theme-sticky-note",
  "theme-store",
  "VITE_API_BASE_URL",
  "tiez_store_css"
] as const;

/**
 * `store-` 合法保留：仅允许出现在前后端归一器对旧前缀的识别代码行。
 * 前端 themes.ts 未使用 `store-` 字面量（靠「未命中表则落默认」实现），
 * 后端 ui_cmd.rs / web_ui.rs 以 `t.starts_with("store-")` 显式识别。
 */
const STORE_ALLOWED_LINE_RE = /\bstarts_with\("store-"\)/;

/** 判定是否为测试/向量表文件（排除项） */
const isExcluded = (relPath: string): boolean =>
  /(^|[\\/])__tests__([\\/])/.test(relPath) ||
  /(^|[\\/])__fixtures__([\\/])/.test(relPath) ||
  /\.test\.(ts|tsx)$/.test(relPath);

/** 递归收集指定扩展名的产物代码文件（已排除测试/向量表） */
const collectFiles = (root: string, exts: string[]): string[] => {
  if (!existsSync(root)) return [];
  const result: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(full);
        continue;
      }
      if (!exts.some((ext) => entry.name.endsWith(ext))) continue;
      const rel = relative(REPO_ROOT, full);
      if (isExcluded(rel)) continue;
      result.push(full);
    }
  };
  walk(root);
  return result;
};

/** 全部待扫描的产物代码文件 */
const PRODUCT_FILES = SCAN_ROOTS.flatMap(({ dir, exts }) =>
  collectFiles(dir, exts)
);

/**
 * 截除 Rust 文件末尾的 `#[cfg(test)]` 单元测试模块行：
 * 该模块内的 store-* 等字符串属测试断言（测试基础设施），非产物残留，
 * 与前端 *.test.ts 排除口径一致。返回应纳入扫描的行（保留原始行号）。
 */
const productLines = (
  file: string
): Array<{ line: number; text: string }> => {
  const lines = readFileSync(file, "utf-8").split(/\r?\n/);
  let cutoff = lines.length;
  if (file.endsWith(".rs")) {
    const idx = lines.findIndex((l) => /^\s*#\[cfg\(test\)\]/.test(l));
    if (idx >= 0) cutoff = idx;
  }
  return lines
    .slice(0, cutoff)
    .map((text, idx) => ({ line: idx + 1, text }));
};

/** 在文件集合中查找某子串的全部命中位置（文件:行:行内容，已排除 Rust 测试模块） */
const findHits = (
  pattern: string
): Array<{ file: string; line: number; text: string }> => {
  const hits: Array<{ file: string; line: number; text: string }> = [];
  for (const file of PRODUCT_FILES) {
    for (const { line, text } of productLines(file)) {
      if (text.includes(pattern)) {
        hits.push({ file: relative(REPO_ROOT, file), line, text });
      }
    }
  }
  return hits;
};

describe("Property 7: 无残留（需求 8.4）", () => {
  it("扫描到产物代码文件（前端 src + 后端 src-tauri/src）", () => {
    // 防御：若扫描集合为空，说明路径定位错误，残留断言将失去意义
    expect(PRODUCT_FILES.length).toBeGreaterThan(0);
  });

  it.each(STRICT_PATTERNS)("标识 `%s` 在产物代码中匹配数为 0", (pattern) => {
    const hits = findHits(pattern);
    // 命中即报告具体文件行，便于定位残留
    const detail = hits
      .map((h) => `${h.file}:${h.line}: ${h.text.trim()}`)
      .join("\n");
    expect(hits, `发现残留 ${pattern}：\n${detail}`).toEqual([]);
  });

  it("`store-` 仅出现在归一识别代码行（starts_with(\"store-\")），无其他残留", () => {
    // 仅统计代码行：排除以 // 或 /// 或 * 开头的注释行（迁移逻辑说明）
    const illegal = findHits("store-").filter((h) => {
      const trimmed = h.text.trim();
      const isComment =
        trimmed.startsWith("//") ||
        trimmed.startsWith("*") ||
        trimmed.startsWith("/*");
      if (isComment) return false; // 注释中的 store-* 说明属合法
      return !STORE_ALLOWED_LINE_RE.test(h.text); // 非归一识别行即为非法残留
    });
    const detail = illegal
      .map((h) => `${h.file}:${h.line}: ${h.text.trim()}`)
      .join("\n");
    expect(illegal, `发现非法 store- 残留：\n${detail}`).toEqual([]);
  });
});

/**
 * 构建产物（dist）条件扫描：dist 为可选构建输出，存在时一并核验严格标识 0 匹配，
 * 使本测试在 CI「先构建后测试」场景同时覆盖前端构建产物（设计文档 §12.1 第 5 点）。
 * dist 不存在时跳过（不强制构建）。
 */
describe("Property 7: 构建产物（dist）残留核验（存在时）", () => {
  const DIST_DIR = join(REPO_ROOT, "dist");
  const distFiles = existsSync(DIST_DIR)
    ? collectFiles(DIST_DIR, [".js", ".css", ".html"])
    : [];

  it.runIf(distFiles.length > 0).each(STRICT_PATTERNS)(
    "dist 构建产物中标识 `%s` 匹配数为 0",
    (pattern) => {
      const hits: string[] = [];
      for (const file of distFiles) {
        const content = readFileSync(file, "utf-8");
        if (content.includes(pattern)) hits.push(relative(REPO_ROOT, file));
      }
      expect(hits, `dist 残留 ${pattern}：\n${hits.join("\n")}`).toEqual([]);
    }
  );
});
