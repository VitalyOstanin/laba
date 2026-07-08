import { en, type Key } from "./locales/en";
import { ru } from "./locales/ru";

const dict = navigator.language.startsWith("ru") ? ru : en;

/** Translate a key. Missing keys return the key itself (visible in dev). */
export function t(key: Key): string {
  return dict[key] ?? key;
}
