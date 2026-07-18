import { describe, it, expect, afterEach, vi } from "vitest";
import {
  shouldShowUpdate,
  canSelfUpdate,
  checkForUpdate,
  DEV_FIXTURE_VERSION,
} from "./updater";

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

describe("checkForUpdate", () => {
  afterEach(() => {
    vi.resetModules();
    vi.unstubAllGlobals();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (globalThis.window as any).__TAURI_INTERNALS__;
  });

  it("returns the dev fixture when no Tauri runtime is present", async () => {
    const res = await checkForUpdate();
    expect(res).toEqual({
      status: "available",
      update: {
        version: DEV_FIXTURE_VERSION,
        notes: "Example release notes for the dev mock.",
      },
    });
  });

  // With a Tauri runtime, the outcome mirrors the plugin's check(): a handle →
  // "available", null → "current", a thrown error → "failed". `hasTauri` is a
  // module-load constant, so each case re-imports the module with the flag set.
  async function withTauri(
    checkImpl: () => Promise<unknown>,
  ): Promise<Awaited<ReturnType<typeof checkForUpdate>>> {
    vi.resetModules();
    (
      globalThis.window as unknown as Record<string, unknown>
    ).__TAURI_INTERNALS__ = {};
    vi.doMock("@tauri-apps/plugin-updater", () => ({ check: checkImpl }));
    const mod = await import("./updater");
    return mod.checkForUpdate();
  }

  it("reports available when the plugin returns an update handle", async () => {
    const res = await withTauri(() =>
      Promise.resolve({ version: "1.2.3", body: "notes" }),
    );
    expect(res).toEqual({
      status: "available",
      update: { version: "1.2.3", notes: "notes" },
    });
  });

  it("reports current when the plugin returns no update", async () => {
    const res = await withTauri(() => Promise.resolve(null));
    expect(res).toEqual({ status: "current" });
  });

  it("reports failed when the plugin throws", async () => {
    const res = await withTauri(() => Promise.reject(new Error("network")));
    expect(res).toEqual({ status: "failed" });
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
