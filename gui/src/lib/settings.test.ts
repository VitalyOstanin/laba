import { describe, it, expect, beforeEach } from "vitest";
import { defaultSettings, setPollOverride } from "./store";
import { applyTheme } from "./theme";

describe("setPollOverride", () => {
  it("sets a positive value", () => {
    const s = setPollOverride(defaultSettings, "work", "300");
    expect(s.poll_override).toEqual({ work: 300 });
  });

  it("clears the override on blank or non-positive input", () => {
    const base = { ...defaultSettings, poll_override: { work: 300 } };
    expect(setPollOverride(base, "work", "").poll_override).toEqual({});
    expect(setPollOverride(base, "work", "0").poll_override).toEqual({});
    expect(setPollOverride(base, "work", "-5").poll_override).toEqual({});
  });

  it("does not mutate the input", () => {
    const base = { ...defaultSettings, poll_override: { a: 100 } };
    setPollOverride(base, "b", "200");
    expect(base.poll_override).toEqual({ a: 100 });
  });
});

describe("applyTheme", () => {
  beforeEach(() => {
    document.documentElement.removeAttribute("data-theme");
  });

  it("sets data-theme for explicit choices", () => {
    applyTheme("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    applyTheme("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("removes data-theme for system", () => {
    document.documentElement.setAttribute("data-theme", "dark");
    applyTheme("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });
});
