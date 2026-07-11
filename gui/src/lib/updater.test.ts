import { describe, it, expect } from "vitest";
import { shouldShowUpdate } from "./updater";

describe("shouldShowUpdate", () => {
  const avail = { version: "0.2.0", notes: null };

  it("hides when no update is available", () => {
    expect(shouldShowUpdate(null, null)).toBe(false);
    expect(shouldShowUpdate(null, "0.1.0")).toBe(false);
  });

  it("shows an available update that was not dismissed", () => {
    expect(shouldShowUpdate(avail, null)).toBe(true);
    expect(shouldShowUpdate(avail, undefined)).toBe(true);
    expect(shouldShowUpdate(avail, "0.1.9")).toBe(true);
  });

  it("hides the exact version the user dismissed", () => {
    expect(shouldShowUpdate(avail, "0.2.0")).toBe(false);
  });

  it("shows again when a newer version supersedes the dismissed one", () => {
    expect(shouldShowUpdate({ version: "0.3.0", notes: null }, "0.2.0")).toBe(
      true,
    );
  });
});
