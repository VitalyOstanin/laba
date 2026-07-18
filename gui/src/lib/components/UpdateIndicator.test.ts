import "@testing-library/jest-dom/vitest";
import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import { get } from "svelte/store";
import UpdateIndicator from "./UpdateIndicator.svelte";
import { updateState, updateBannerOpen } from "../store";

afterEach(() => {
  cleanup();
  updateState.set({ phase: "checking" });
  updateBannerOpen.set(false);
});

describe("UpdateIndicator", () => {
  it("renders nothing when update checking is disabled", () => {
    updateState.set({ phase: "disabled" });
    render(UpdateIndicator);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows a checking status while the check is in flight", () => {
    updateState.set({ phase: "checking" });
    render(UpdateIndicator);
    expect(screen.getByText("Checking for updates…")).toBeInTheDocument();
  });

  it("confirms the running version is current", () => {
    updateState.set({ phase: "current" });
    render(UpdateIndicator);
    expect(screen.getByText("Up to date")).toBeInTheDocument();
  });

  it("shows the available version and forces the banner open on click", async () => {
    updateState.set({
      phase: "available",
      update: { version: "0.1.7", notes: null },
    });
    render(UpdateIndicator);
    const btn = screen.getByRole("button");
    expect(btn).toHaveTextContent("0.1.7");
    // The indicator stays visible independently of the banner's dismissal; a
    // click re-opens the banner via the shared flag.
    expect(get(updateBannerOpen)).toBe(false);
    await fireEvent.click(btn);
    expect(get(updateBannerOpen)).toBe(true);
  });

  it("offers a retry when the check failed", async () => {
    updateState.set({ phase: "failed" });
    render(UpdateIndicator);
    const btn = screen.getByRole("button");
    expect(btn).toHaveTextContent("Update check failed");
    // Clicking re-runs the check: in the test (no Tauri) it resolves to the dev
    // fixture's available update, so the state leaves the failed phase.
    await fireEvent.click(btn);
    // The click handler kicks off an async check; the phase moves off "failed".
    // (runUpdateCheck sets "checking" synchronously before awaiting.)
    expect(get(updateState).phase).not.toBe("failed");
  });
});
