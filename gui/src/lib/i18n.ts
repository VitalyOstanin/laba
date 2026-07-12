import { derived, writable } from "svelte/store";
import { en, type Key } from "./locales/en";
import { ru } from "./locales/ru";
import type { Dict } from "./locales/en";
import type { Lang } from "./types";

/** Concrete BCP-47 locale actually rendered (system choice resolved). */
export type Locale = "en" | "ru";

/** Resolve a language choice to a concrete locale. */
function resolveLocale(lang: Lang): Locale {
  if (lang === "ru" || lang === "en") return lang;
  return navigator.language.startsWith("ru") ? "ru" : "en";
}

/** Resolve a language choice to a concrete dictionary. */
function resolveDict(lang: Lang): Dict {
  return resolveLocale(lang) === "ru" ? ru : en;
}

/** Active language; drives `t`. Set from settings on startup and on change. */
export const language = writable<Lang>("system");

/**
 * Active concrete locale for `Intl.*` formatting (units, plurals, dates).
 * Use as `$locale` in components, or `get(locale)` outside them.
 */
export const locale = derived(language, resolveLocale);

/**
 * Reactive translator: use as `$t("key")` in components. Missing keys return
 * the key itself (visible in dev).
 */
export const t = derived(language, ($lang) => {
  const dict = resolveDict($lang);
  return (key: Key): string => dict[key] ?? key;
});

/**
 * Pick the CLDR plural form for `n` in `loc` and return the matching dictionary
 * string. Keys follow `<prefix>.<category>` (one/few/many/other); the `other`
 * form is the fallback when a language lacks a category. English uses only
 * one/other; Russian uses one/few/many.
 */
export function plural(
  loc: Locale,
  n: number,
  translate: (key: Key) => string,
  prefix: string,
): string {
  const category = new Intl.PluralRules(loc).select(n);
  const key = `${prefix}.${category}` as Key;
  const value = translate(key);
  return value === key ? translate(`${prefix}.other` as Key) : value;
}
