import { derived, writable } from "svelte/store";
import { en, type Key } from "./locales/en";
import { ru } from "./locales/ru";
import type { Dict } from "./locales/en";
import type { Lang } from "./types";

/** Resolve a language choice to a concrete dictionary. */
function resolveDict(lang: Lang): Dict {
  const concrete =
    lang === "system"
      ? navigator.language.startsWith("ru")
        ? "ru"
        : "en"
      : lang;
  return concrete === "ru" ? ru : en;
}

/** Active language; drives `t`. Set from settings on startup and on change. */
export const language = writable<Lang>("system");

/**
 * Reactive translator: use as `$t("key")` in components. Missing keys return
 * the key itself (visible in dev).
 */
export const t = derived(language, ($lang) => {
  const dict = resolveDict($lang);
  return (key: Key): string => dict[key] ?? key;
});
