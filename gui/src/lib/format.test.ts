import { describe, it, expect } from "vitest";
import { fmtDateTime, fmtRelative } from "./format";

describe("fmtDateTime", () => {
  it("renders an absolute date + time in a named zone", () => {
    // 09:30 UTC is 12:30 in Europe/Moscow (+03:00).
    const s = fmtDateTime("2026-07-10T09:30:00Z", "en", "Europe/Moscow");
    expect(s).toContain("12:30");
    expect(s).toContain("2026");
  });

  it("system zone falls back to the local zone (no throw)", () => {
    const s = fmtDateTime("2026-07-10T09:30:00Z", "en", "system");
    expect(s).toContain("2026");
  });

  it("an unknown IANA zone falls back to local instead of throwing", () => {
    const s = fmtDateTime("2026-07-10T09:30:00Z", "en", "Not/AZone");
    expect(s).toContain("2026");
  });

  it("passes unparseable input through unchanged", () => {
    expect(fmtDateTime("not-a-date")).toBe("not-a-date");
    expect(fmtDateTime("")).toBe("");
  });
});

describe("fmtRelative", () => {
  const now = Date.parse("2026-07-10T12:00:00Z");

  it("picks minutes for a few-minute-old timestamp", () => {
    expect(fmtRelative("2026-07-10T11:55:00Z", "en", now)).toBe(
      "5 minutes ago",
    );
  });

  it("picks hours", () => {
    expect(fmtRelative("2026-07-10T09:00:00Z", "en", now)).toBe("3 hours ago");
  });

  it("uses the numeric:auto label for yesterday", () => {
    expect(fmtRelative("2026-07-09T12:00:00Z", "en", now)).toBe("yesterday");
  });

  it("handles future timestamps", () => {
    expect(fmtRelative("2026-07-10T12:05:00Z", "en", now)).toBe("in 5 minutes");
  });

  it("passes unparseable input through unchanged", () => {
    expect(fmtRelative("nope", "en", now)).toBe("nope");
  });
});
