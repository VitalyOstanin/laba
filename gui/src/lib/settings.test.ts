import { describe, it, expect, beforeEach } from "vitest";
import {
  defaultSettings,
  setPollOverride,
  setServerEnabled,
  setTimelogStart,
} from "./store";
import { applyTheme } from "./theme";
import { fmtMinutes, fmtSigned } from "./format";

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

describe("setServerEnabled", () => {
  it("adds and removes from disabled_servers", () => {
    const off = setServerEnabled(defaultSettings, "work", false);
    expect(off.disabled_servers).toEqual(["work"]);
    const on = setServerEnabled(off, "work", true);
    expect(on.disabled_servers).toEqual([]);
  });

  it("does not duplicate a disabled server", () => {
    let s = setServerEnabled(defaultSettings, "a", false);
    s = setServerEnabled(s, "a", false);
    expect(s.disabled_servers).toEqual(["a"]);
  });
});

describe("setTimelogStart", () => {
  it("sets a date and clears the auto flag", () => {
    const s = setTimelogStart(defaultSettings, "work", "2026-07-01");
    expect(s.timelog_start.work).toEqual({ date: "2026-07-01", auto: false });
  });

  it("clears the start on empty input", () => {
    const base = {
      ...defaultSettings,
      timelog_start: { work: { date: "2026-07-01", auto: true } },
    };
    expect(setTimelogStart(base, "work", "").timelog_start).toEqual({});
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
