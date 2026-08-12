import { describe, it, expect } from "vitest";
import { ErrorBoundary } from "../ErrorBoundary";

/**
 * The boundary's error->state derivation is the load-bearing pure logic (audit P1:
 * a render throw must flip to the fallback instead of white-screening the window).
 * Rendering needs a DOM renderer the node test env doesn't have, so we assert the
 * static reducer directly.
 */
describe("ErrorBoundary.getDerivedStateFromError", () => {
  it("flips to hasError and extracts an Error message", () => {
    const state = ErrorBoundary.getDerivedStateFromError(new Error("boom"));
    expect(state.hasError).toBe(true);
    expect(state.message).toBe("boom");
  });

  it("handles a thrown string", () => {
    const state = ErrorBoundary.getDerivedStateFromError("plain failure");
    expect(state.hasError).toBe(true);
    expect(state.message).toBe("plain failure");
  });

  it("handles a non-error, non-string throw without crashing", () => {
    const state = ErrorBoundary.getDerivedStateFromError({ weird: true });
    expect(state.hasError).toBe(true);
    expect(typeof state.message).toBe("string");
  });
});
