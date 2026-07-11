import { get } from "svelte/store";
import {
  listServers,
  listTasks,
  listNotifications,
  getTimelog,
  notifyItems,
  type NotifyItem,
} from "./api";
import {
  servers,
  byServer,
  summaries,
  activeServer,
  timelog,
  settings,
  unreadOf,
  freshUnread,
  type ServerState,
} from "./store";
import { t } from "./i18n";
import { goto } from "$app/navigation";
import { openExternal } from "./external";
import type { ServerInfo, Notification } from "./types";

// Unread notification ids observed on the previous poll of each server, so a
// desktop notification fires only for ids that are new (see `freshUnread`).
const seenUnread = new Map<string, Set<string>>();
// More than this many new items at once collapse into a single summary banner
// instead of one banner per item.
const NOTIFY_COLLAPSE = 3;

/**
 * Desktop-notify the unread items that are new since the last poll of `s`. The
 * first poll only establishes a baseline (no startup burst); the settings toggle
 * suppresses banners but the baseline is still tracked so enabling it later does
 * not dump the whole backlog.
 */
function maybeNotify(s: ServerInfo, notifications: Notification[]): void {
  const prev = seenUnread.get(s.name);
  const { fresh, seen } = freshUnread(prev, notifications);
  seenUnread.set(s.name, seen);
  if (prev === undefined || fresh.length === 0) return;
  if (!get(settings).desktop_notifications) return;
  // A notification failure must never abort the poll (which also refreshes the
  // task/notification lists), so swallow any error here.
  try {
    const items = buildNotifyItems(s, fresh);
    if (items.length > 0) void notifyItems(items).catch(() => {});
  } catch (e) {
    console.error("desktop notification failed", e);
  }
}

/** Route a click on a desktop notification to the item it announced. Mirrors the
 * in-app targets: an OpenProject task opens its detail screen, a GitHub item
 * opens in the browser, a summary just focuses the server. */
function routeNotification(payload: unknown): void {
  const p = payload as Record<string, unknown> | null;
  if (!p || typeof p.kind !== "string") return;
  if (p.kind === "external" && typeof p.url === "string") {
    void openExternal(p.url);
    return;
  }
  if (typeof p.server === "string") activeServer.set(p.server);
  if (p.kind === "task" && p.server != null && p.id != null) {
    void goto(
      `/task?server=${encodeURIComponent(String(p.server))}&id=${encodeURIComponent(String(p.id))}`,
    );
  } else {
    void goto("/");
  }
}

/** Subscribe to click-through events from Linux desktop notifications. No-op in
 * a plain browser (dev-mock) where the Tauri event bus is absent. */
async function registerNotificationClick(): Promise<void> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window))
    return;
  const { listen } = await import("@tauri-apps/api/event");
  await listen("open-notification", (e) => routeNotification(e.payload));
}

/**
 * Turn fresh notifications into banner items. A click target routes the banner:
 * an OpenProject item opens its detail screen, a GitHub item opens the issue/PR
 * in the browser, anything else just focuses the server. Many at once collapse
 * into one summary that focuses the server.
 */
function buildNotifyItems(s: ServerInfo, fresh: Notification[]): NotifyItem[] {
  const label = s.display_name || s.name;
  if (fresh.length > NOTIFY_COLLAPSE) {
    const suffix = get(t)("notif.newCountSuffix");
    return [
      {
        title: label,
        body: `${fresh.length} ${suffix}`,
        target: { kind: "server", server: s.name },
      },
    ];
  }
  return fresh.map((raw) => {
    const n = raw as Record<string, unknown>;
    const subject = String(n.wpTitle ?? n.subject ?? "");
    const reason = n.reason ? `${String(n.reason)}: ` : "";
    const wpId = Number(n.wpId);
    let target: unknown;
    if (s.supports_task_detail && Number.isFinite(wpId)) {
      target = { kind: "task", server: s.name, id: wpId };
    } else if (typeof n.htmlUrl === "string" && n.htmlUrl) {
      target = { kind: "external", url: n.htmlUrl };
    } else {
      target = { kind: "server", server: s.name };
    }
    return { title: label, body: reason + subject, target };
  });
}

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

  // Route clicks on Linux desktop notifications to the item they announced.
  void registerNotificationClick();
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
  try {
    const [tasks, notifs] = await Promise.all([
      listTasks(s.name, 1),
      listNotifications(s.name, 1),
    ]);
    const unread = notifs.items.filter(unreadOf).length;
    summaries.update((m) => ({ ...m, [s.name]: { error: null, unread } }));
    // Announce items that became unread since the last poll (all servers, not
    // just the active one).
    maybeNotify(s, notifs.items);
    // Re-read after the await: the user may have switched servers while the
    // requests were in flight. Deciding by the pre-await snapshot would let a
    // stale poll leave resident arrays on a now-inactive server.
    if (get(activeServer) === s.name) {
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
    if (get(activeServer) === s.name) {
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
