import { get } from "svelte/store";
import { listServers, listTasks, listNotifications, getTimelog } from "./api";
import { servers, byServer, activeServer, timelog } from "./store";
import type { ServerInfo } from "./types";

const timers = new Map<string, ReturnType<typeof setInterval>>();
let timelogTimer: ReturnType<typeof setInterval> | undefined;
let resumeHandler: (() => void) | undefined;
let resuming = false;

/** Aggregate timelog refresh interval (seconds). */
const TIMELOG_INTERVAL_SECS = 120;

/** Load the server list, seed the active server, and start per-server polling. */
export async function startPolling(): Promise<void> {
  const list = await listServers();
  servers.set(list);
  const enabled = list.filter((s) => s.enabled);
  if (get(activeServer) === null || !enabled.some((s) => s.name === get(activeServer))) {
    const def = enabled.find((s) => s.is_default) ?? enabled[0];
    activeServer.set(def ? def.name : null);
  }
  for (const s of enabled) {
    void pollOnce(s);
    const id = setInterval(() => void pollOnce(s), s.poll_secs * 1000);
    timers.set(s.name, id);
  }
  void refreshTimelog();
  timelogTimer = setInterval(
    () => void refreshTimelog(),
    TIMELOG_INTERVAL_SECS * 1000,
  );

  // setInterval timers are suspended while the system sleeps and resume only on
  // the next tick, so data is stale for up to one interval after wake. Refresh
  // immediately when the window regains focus or connectivity is restored.
  resumeHandler = () => void refreshAll();
  window.addEventListener("focus", resumeHandler);
  window.addEventListener("online", resumeHandler);
}

export function stopPolling(): void {
  for (const id of timers.values()) clearInterval(id);
  timers.clear();
  if (timelogTimer) clearInterval(timelogTimer);
  timelogTimer = undefined;
  if (resumeHandler) {
    window.removeEventListener("focus", resumeHandler);
    window.removeEventListener("online", resumeHandler);
    resumeHandler = undefined;
  }
}

/** Refresh every enabled server and the aggregate timelog at once. */
export async function refreshAll(): Promise<void> {
  if (resuming) return;
  resuming = true;
  try {
    const enabled = get(servers).filter((s) => s.enabled);
    await Promise.all([...enabled.map((s) => pollOnce(s)), refreshTimelog()]);
  } finally {
    resuming = false;
  }
}

/** Refresh a single server now (after a write action). */
export async function refreshServer(name: string): Promise<void> {
  const s = get(servers).find((x) => x.name === name);
  if (s) await pollOnce(s);
}

/** Refresh the aggregate timelog; keep the last value on failure. */
export async function refreshTimelog(): Promise<void> {
  try {
    timelog.set(await getTimelog());
  } catch {
    // Keep the previous value on transient errors.
  }
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
