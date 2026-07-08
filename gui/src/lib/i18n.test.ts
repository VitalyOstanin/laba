import { describe, it, expect } from "vitest";
import { en } from "./locales/en";
import { ru } from "./locales/ru";

describe("locales", () => {
  it("ru has every en key", () => {
    for (const k of Object.keys(en)) {
      expect(ru).toHaveProperty(k);
    }
  });
});
