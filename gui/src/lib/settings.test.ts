import { describe, it, expect, beforeEach } from "vitest";
import { parsePollSecs } from "./store";
import { applyTheme } from "./theme";
import { fmtMinutes, fmtSigned } from "./format";
import { plural } from "./i18n";

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

  it("localizes unit labels for Russian", () => {
    expect(fmtMinutes(90, "ru")).toBe("1 ч 30 мин");
    expect(fmtMinutes(45, "ru")).toBe("45 мин");
    expect(fmtSigned(-120, "ru")).toBe("−2 ч");
  });
});

describe("plural", () => {
  const dict: Record<string, string> = {
    "notif.newCount.one": "новое уведомление",
    "notif.newCount.few": "новых уведомления",
    "notif.newCount.many": "новых уведомлений",
    "notif.newCount.other": "новых уведомлений",
  };
  const tr = (k: string) => dict[k] ?? k;

  it("selects the Russian plural form by count", () => {
    expect(plural("ru", 1, tr as never, "notif.newCount")).toBe(
      "новое уведомление",
    );
    expect(plural("ru", 3, tr as never, "notif.newCount")).toBe(
      "новых уведомления",
    );
    expect(plural("ru", 5, tr as never, "notif.newCount")).toBe(
      "новых уведомлений",
    );
    expect(plural("ru", 21, tr as never, "notif.newCount")).toBe(
      "новое уведомление",
    );
  });

  it("falls back to the other form for English one/other", () => {
    expect(plural("en", 1, tr as never, "notif.newCount")).toBe(
      "новое уведомление",
    );
    expect(plural("en", 9, tr as never, "notif.newCount")).toBe(
      "новых уведомлений",
    );
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
