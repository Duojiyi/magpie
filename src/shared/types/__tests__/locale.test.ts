import { describe, it, expect } from "vitest";
import {
  DEFAULT_LOCALE,
  SUPPORTED_LOCALES,
  isLocale,
  normalizeLocale,
} from "../locale";

/**
 * Guards for the locale whitelist (audit P1-7): a persisted/synced `app.language` that
 * is not a real translation table must never reach `translations[language]`, or `t()`
 * throws and the whole window goes blank. `normalizeLocale` is that gate.
 */
describe("normalizeLocale / isLocale", () => {
  it("accepts every supported locale unchanged", () => {
    for (const locale of SUPPORTED_LOCALES) {
      expect(isLocale(locale)).toBe(true);
      expect(normalizeLocale(locale)).toBe(locale);
    }
  });

  it("falls back to the default for unknown or malformed values", () => {
    for (const bad of ["jp", "fr", "EN", "", "zh-CN", " zh "]) {
      expect(isLocale(bad)).toBe(false);
      expect(normalizeLocale(bad)).toBe(DEFAULT_LOCALE);
    }
  });

  it("falls back to the default for non-string values", () => {
    for (const bad of [null, undefined, 42, {}, [], true]) {
      expect(isLocale(bad)).toBe(false);
      expect(normalizeLocale(bad)).toBe(DEFAULT_LOCALE);
    }
  });

  it("uses a supported locale as the default", () => {
    expect(SUPPORTED_LOCALES).toContain(DEFAULT_LOCALE);
  });
});
