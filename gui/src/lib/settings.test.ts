import { describe, it, expect, beforeEach } from "vitest";
import { parsePollSecs } from "./store";
import { applyTheme } from "./theme";
import { fmtMinutes, fmtSigned } from "./format";

describe("parsePollSecs", () => {
  it("parses a positive value to seconds", () => {
    expect(parsePollSecs("300")).toBe(300);
  });

  it("returns undefined (clear the override) on blank or non-positive input", () => {
    expect(parsePollSecs("")).toBeUndefined();
    expect(parsePollSecs("0")).toBeUndefined();
    expect(parsePollSecs("-5")).toBeUndefined();
  });
});

describe("fmtMinutes / fmtSigned", () => {
  it("formats minutes", () => {
    expect(fmtMinutes(0)).toBe("0m");
    expect(fmtMinutes(90)).toBe("1h 30m");
    expect(fmtMinutes(480)).toBe("8h");
    expect(fmtMinutes(45)).toBe("45m");
  });

  it("formats signed deltas", () => {
    expect(fmtSigned(0)).toBe("0");
    expect(fmtSigned(90)).toBe("+1h 30m");
    expect(fmtSigned(-120)).toBe("−2h");
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
