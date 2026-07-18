import { writable, derived } from "svelte/store";
import type {
  ServerInfo,
  Task,
  Notification,
  Settings,
  TimelogResult,
} from "./types";
import type { AvailableUpdate } from "./updater";

export const defaultSettings: Settings = {
  theme: "system",
  language: "system",
  minimize_to_tray: true,
  desktop_notifications: true,
  week_start: "system",
  timezone: "system",
  ui_scale: 1,
  backends_hint_dismissed: false,
  relative_times: false,
  show_notifications: true,
  show_tasks: true,
  show_timelog: true,
};

export const settings = writable<Settings>(defaultSettings);

export const timelog = writable<TimelogResult | null>(null);

/**
 * Parse a raw poll-interval input to seconds, or `undefined` to clear the
 * override (server falls back to its backend default) when blank/non-positive.
 */
export function parsePollSecs(raw: string): number | undefined {
  const n = parseInt(raw, 10);
  return Number.isFinite(n) && n > 0 ? n : undefined;
}

export interface ServerState {
  tasks: Task[];
  notifications: Notification[];
  error: string | null;
  // Next page cursor for each list; null once the list is fully loaded (or the
  // backend returns everything at once, as GitHub does).
  taskCursor: number | null;
  notifCursor: number | null;
}

export type ByServer = Record<string, ServerState>;

/**
 * Cheap per-server summary retained for every enabled server even when its
 * full lists are evicted from `byServer`. Feeds the server-switcher error dot
 * and the aggregate unread count without holding the whole task/notification
 * arrays resident.
 */
export interface ServerSummary {
  error: string | null;
  unread: number;
}

export const servers = writable<ServerInfo[]>([]);
export const activeServer = writable<string | null>(null);
// Full resident state, kept only for the active server; other servers are
// evicted to `summaries` to bound memory by the viewport.
export const byServer = writable<ByServer>({});
export const summaries = writable<Record<string, ServerSummary>>({});
export const filterText = writable<string>("");

/**
 * The update discovered by the startup check (null = none available, or not yet
 * checked). Shared state so the always-visible header indicator and the
 * dismissible update banner reflect a single check.
 */
export const availableUpdate = writable<AvailableUpdate | null>(null);

/**
 * Session flag set by the header update indicator to force the update banner
 * open, overriding a prior "remind later"/"skip" for the current run. The banner
 * clears it when the user dismisses again.
 */
export const updateBannerOpen = writable<boolean>(false);

/**
 * Where a server is in its refresh cycle, for the sync indicator: `syncing` while
 * a poll is in flight, `idle` after a success, `stale` after a failed poll (the
 * last cached data is still shown). `lastSyncMs` is the epoch-ms of the last
 * successful poll, or null before the first.
 */
export type SyncPhase = "syncing" | "idle" | "stale";
export interface SyncInfo {
  phase: SyncPhase;
  lastSyncMs: number | null;
}
export const syncByServer = writable<Record<string, SyncInfo>>({});

/**
 * A server's first page persisted to the webview's local storage so the
 * dashboard shows the last-known tasks and notifications immediately on launch,
 * before the first network poll returns. `savedAtMs` stamps when it was written.
 */
export interface CacheEntry {
  tasks: Task[];
  notifications: Notification[];
  taskCursor: number | null;
  notifCursor: number | null;
  unread: number;
  savedAtMs: number;
}

/** Reduce a resident server state to the persisted cache entry. */
export function toCacheEntry(
  s: ServerState,
  unread: number,
  nowMs: number,
): CacheEntry {
  return {
    tasks: s.tasks,
    notifications: s.notifications,
    taskCursor: s.taskCursor,
    notifCursor: s.notifCursor,
    unread,
    savedAtMs: nowMs,
  };
}

/** Rebuild a resident server state from a cache entry (no error on load). */
export function fromCacheEntry(e: CacheEntry): ServerState {
  return {
    tasks: e.tasks,
    notifications: e.notifications,
    error: null,
    taskCursor: e.taskCursor,
    notifCursor: e.notifCursor,
  };
}

/**
 * Parse a persisted cache entry, tolerating absent or malformed data (returns
 * null). Guards against a corrupt or partial storage value crashing startup.
 */
export function parseCacheEntry(raw: string | null): CacheEntry | null {
  if (!raw) return null;
  try {
    const v = JSON.parse(raw) as Partial<CacheEntry>;
    if (!Array.isArray(v.tasks) || !Array.isArray(v.notifications)) return null;
    return {
      tasks: v.tasks,
      notifications: v.notifications,
      taskCursor: v.taskCursor ?? null,
      notifCursor: v.notifCursor ?? null,
      unread: typeof v.unread === "number" ? v.unread : 0,
      savedAtMs: typeof v.savedAtMs === "number" ? v.savedAtMs : 0,
    };
  } catch {
    return null;
  }
}

/**
 * A parsed filter query: whitespace-separated tokens, where a token prefixed
 * with `-` or `!` excludes matching rows and any other token includes only
 * matching rows. A lone `-`/`!` (no term after it) is ignored. A row passes when
 * it contains every include term and none of the exclude terms.
 */
export interface FilterQuery {
  include: string[];
  exclude: string[];
}

export function parseFilter(text: string): FilterQuery {
  const include: string[] = [];
  const exclude: string[] = [];
  for (const raw of text.trim().toLowerCase().split(/\s+/)) {
    if (!raw) continue;
    if ((raw[0] === "-" || raw[0] === "!") && raw.length > 1) {
      exclude.push(raw.slice(1));
    } else if (raw[0] !== "-" && raw[0] !== "!") {
      include.push(raw);
    }
  }
  return { include, exclude };
}

/**
 * Collect every string/number leaf value of a (possibly nested) row into one
 * lowercase haystack. Recurses into objects (e.g. a task's `id: {display, raw}`)
 * and arrays (labels, custom fields) so the filter matches nested values, not
 * just top-level ones.
 */
function haystack(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "object") {
    return Object.values(value as Record<string, unknown>)
      .map(haystack)
      .join(" ");
  }
  return String(value);
}

/**
 * Include/exclude substring match over all (nested) field values. Empty (or
 * exclude-only-but-blank) query returns every row.
 */
function filterRows<T>(rows: T[], text: string): T[] {
  const { include, exclude } = parseFilter(text);
  if (include.length === 0 && exclude.length === 0) return rows;
  return rows.filter((r) => {
    const hay = haystack(r).toLowerCase();
    return (
      include.every((t) => hay.includes(t)) &&
      !exclude.some((t) => hay.includes(t))
    );
  });
}

/** Filter tasks by a substring over all field values. */
export function filterTasks(tasks: Task[], text: string): Task[] {
  return filterRows(tasks, text);
}

/** Filter notifications by a substring over all field values (subject, reason,
 * project, …), mirroring the task-list filter. */
export function filterNotifications(
  notifications: Notification[],
  text: string,
): Notification[] {
  return filterRows(notifications, text);
}

/**
 * Where a content link (a task or a notification's subject) should open, and
 * whether a secondary "open in browser" control is needed. `openTarget` is the
 * server's effective preference; `app` is only honored when the backend can
 * render a detail screen (`canDetail`). When the app screen is preferred but the
 * item also has a web URL, the secondary browser control is offered so a
 * poor-web-UI server can still open in the browser.
 */
export function contentOpenPlan(p: {
  openTarget: "app" | "browser";
  canDetail: boolean;
  hasHref: boolean;
}): { primary: "app" | "browser" | "none"; secondaryBrowser: boolean } {
  if (p.openTarget === "app" && p.canDetail) {
    return { primary: "app", secondaryBrowser: p.hasHref };
  }
  return { primary: p.hasHref ? "browser" : "none", secondaryBrowser: false };
}

/**
 * Whether a normalized notification is unread.
 *
 * Both backends set `read: boolean` (OpenProject in `normalize::notification_entity`,
 * GitHub in `github::notification_from_gh`, which fetches read items too via
 * `all=true`). The rule is: unread unless explicitly `read === true`.
 */
export function unreadOf(n: Notification): boolean {
  return n.read !== true;
}

/** Notification list view: only unread (triage pending) or everything. */
export type NotifView = "unread" | "all";

/**
 * Filter a notification list for the chosen view: `all` passes everything,
 * `unread` keeps only items not yet read. Pure, so the column can stay thin.
 */
export function notificationsForView(
  list: Notification[],
  view: NotifView,
): Notification[] {
  return view === "all" ? list : list.filter(unreadOf);
}

/** Number of unread notifications in a loaded server state. */
export function unreadIn(s: ServerState): number {
  return s.notifications.filter(unreadOf).length;
}

/**
 * Detect unread notifications that are new since the last poll of a server, for
 * desktop notification. `prevSeen` is the set of unread ids observed on the
 * previous poll, or `undefined` on the first poll (which establishes a baseline
 * silently — no burst of banners for the existing backlog at startup).
 *
 * Returns the fresh notifications to announce and the `seen` set to carry to the
 * next poll. `seen` is the current unread ids only, so a notification marked read
 * elsewhere drops out and would announce again if it later returns to unread.
 */
export function freshUnread(
  prevSeen: Set<string> | undefined,
  notifications: Notification[],
): { fresh: Notification[]; seen: Set<string> } {
  const unread = notifications.filter(unreadOf);
  const seen = new Set(unread.map((n) => String(n.id)));
  const fresh =
    prevSeen === undefined
      ? []
      : unread.filter((n) => !prevSeen.has(String(n.id)));
  return { fresh, seen };
}

/** Reduce a loaded server state to its retained summary. */
export function summarize(s: ServerState): ServerSummary {
  return { error: s.error, unread: unreadIn(s) };
}

/** Aggregate unread across every server's retained summary. */
export function totalUnread(sums: Record<string, ServerSummary>): number {
  return Object.values(sums).reduce((sum, s) => sum + s.unread, 0);
}

export const unreadCount = derived(summaries, ($s) => totalUnread($s));
