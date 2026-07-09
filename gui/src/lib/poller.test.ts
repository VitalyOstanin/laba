import { describe, it, expect, beforeEach, vi } from "vitest";

// Mock the Tauri command layer so the poller exercises pure store logic.
vi.mock("./api", () => ({
  listServers: vi.fn(),
  listTasks: vi.fn(async () => []),
  listNotifications: vi.fn(async () => []),
  getTimelog: vi.fn(async () => null),
}));

import * as api from "./api";
import { refreshAll } from "./poller";
import { servers, byServer } from "./store";
import type { ServerInfo } from "./types";

function srv(name: string, enabled: boolean): ServerInfo {
  return {
    name,
    base_url: `https://${name}`,
    backend: "openproject",
    enabled,
    is_default: false,
    poll_secs: 60,
  } as ServerInfo;
}

describe("refreshAll", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    servers.set([]);
    byServer.set({});
  });

  it("polls only enabled servers and the timelog", async () => {
    servers.set([srv("a", true), srv("b", false), srv("c", true)]);
    await refreshAll();
    const polled = vi.mocked(api.listTasks).mock.calls.map((c) => c[0]).sort();
    expect(polled).toEqual(["a", "c"]);
    expect(api.getTimelog).toHaveBeenCalledTimes(1);
  });

  it("guards against overlapping runs", async () => {
    servers.set([srv("a", true)]);
    let release: () => void = () => {};
    vi.mocked(api.listTasks).mockImplementationOnce(
      () => new Promise((r) => (release = () => r([]))),
    );
    const first = refreshAll();
    const second = refreshAll(); // must return early while `first` is pending
    release();
    await Promise.all([first, second]);
    expect(api.getTimelog).toHaveBeenCalledTimes(1);
  });
});
