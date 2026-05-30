// Feature: magpie-theme-redesign, Property 6: 默认一致
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { DEFAULT_THEME } from "./themes";

/**
 * Property 6: 默认一致
 * **Validates: Requirements 2.1, 2.2, 2.3**
 *
 * 断言前后端默认主题一致为 ink：
 * - 前端：DEFAULT_THEME === "ink"（需求 2.1）
 * - 后端：setup.rs 中两处读取 `app.theme` 的兜底（load_settings 与 apply_initial_theme）
 *   均以 "ink" 作为 unwrap_or 缺省值（需求 2.2、2.3）
 *
 * 由于前端无法直接读取 Rust 常量，后端一致性以「源码文本守卫」实现：
 * 用正则提取 setup.rs 中每处 `.get("app.theme")` 之后的两级 unwrap_or 兜底字面量，
 * 断言其与前端 DEFAULT_THEME 字面量完全一致，使单一测试覆盖前后端两侧。
 */

/** 后端兜底应一致的目标默认主题字面量（与前端 DEFAULT_THEME 同源） */
const EXPECTED_DEFAULT = "ink";

/** setup.rs 绝对路径：从本测试文件（src/shared/config）回溯到仓库根再定位后端源码 */
const setupRsPath = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../../src-tauri/src/app/setup.rs"
);

/**
 * 匹配 `app.theme` 读取处的两级 unwrap_or 兜底：
 *   .get("app.theme") ... .unwrap_or(Some("<内层>".to_string())).unwrap_or("<外层>".to_string())
 * 捕获组 1=内层 Some 兜底，捕获组 2=外层兜底。
 */
const APP_THEME_FALLBACK_RE =
  /\.get\("app\.theme"\)[\s\S]*?\.unwrap_or\(Some\("([^"]+)"\.to_string\(\)\)\)\s*\.unwrap_or\("([^"]+)"\.to_string\(\)\)/g;

describe("Property 6: 前后端默认主题一致（需求 2.1、2.2、2.3）", () => {
  it("前端 DEFAULT_THEME === \"ink\"", () => {
    expect(DEFAULT_THEME).toBe(EXPECTED_DEFAULT);
  });

  it("后端 setup.rs 两处 app.theme 兜底均为 \"ink\" 且与前端一致", () => {
    const source = readFileSync(setupRsPath, "utf-8");
    const matches = [...source.matchAll(APP_THEME_FALLBACK_RE)];

    // 两处兜底：load_settings 与 apply_initial_theme，缺一不可
    expect(matches.length).toBe(2);

    for (const [, inner, outer] of matches) {
      // 内外两级兜底均须等于默认主题字面量，且与前端 DEFAULT_THEME 一致
      expect(inner).toBe(EXPECTED_DEFAULT);
      expect(outer).toBe(EXPECTED_DEFAULT);
      expect(inner).toBe(DEFAULT_THEME);
      expect(outer).toBe(DEFAULT_THEME);
    }
  });
});
