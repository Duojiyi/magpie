export type Locale = "zh" | "en" | "tw";

/** The locales that actually have a translation table in `locales.ts`. */
export const SUPPORTED_LOCALES: readonly Locale[] = ["zh", "en", "tw"];

/** Fallback used whenever a persisted/synced language value is missing or invalid. */
export const DEFAULT_LOCALE: Locale = "zh";

/** Runtime guard: is `value` one of the supported locales? */
export function isLocale(value: unknown): value is Locale {
  return (
    typeof value === "string" &&
    (SUPPORTED_LOCALES as readonly string[]).includes(value)
  );
}

/**
 * Coerce an untrusted language value (from the DB / cloud sync / an older build) to a
 * known `Locale`, falling back to `DEFAULT_LOCALE`. This is the guard that prevents a
 * bad persisted `app.language` (e.g. "jp") from reaching `translations[language]` and
 * crashing `t()` into a full white screen (audit P1-7).
 */
export function normalizeLocale(value: unknown): Locale {
  return isLocale(value) ? value : DEFAULT_LOCALE;
}
