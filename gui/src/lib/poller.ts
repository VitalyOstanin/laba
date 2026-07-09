import { get } from "svelte/store";
import { listServers, listTasks, listNotifications, getTimelog } from "./api";
import {
  servers,
  byServer,
  summaries,
  activeServer,
  timelog,
  unreadOf,
  type ServerState,
} from "./store";
import type { ServerInfo } from "./types";

const timers = new Map<string, ReturnType<typeof setInterval>>();
let timelogTimer: ReturnType<typeof setInterval> | undefined;
let resumeHandler: (() => void) | undefined;
let resuming = false;
let unsubActive: (() => void) | undefined;

/** Aggregate timelog refresh interval (seconds). */
const TIMELOG_INTERVAL_SECS = 120;

/** Load the server list, seed the active server, and start per-server polling. */
export async function startPolling(): Promise<void> {
  const list = await listServers();
  servers.set(list);
  const enabled = list.filter((s) => s.enabled);
  if (
    get(activeServer) === null ||
    !enabled.some((s) => s.name === get(activeServer))
  ) {
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

  // Switching servers loads the newly active one in full and evicts the
  // previously active one's arrays back to a summary, bounding resident data.
  unsubActive = activeServer.subscribe((name) => {
    if (name) void onActivate(name);
  });

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
  if (unsubActive) {
    unsubActive();
    unsubActive = undefined;
  }
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

/** Load the newly activated server in full if its arrays are not resident. */
async function onActivate(name: string): Promise<void> {
  // Evict every other server's resident arrays; their summaries remain.
  byServer.update((by) => {
    const next: typeof by = {};
    if (by[name]) next[name] = by[name];
    return next;
  });
  if (!get(byServer)[name]) {
    const s = get(servers).find((x) => x.name === name);
    if (s) await pollOnce(s);
  }
}

/**
 * Refresh one server's first page. The active server keeps its full arrays and
 * page cursors resident in `byServer`; other servers retain only a summary
 * (error flag + unread count) so memory stays bounded by the viewport. On
 * failure old data and the last summary are kept and the error is recorded.
 */
async function pollOnce(s: ServerInfo): Promise<void> {
  const isActive = get(activeServer) === s.name;
  try {
    const [tasks, notifs] = await Promise.all([
      listTasks(s.name, 1),
      listNotifications(s.name, 1),
    ]);
    const unread = notifs.items.filter(unreadOf).length;
    summaries.update((m) => ({ ...m, [s.name]: { error: null, unread } }));
    if (isActive) {
      byServer.update((by) => ({
        ...by,
        [s.name]: {
          tasks: tasks.items,
          notifications: notifs.items,
          error: null,
          taskCursor: tasks.next_offset,
          notifCursor: notifs.next_offset,
        },
      }));
    } else {
      // Not active: drop any resident arrays; the summary above is enough.
      byServer.update((by) => {
        if (!(s.name in by)) return by;
        const { [s.name]: _drop, ...rest } = by;
        return rest;
      });
    }
  } catch (e) {
    summaries.update((m) => ({
      ...m,
      [s.name]: { error: String(e), unread: m[s.name]?.unread ?? 0 },
    }));
    if (isActive) {
      byServer.update((by) => ({
        ...by,
        [s.name]: {
          tasks: by[s.name]?.tasks ?? [],
          notifications: by[s.name]?.notifications ?? [],
          error: String(e),
          taskCursor: by[s.name]?.taskCursor ?? null,
          notifCursor: by[s.name]?.notifCursor ?? null,
        },
      }));
    }
  }
}

/** Append the next page of tasks for a resident server, following its cursor. */
export async function loadMoreTasks(name: string): Promise<void> {
  await loadMore(name, "tasks");
}

/** Append the next page of notifications for a resident server. */
export async function loadMoreNotifications(name: string): Promise<void> {
  await loadMore(name, "notifications");
}

const loading = new Set<string>();

async function loadMore(
  name: string,
  which: "tasks" | "notifications",
): Promise<void> {
  const state = get(byServer)[name];
  if (!state) return;
  const cursor = which === "tasks" ? state.taskCursor : state.notifCursor;
  if (cursor === null) return;
  const key = `${name}:${which}`;
  if (loading.has(key)) return;
  loading.add(key);
  try {
    const page =
      which === "tasks"
        ? await listTasks(name, cursor)
        : await listNotifications(name, cursor);
    byServer.update((by) => {
      const cur = by[name];
      if (!cur) return by;
      const merged: ServerState =
        which === "tasks"
          ? {
              ...cur,
              tasks: [...cur.tasks, ...page.items],
              taskCursor: page.next_offset,
            }
          : {
              ...cur,
              notifications: [...cur.notifications, ...page.items],
              notifCursor: page.next_offset,
            };
      return { ...by, [name]: merged };
    });
    if (which === "notifications") {
      const st = get(byServer)[name];
      if (st) {
        const unread = st.notifications.filter(unreadOf).length;
        summaries.update((m) => ({
          ...m,
          [name]: { error: st.error, unread },
        }));
      }
    }
  } finally {
    loading.delete(key);
  }
}
