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
  // static backend capabilities (notifications, read toggle, status filters,
  // task detail, custom fields, ...); read via the helpers in ./capabilities
  capabilities: Capabilities;
  // named status filters (label -> statuses) shown as task-list tabs
  status_filters: StatusFilter[];
  // where content (tasks, notifications) opens on click: "app" (laba detail) or
  // "browser" (effective value: per-server override or backend default)
  open_content_in: "app" | "browser";
  // custom-field names shown as extra task-list columns (and sort keys), e.g. Rank
  display_fields: string[];
  // per-server proxy override (URL, "direct", or null = inherit global/env)
  proxy: string | null;
  // whether an OpenProject token is stored (drives "sign in / update token");
  // always false for GitHub (authenticates via gh)
  has_token: boolean;
}

// How a notification's read state can be written from the app (mirrors the Rust
// ReadToggle enum; serde renames the variants to lowercase).
export type ReadToggle = "none" | "oneway" | "twoway";
// Whether a task can be opened for its description and comments (Rust DetailSupport).
export type DetailSupport = "none" | "inapp";
// Time-logging support for a backend (Rust TimelogSupport).
export type TimelogSupport = "none" | "basic" | "withactivities";

// Static capabilities of a backend, mirroring the Rust `Capabilities` struct.
// Nuances are enums (one-way vs two-way read, timelog with/without activities)
// rather than bare booleans; use the helpers in ./capabilities to read them.
export interface Capabilities {
  // the backend exposes a notification inbox
  notifications: boolean;
  // how a notification's read state can be written
  notification_read: ReadToggle;
  // tasks carry a workflow status worth filtering by (drives the status tabs)
  status_filters: boolean;
  // a task can be opened for its description and comments
  task_detail: DetailSupport;
  // tasks carry custom fields shown as extra list columns
  custom_fields: boolean;
  // time logging support
  timelog: TimelogSupport;
  // laba keeps a local assignee history because the server forgets past assignees
  needs_local_history: boolean;
  // where a task opens by default (per-server override notwithstanding)
  default_open_target: "app" | "browser";
  // default polling interval in seconds
  default_poll_secs: number;
}

// One expanded custom field on a task: {key, name, value}. `name` is the human
// field label used both to match a display field and as the column header.
export interface CustomField {
  key: string;
  name: string | null;
  value: unknown;
}

// A task's identity: a human-facing display id (format varies by backend) and
// the raw id the backend uses for its own API calls. Mirrors Rust TaskId.
export interface TaskId {
  display: string;
  raw: string;
}

// What kind of thing a task is (Rust TaskKind, flat token; Other -> raw label).
export type TaskKind = "issue" | "pullRequest" | "workPackage" | (string & {});
// Why a task is in the user's list — the core of "don't miss it" (Rust InboxReason).
export type InboxReason =
  | "assigned"
  | "authored"
  | "reviewRequested"
  | "mentioned"
  | "involved"
  | "own";
// Normalized workflow status bucket for tint/filter (Rust StatusCategory).
export type StatusCategory = "open" | "inProgress" | "done" | "unknown";

// A named task filter: a label plus the statuses it groups. Shown as a tab.
export interface StatusFilter {
  label: string;
  statuses: string[];
}

// Semantic tint token for a task row; maps to a theme-aware CSS token in the UI.
export type StatusColorToken =
  "danger" | "warn" | "success" | "progress" | "dimmed";

// A task (issue / pull request / work package) in the inbox. Mirrors the Rust
// `entities::Task`, serialized camelCase. Every field the backend cannot supply
// is null / empty rather than omitted, so the shape is stable across backends.
export interface Task {
  id: TaskId;
  kind: TaskKind;
  reason: InboxReason;
  title: string;
  url: string | null;
  status: string | null;
  statusCategory: StatusCategory;
  project: string | null;
  // true when the task is in a repository the user owns (GitHub: repo owner ==
  // login); drives the "My repos" vs "All" scope tabs
  mine: boolean;
  assignee: string | null;
  author: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  dueDate: string | null;
  priority: string | null;
  labels: string[];
  customFields: CustomField[];
}

// The task-detail screen (OpenProject only) consumes the raw flattened work
// package — subject, description, customFields, ... — which is a different shape
// from the normalized list `Task`. Kept as an open record until the detail path
// is typed too.
export type WorkPackageDetail = Record<string, unknown>;
// One comment on the detail screen (raw flattened OpenProject activity).
export type WpComment = Record<string, unknown>;

// What a notification is about (Rust NotifKind, flat token; Other -> raw label).
export type NotifKind = "issue" | "pullRequest" | "checkSuite" | (string & {});
// CI (check-suite) run outcome, for tinting (Rust CiOutcome).
export type CiOutcome = "failure" | "success" | "neutral";

// A notification in the inbox. Mirrors the Rust `entities::Notification`. `url`
// is resolved to a concrete target where possible (including a specific CI run).
export interface Notification {
  id: string;
  reason: string;
  kind: NotifKind;
  title: string;
  project: string | null;
  url: string | null;
  updatedAt: string | null;
  read: boolean;
  // CI run outcome, present only for check-suite notifications.
  outcome: CiOutcome | null;
  // Related work package id when the notification opens in-app (OpenProject).
  wpId: number | null;
}

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
  // show a desktop notification when new unread items arrive
  desktop_notifications: boolean;
  // first day of the week for week-based grouping; "system" follows the locale
  week_start: WeekStart;
  // IANA timezone name for the day boundary and datetime display; "system" = local
  timezone: string;
  // interface scale factor (1 = no scaling)
  ui_scale: number;
  // release version dismissed in the update banner (omitted when nothing dismissed)
  dismissed_update_version?: string | null;
  // the user dismissed the "add a server / available backends" hint banner
  backends_hint_dismissed: boolean;
  // show timestamps as a relative label ("5 minutes ago") instead of the
  // absolute zoned datetime; the alternate form is offered on hover either way
  relative_times: boolean;
  // dashboard layout: show the notifications column
  show_notifications: boolean;
  // dashboard layout: show the tasks column
  show_tasks: boolean;
  // dashboard layout: show the time-logged indicator bar
  show_timelog: boolean;
}

// One release in the update changelog, newest first (see core `update`).
export interface ReleaseNote {
  version: string;
  name: string | null;
  body: string;
  published_at: string | null;
}

// gh CLI availability for the GitHub task backend (see core `github`).
export type GhStatus = "ready" | "missing" | "unauthenticated";
export interface GhDependency {
  // a GitHub-backend server is configured, so gh is actually needed
  used: boolean;
  status: GhStatus;
}
// The authenticated gh account: which login on which host (see core `github`).
export interface GhAccount {
  login: string;
  host: string;
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
