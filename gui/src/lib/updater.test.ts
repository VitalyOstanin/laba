import { describe, it, expect, afterEach } from "vitest";
import { shouldShowUpdate, canSelfUpdate } from "./updater";

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

describe("canSelfUpdate", () => {
  const realNavigator = globalThis.navigator;

  afterEach(() => {
    Object.defineProperty(globalThis, "navigator", {
      value: realNavigator,
      configurable: true,
    });
  });

  function setUserAgent(ua: string): void {
    Object.defineProperty(globalThis, "navigator", {
      value: { userAgent: ua },
      configurable: true,
    });
  }

  it("disables self-update on macOS", () => {
    setUserAgent(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
    );
    expect(canSelfUpdate()).toBe(false);
  });

  it("enables self-update on Linux and Windows", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36");
    expect(canSelfUpdate()).toBe(true);
    setUserAgent(
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    );
    expect(canSelfUpdate()).toBe(true);
  });
});
