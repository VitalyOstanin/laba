import { describe, it, expect } from "vitest";
import { settingsSectionMatches } from "./settings-search";

describe("settingsSectionMatches", () => {
  const text = "Theme System Dark Light";
  it("matches every section on a blank query", () => {
    expect(settingsSectionMatches(text, "")).toBe(true);
    expect(settingsSectionMatches(text, "   ")).toBe(true);
  });
  it("matches case-insensitively on a substring", () => {
    expect(settingsSectionMatches(text, "dark")).toBe(true);
    expect(settingsSectionMatches(text, "THEME")).toBe(true);
  });
  it("requires every query word to appear, in any order", () => {
    expect(settingsSectionMatches(text, "dark theme")).toBe(true);
    expect(settingsSectionMatches(text, "theme dark")).toBe(true);
    expect(settingsSectionMatches(text, "dark timezone")).toBe(false);
  });
  it("does not match when no word is present", () => {
    expect(settingsSectionMatches(text, "proxy")).toBe(false);
  });
});
