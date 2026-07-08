import { get } from "svelte/store";
import { listServers, listTasks, listNotifications } from "./api";
import { servers, byServer, activeServer } from "./store";
import type { ServerInfo } from "./types";

const timers = new Map<string, ReturnType<typeof setInterval>>();

/** Load the server list, seed the active server, and start per-server polling. */
export async function startPolling(): Promise<void> {
  const list = await listServers();
  servers.set(list);
  if (get(activeServer) === null) {
    const def = list.find((s) => s.is_default) ?? list[0];
    activeServer.set(def ? def.name : null);
  }
  for (const s of list) {
    void pollOnce(s);
    const id = setInterval(() => void pollOnce(s), s.poll_secs * 1000);
    timers.set(s.name, id);
  }
}

export function stopPolling(): void {
  for (const id of timers.values()) clearInterval(id);
  timers.clear();
}

/** Refresh one server; on failure keep old data and record the error. */
async function pollOnce(s: ServerInfo): Promise<void> {
  try {
    const [tasks, notifications] = await Promise.all([
      listTasks(s.name),
      listNotifications(s.name),
    ]);
    byServer.update((by) => ({
      ...by,
      [s.name]: { tasks, notifications, error: null },
    }));
  } catch (e) {
    byServer.update((by) => ({
      ...by,
      [s.name]: {
        tasks: by[s.name]?.tasks ?? [],
        notifications: by[s.name]?.notifications ?? [],
        error: String(e),
      },
    }));
  }
}
