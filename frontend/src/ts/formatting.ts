import { LANG_INTL_LOCALE } from "./i18n";
import type { UiLanguage } from "./types";

/**
 * Locale-aware number, byte, date, case, and collation utilities
 * aligned with TEXTCRAFT (references/12-turkish-typesetting.md, 13-numerals-measurements.md, 14-dates-time.md).
 */

export function formatLocaleNumber(
  value: number | null | undefined,
  lang: UiLanguage,
  maximumFractionDigits = 1
): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }
  const locale = LANG_INTL_LOCALE[lang] ?? "en-US";
  return new Intl.NumberFormat(locale, { maximumFractionDigits }).format(value);
}

export function formatLocaleBytes(
  bytes: number | null | undefined,
  lang: UiLanguage
): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) {
    return "—";
  }
  const locale = LANG_INTL_LOCALE[lang] ?? "en-US";
  if (bytes === 0) {
    return "0 KB";
  }
  if (bytes < 1024 * 1024) {
    const kb = Math.max(1, Math.round(bytes / 1024));
    return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(kb)} KB`;
  }
  const mb = bytes / (1024 * 1024);
  return `${new Intl.NumberFormat(locale, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(mb)} MB`;
}

export function formatLocaleEpoch(
  secs: number | null | undefined,
  lang: UiLanguage,
  unknownFallback: string
): string {
  if (secs === null || secs === undefined || !Number.isFinite(secs) || secs <= 0) {
    return unknownFallback;
  }
  const locale = LANG_INTL_LOCALE[lang] ?? "en-US";
  try {
    return new Intl.DateTimeFormat(locale, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(secs * 1000));
  } catch {
    return new Date(secs * 1000).toISOString().slice(0, 16).replace("T", " ");
  }
}

export function toLocaleLowerFold(text: string, lang: UiLanguage): string {
  const locale = LANG_INTL_LOCALE[lang] ?? "en-US";
  return text.normalize("NFC").toLocaleLowerCase(locale);
}

export function toLocaleUpperSafe(text: string, lang: UiLanguage): string {
  const locale = LANG_INTL_LOCALE[lang] ?? "en-US";
  return text.normalize("NFC").toLocaleUpperCase(locale);
}

export function compareLocaleStrings(a: string, b: string, lang: UiLanguage): number {
  const locale = LANG_INTL_LOCALE[lang] ?? "en-US";
  return new Intl.Collator(locale, {
    sensitivity: "base",
    numeric: true,
  }).compare(a.normalize("NFC"), b.normalize("NFC"));
}
