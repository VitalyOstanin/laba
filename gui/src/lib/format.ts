import type { Locale } from "./i18n";

/** Format one unit value with the locale's narrow unit label (e.g. `1h`, `1 ч`). */
function unit(loc: Locale, value: number, u: "hour" | "minute"): string {
  return new Intl.NumberFormat(loc, {
    style: "unit",
    unit: u,
    unitDisplay: "narrow",
  }).format(value);
}

/**
 * Format a non-negative minute count as `Xh Ym` / `Xh` / `Ym` / `0m`, with the
 * hour/minute labels localized for `loc` (English `h`/`m`, Russian `ч`/`мин`).
 */
export function fmtMinutes(min: number, loc: Locale = "en"): string {
  const m = Math.max(0, Math.round(min));
  const h = Math.floor(m / 60);
  const r = m % 60;
  if (h && r) return `${unit(loc, h, "hour")} ${unit(loc, r, "minute")}`;
  if (h) return unit(loc, h, "hour");
  return unit(loc, r, "minute");
}

/** Format a signed minute delta as `+Xh Ym` / `−Xh Ym` / `0`. */
export function fmtSigned(min: number, loc: Locale = "en"): string {
  const r = Math.round(min);
  if (r === 0) return "0";
  const sign = r > 0 ? "+" : "−";
  return `${sign}${fmtMinutes(Math.abs(r), loc)}`;
}

/**
 * Parse a `YYYY-MM-DD…` prefix into a local-midnight Date, so a date-only value
 * is not shifted a day by timezone conversion. Returns null on no match.
 */
function parseIsoDate(iso: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

/** Format an ISO date as a locale medium date (e.g. `Jul 8, 2026` / `8 июл. 2026 г.`). */
export function fmtDate(iso: string, loc: Locale = "en"): string {
  const d = parseIsoDate(iso);
  if (!d) return iso;
  return new Intl.DateTimeFormat(loc, { dateStyle: "medium" }).format(d);
}

/** Format an ISO date as a compact locale day/month (e.g. `07/08` / `08.07`). */
export function fmtDayMonth(iso: string, loc: Locale = "en"): string {
  const d = parseIsoDate(iso);
  if (!d) return iso;
  return new Intl.DateTimeFormat(loc, {
    month: "2-digit",
    day: "2-digit",
  }).format(d);
}

/**
 * Format a full ISO timestamp as an absolute locale date + time (e.g.
 * `Jul 10, 2026, 12:20 PM` / `10 июл. 2026 г., 12:20`), rendered in the given
 * IANA `timezone`. The `"system"` sentinel (or an unknown zone) falls back to the
 * machine's local zone. Unparseable input passes through unchanged. This is the
 * default timestamp shown in the lists; the relative form is opt-in.
 */
export function fmtDateTime(
  iso: string,
  loc: Locale = "en",
  timezone = "system",
): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const opts: Intl.DateTimeFormatOptions = {
    dateStyle: "medium",
    timeStyle: "short",
  };
  if (timezone && timezone !== "system") opts.timeZone = timezone;
  try {
    return new Intl.DateTimeFormat(loc, opts).format(d);
  } catch {
    // Unknown IANA zone: render in the local zone rather than throw.
    delete opts.timeZone;
    return new Intl.DateTimeFormat(loc, opts).format(d);
  }
}

/** Coarsest-fitting relative unit, largest first; `second` is the floor. */
const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 31536000],
  ["month", 2592000],
  ["week", 604800],
  ["day", 86400],
  ["hour", 3600],
  ["minute", 60],
  ["second", 1],
];

/**
 * Format a full ISO timestamp relative to `nowMs` (e.g. `5 minutes ago`,
 * `вчера`), picking the coarsest unit that fits. `nowMs` is a parameter so the
 * function is pure and testable. Unparseable input passes through unchanged.
 */
export function fmtRelative(
  iso: string,
  loc: Locale = "en",
  nowMs: number = Date.now(),
): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const diffSec = Math.round((d.getTime() - nowMs) / 1000);
  const rtf = new Intl.RelativeTimeFormat(loc, { numeric: "auto" });
  const abs = Math.abs(diffSec);
  for (const [unit, secs] of RELATIVE_UNITS) {
    if (abs >= secs || unit === "second") {
      return rtf.format(Math.round(diffSec / secs), unit);
    }
  }
  return rtf.format(0, "second");
}
