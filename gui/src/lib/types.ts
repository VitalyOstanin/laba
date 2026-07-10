export interface ServerInfo {
  // short name / identifier (the profile key), shown in the switcher
  name: string;
  // full display name (display_name, or the key when unset)
  display_name: string;
  base_url: string;
  backend: "openproject" | "github";
  is_default: boolean;
  // effective poll interval (override or backend default), for display
  poll_secs: number;
  // raw poll-interval override (null = backend default), for the settings input
  poll_override: number | null;
  enabled: boolean;
  // timelog window start, for the settings input
  timelog_start: TimelogStart | null;
  // per-status row tint tokens (status -> "danger"|"warn"|"success"|"progress"|"dimmed")
  status_colors: Record<string, StatusColorToken>;
  // whether this server has a notification inbox (else the column is hidden)
  has_notifications: boolean;
  // whether notification read state can be toggled from the app
  can_toggle_read: boolean;
  // whether tasks have a workflow status worth filtering by (drives the tabs)
  supports_status_filters: boolean;
  // named status filters (label -> statuses) shown as task-list tabs
  status_filters: StatusFilter[];
}

// A named task filter: a label plus the statuses it groups. Shown as a tab.
export interface StatusFilter {
  label: string;
  statuses: string[];
}

// Semantic tint token for a task row; maps to a theme-aware CSS token in the UI.
export type StatusColorToken =
  "danger" | "warn" | "success" | "progress" | "dimmed";

// Normalized task/notification: open-ended maps from core.
export type Task = Record<string, unknown>;
export type Notification = Record<string, unknown>;

// One page of items plus the cursor for the next page. `next_offset` is an
// opaque backend cursor (OpenProject: next 1-based page number; null when the
// collection is exhausted or the backend returns everything at once).
export interface Page<T> {
  items: T[];
  next_offset: number | null;
}

export type Theme = "system" | "dark" | "light";
export type Lang = "system" | "en" | "ru";
export type WeekStart = "system" | "monday" | "sunday";

export interface TimelogStart {
  date: string; // YYYY-MM-DD
  auto: boolean;
}

// App-level preferences. Server-level settings (enabled, poll interval, timelog
// start) live on the server profile in config.json, not here.
export interface Settings {
  theme: Theme;
  language: Lang;
  minimize_to_tray: boolean;
  // first day of the week for week-based grouping; "system" follows the locale
  week_start: WeekStart;
  // IANA timezone name for the day boundary and datetime display; "system" = local
  timezone: string;
  // interface scale factor (1 = no scaling)
  ui_scale: number;
}

export type TimelogState = "red" | "yellow" | "green" | "over";

export interface TimelogStatus {
  logged_min: number;
  planned_min: number;
  today_deficit_min: number;
  deficit_min: number;
  surplus_min: number;
  status: TimelogState;
}

export interface DayCell {
  date: string;
  weekday: boolean;
  plan_min: number;
  logged_min: number;
  deficit_min: number;
  surplus_min: number;
}

export interface TimelogResult {
  configured: boolean;
  status: TimelogStatus;
  timeline: DayCell[];
  start: string;
  start_is_default: boolean;
  excluded: string[];
}

export interface Activity {
  id: number;
  name: string;
}

export interface Candidate {
  server: string;
  wp_id: number;
  subject: string;
  logged_min: number;
}
