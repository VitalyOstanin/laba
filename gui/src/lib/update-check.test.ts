import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { get } from "svelte/store";

// Mock the updater bridge so runUpdateCheck can be driven through every outcome
// without a Tauri runtime or network. `vi.hoisted` makes the spy available to
// the hoisted `vi.mock` factory without a top-level-variable reference error.
const { checkForUpdate } = vi.hoisted(() => ({ checkForUpdate: vi.fn() }));
vi.mock("$lib/updater", () => ({ checkForUpdate }));

import { runUpdateCheck } from "./update-check";
import { settings, updateState, defaultSettings } from "./store";

beforeEach(() => {
  checkForUpdate.mockReset();
  settings.set({ ...defaultSettings });
  updateState.set({ phase: "checking" });
});

afterEach(() => {
  settings.set({ ...defaultSettings });
  updateState.set({ phase: "checking" });
});

describe("runUpdateCheck", () => {
  it("goes straight to disabled and skips the check when check_updates is off", async () => {
    settings.set({ ...defaultSettings, check_updates: false });
    await runUpdateCheck();
    expect(get(updateState)).toEqual({ phase: "disabled" });
    expect(checkForUpdate).not.toHaveBeenCalled();
  });

  it("publishes the available update", async () => {
    const update = { version: "0.9.0", notes: null };
    checkForUpdate.mockResolvedValue({ status: "available", update });
    await runUpdateCheck();
    expect(get(updateState)).toEqual({ phase: "available", update });
  });

  it("reports the current phase when on the latest version", async () => {
    checkForUpdate.mockResolvedValue({ status: "current" });
    await runUpdateCheck();
    expect(get(updateState)).toEqual({ phase: "current" });
  });

  it("reports the failed phase when the check errors", async () => {
    checkForUpdate.mockResolvedValue({ status: "failed" });
    await runUpdateCheck();
    expect(get(updateState)).toEqual({ phase: "failed" });
  });

  it("sets checking before awaiting the check", async () => {
    let observed: string | undefined;
    checkForUpdate.mockImplementation(() => {
      observed = get(updateState).phase;
      return Promise.resolve({ status: "current" });
    });
    await runUpdateCheck();
    expect(observed).toBe("checking");
  });
});
