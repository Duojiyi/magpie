import { describe, expect, it } from "vitest";
import { normalizeSoundVolume } from "../useSoundEffects";

describe("normalizeSoundVolume", () => {
  it("keeps the 0..1 slider scale as-is", () => {
    expect(normalizeSoundVolume("0.35")).toBe(0.35);
    expect(normalizeSoundVolume(1)).toBe(1);
  });

  it("keeps an explicit mute instead of treating 0 as unset", () => {
    expect(normalizeSoundVolume("0")).toBe(0);
  });

  it("reads legacy 0..100 values as percentages", () => {
    expect(normalizeSoundVolume("80")).toBeCloseTo(0.8);
    expect(normalizeSoundVolume(100)).toBe(1);
  });

  it("clamps garbage to a safe range", () => {
    expect(normalizeSoundVolume("abc")).toBe(1);
    expect(normalizeSoundVolume("-3")).toBe(0);
    expect(normalizeSoundVolume("5000")).toBe(1);
  });
});
