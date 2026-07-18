import "@testing-library/jest-dom/vitest";
import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup, fireEvent } from "@testing-library/svelte";
import { get } from "svelte/store";
import UpdateIndicator from "./UpdateIndicator.svelte";
import { availableUpdate, updateBannerOpen } from "../store";

afterEach(() => {
  cleanup();
  availableUpdate.set(null);
  updateBannerOpen.set(false);
});

describe("UpdateIndicator", () => {
  it("renders nothing when no update is available", () => {
    availableUpdate.set(null);
    render(UpdateIndicator);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("shows the available version and forces the banner open on click", async () => {
    availableUpdate.set({ version: "0.1.7", notes: null });
    render(UpdateIndicator);
    const btn = screen.getByRole("button");
    expect(btn).toHaveTextContent("0.1.7");
    // The indicator stays visible independently of the banner's dismissal; a
    // click re-opens the banner via the shared flag.
    expect(get(updateBannerOpen)).toBe(false);
    await fireEvent.click(btn);
    expect(get(updateBannerOpen)).toBe(true);
  });
});
