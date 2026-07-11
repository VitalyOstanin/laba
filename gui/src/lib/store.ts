import { writable, derived } from "svelte/store";
import type {
  ServerInfo,
  Task,
  Notification,
  Settings,
  TimelogResult,
} from "./types";

export const defaultSettings: Settings = {
  theme: "system",
  language: "system",
  minimize_to_tray: true,
  desktop_notifications: true,
  week_start: "system",
  timezone: "system",
  ui_scale: 1,
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

/** Substring match over the concatenation of all field values. */
export function filterTasks(tasks: Task[], text: string): Task[] {
  const q = text.trim().toLowerCase();
  if (!q) return tasks;
  return tasks.filter((t) =>
    Object.values(t)
      .map((v) => String(v ?? ""))
      .join(" ")
      .toLowerCase()
      .includes(q),
  );
}

/**
 * Whether a normalized notification is unread.
 *
 * OpenProject sets `read: boolean` (`core::normalize::notification`); GitHub
 * notifications carry no read field and `gh api notifications` returns only
 * unread ones. So the rule is: unread unless explicitly `read === true`.
 */
export function unreadOf(n: Notification): boolean {
  return (n as Record<string, unknown>)["read"] !== true;
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
  const seen = new Set(unread.map((n) => String((n as Record<string, unknown>).id)));
  const fresh =
    prevSeen === undefined
      ? []
      : unread.filter(
          (n) => !prevSeen.has(String((n as Record<string, unknown>).id)),
        );
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
