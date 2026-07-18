import "@testing-library/jest-dom/vitest";
import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { render, cleanup, fireEvent, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";

// Mock the poller so the per-server refresh icon can be exercised without a
// backend; a deferred promise lets the test observe the in-flight spinner.
let resolveRefresh: () => void;
vi.mock("../poller", () => ({
  refreshServer: vi.fn(
    () =>
      new Promise<void>((r) => {
        resolveRefresh = r;
      }),
  ),
}));

import { refreshServer } from "../poller";
import { servers, activeServer, summaries } from "../store";
import type { ServerInfo } from "../types";
import ServerSwitcher from "./ServerSwitcher.svelte";

function seedServer(name: string, enabled = true) {
  // The switcher reads only these fields; cast past the full ServerInfo shape.
  const info = {
    name,
    display_name: name.toUpperCase(),
    base_url: "github.com",
    backend: "github",
    enabled,
  } as unknown as ServerInfo;
  servers.set([info]);
  activeServer.set(name);
  summaries.set({});
}

beforeEach(() => {
  seedServer("gh");
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  servers.set([]);
  activeServer.set(null);
  summaries.set({});
});

const refreshBtn = (c: HTMLElement) =>
  c.querySelector<HTMLButtonElement>(".server-refresh")!;

describe("ServerSwitcher refresh icon", () => {
  it("resyncs that server and shows a spinner while in flight", async () => {
    const { container } = render(ServerSwitcher);
    const btn = refreshBtn(container);
    // Idle: the refresh glyph is shown, no spinner.
    expect(container.querySelector(".refresh-glyph")).toBeInTheDocument();
    expect(container.querySelector(".server-refresh .spinner")).toBeNull();

    await fireEvent.click(btn);
    expect(refreshServer).toHaveBeenCalledWith("gh");
    // In flight: spinner replaces the glyph and the button is disabled.
    await waitFor(() =>
      expect(
        container.querySelector(".server-refresh .spinner"),
      ).toBeInTheDocument(),
    );
    expect(btn.disabled).toBe(true);

    resolveRefresh();
    await tick();
    // Settled: glyph returns, button re-enabled.
    await waitFor(() =>
      expect(container.querySelector(".refresh-glyph")).toBeInTheDocument(),
    );
    expect(refreshBtn(container).disabled).toBe(false);
  });

  it("ignores a second click while a refresh is already in flight", async () => {
    const { container } = render(ServerSwitcher);
    const btn = refreshBtn(container);
    await fireEvent.click(btn);
    await fireEvent.click(btn); // disabled + guard: no second call
    expect(refreshServer).toHaveBeenCalledTimes(1);
    resolveRefresh();
    await tick();
  });

  it("shows no refresh icon for a disabled server", async () => {
    seedServer("gh", false);
    const { container } = render(ServerSwitcher);
    expect(container.querySelector(".server-refresh")).toBeNull();
  });
});
