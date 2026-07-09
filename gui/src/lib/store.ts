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
  week_start: "monday",
  poll_override: {},
  timelog_start: {},
  disabled_servers: [],
};

export const settings = writable<Settings>(defaultSettings);

export const timelog = writable<TimelogResult | null>(null);

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

/** Enable or disable a server without removing its profile. */
export function setServerEnabled(
  s: Settings,
  name: string,
  enabled: boolean,
): Settings {
  const set = new Set(s.disabled_servers);
  if (enabled) set.delete(name);
  else set.add(name);
  return { ...s, disabled_servers: [...set].sort() };
}

/** Set a server's timelog start date, clearing the auto (first-launch) flag. */
export function setTimelogStart(
  s: Settings,
  name: string,
  date: string,
): Settings {
  const ts = { ...s.timelog_start };
  if (date) ts[name] = { date, auto: false };
  else delete ts[name];
  return { ...s, timelog_start: ts };
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
