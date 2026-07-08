import { writable, derived } from "svelte/store";
import type { ServerInfo, Task, Notification, Settings } from "./types";

export const defaultSettings: Settings = {
  theme: "system",
  language: "system",
  minimize_to_tray: true,
  poll_override: {},
};

export const settings = writable<Settings>(defaultSettings);

/**
 * Return settings with the poll override for `name` set from a raw input value.
 * A blank or non-positive value clears the override (server falls back to its
 * backend default).
 */
export function setPollOverride(
  s: Settings,
  name: string,
  raw: string,
): Settings {
  const n = parseInt(raw, 10);
  const po = { ...s.poll_override };
  if (Number.isFinite(n) && n > 0) po[name] = n;
  else delete po[name];
  return { ...s, poll_override: po };
}

export interface ServerState {
  tasks: Task[];
  notifications: Notification[];
  error: string | null;
}

export type ByServer = Record<string, ServerState>;

export const servers = writable<ServerInfo[]>([]);
export const activeServer = writable<string | null>(null);
export const byServer = writable<ByServer>({});
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

export function totalUnread(by: ByServer): number {
  return Object.values(by).reduce(
    (sum, s) => sum + s.notifications.filter(unreadOf).length,
    0,
  );
}

export const unreadCount = derived(byServer, ($by) => totalUnread($by));
