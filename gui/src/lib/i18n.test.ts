import { describe, it, expect } from "vitest";
import { en } from "./locales/en";
import { ru } from "./locales/ru";

// Keys whose Russian value legitimately equals the English one: proper nouns
// (brand and product names) and language endonyms shown as-is in the picker.
// A new entry here must be a conscious decision, not a forgotten translation.
const SAME_AS_EN_ALLOWED = new Set<string>([
  "settings.language.en", // "English" — language name shown as-is
  "settings.language.ru", // "Русский" — already the Russian endonym
  "settings.server.openContent.app", // "laba" — app name
  "wizard.backend.op", // "OpenProject" — product name
  "wizard.backend.gh", // "GitHub" — product name
]);

describe("locales", () => {
  it("ru has every en key", () => {
    for (const k of Object.keys(en)) {
      expect(ru).toHaveProperty(k);
    }
  });

  it("ru has no extra keys beyond en", () => {
    for (const k of Object.keys(ru)) {
      expect(en).toHaveProperty(k);
    }
  });

  it("every ru value is a non-empty string", () => {
    for (const [k, v] of Object.entries(ru)) {
      expect(typeof v, `ru[${k}] must be a string`).toBe("string");
      expect((v as string).trim(), `ru[${k}] must not be blank`).not.toBe("");
    }
  });

  it("ru differs from en except for allowed proper nouns", () => {
    const untranslated: string[] = [];
    for (const k of Object.keys(en) as (keyof typeof en)[]) {
      const same = String(ru[k]).trim() === String(en[k]).trim();
      if (same && !SAME_AS_EN_ALLOWED.has(k)) untranslated.push(k);
    }
    expect(untranslated, "Russian values still equal to English").toEqual([]);
  });
});
