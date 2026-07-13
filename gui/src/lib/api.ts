import { invoke } from "./invoke";
import type {
  ServerInfo,
  Task,
  Notification,
  Page,
  Settings,
  TimelogResult,
  Activity,
  Candidate,
  StatusColorToken,
  StatusFilter,
  ReleaseNote,
  GhDependency,
} from "./types";

export const listServers = (): Promise<ServerInfo[]> => invoke("list_servers");
// Cumulative changelog (versions newer than the running one) for the update
// banner's "what's new".
export const getChangelog = (): Promise<ReleaseNote[]> =>
  invoke("get_changelog");
// Whether the gh CLI dependency (GitHub task backend only) is satisfied, so the
// UI can show a friendly install/login hint.
export const ghDependency = (): Promise<GhDependency> =>
  invoke("gh_dependency");
// Show desktop notifications for newly-arrived unread items. `target` is an
// opaque routing payload echoed back on click (Linux) via the `open-notification`
// event, so the frontend decides where a click leads.
export interface NotifyItem {
  title: string;
  body: string;
  target: unknown;
}
export const notifyItems = (items: NotifyItem[]): Promise<void> =>
  invoke("notify_items", { items });
// Window/app controls for keyboard shortcuts. quitApp exits unconditionally;
// closeWindow runs the normal close flow (hide to tray or quit per settings).
export const quitApp = (): Promise<void> => invoke("quit_app");
export const closeWindow = (): Promise<void> => invoke("close_window");
export const listTasks = (
  server: string,
  page?: number,
  pageSize?: number,
): Promise<Page<Task>> => invoke("list_tasks", { server, page, pageSize });
export const listNotifications = (
  server: string,
  page?: number,
  pageSize?: number,
): Promise<Page<Notification>> =>
  invoke("list_notifications", { server, page, pageSize });
// Per-server profile editors (config.json). `name` is the server's short name
// / identifier. Pass null to clear an optional value.
export const setServerDisplayName = (
  name: string,
  displayName: string | null,
): Promise<void> => invoke("set_server_display_name", { name, displayName });
export const setServerEnabled = (
  name: string,
  enabled: boolean,
): Promise<void> => invoke("set_server_enabled", { name, enabled });
export const setServerPollSecs = (
  name: string,
  pollSecs: number | null,
): Promise<void> => invoke("set_server_poll_secs", { name, pollSecs });
export const setServerTimelogStart = (
  name: string,
  date: string | null,
): Promise<void> => invoke("set_server_timelog_start", { name, date });
// Set (token) or clear (null) the row tint for a workflow status on a server.
export const setServerStatusColor = (
  name: string,
  status: string,
  color: StatusColorToken | null,
): Promise<void> => invoke("set_server_status_color", { name, status, color });
// Replace a server's named status filters (task-list tabs). Empty clears them.
export const setServerStatusFilters = (
  name: string,
  filters: StatusFilter[],
): Promise<void> => invoke("set_server_status_filters", { name, filters });
// Replace a server's display fields (extra task-list columns / sort keys), an
// ordered list of custom-field names. Empty clears them.
export const setServerDisplayFields = (
  name: string,
  fields: string[],
): Promise<void> => invoke("set_server_display_fields", { name, fields });
// Per-server proxy override. Empty/null clears it (inherit global/env); a URL
// routes through that proxy; "direct" forces a direct connection.
export const setServerProxy = (
  name: string,
  proxy: string | null,
): Promise<void> => invoke("set_server_proxy", { name, proxy });
// Global default proxy (applies to servers without their own override).
export const getGlobalProxy = (): Promise<string | null> =>
  invoke("get_global_proxy");
export const setGlobalProxy = (proxy: string | null): Promise<void> =>
  invoke("set_global_proxy", { proxy });
export const renameServer = (old: string, next: string): Promise<void> =>
  invoke("rename_server", { old, new: next });
// Add a server profile. backend "github" needs no token (uses gh); "openproject"
// needs a token set separately (keyring/CLI).
export const addServer = (
  name: string,
  url: string,
  backend: "openproject" | "github",
  displayName: string | null,
): Promise<void> => invoke("add_server", { name, url, backend, displayName });
// Validate and store an OpenProject token for a server (keyring/file store),
// entered from the GUI instead of the CLI. Rejects a duplicate account unless
// force. GitHub servers authenticate via gh and are rejected by the backend.
export const loginServer = (
  name: string,
  token: string,
  force: boolean,
): Promise<void> => invoke("login_server", { name, token, force });

export const getSettings = (): Promise<Settings> => invoke("get_settings");
export const saveSettings = (settings: Settings): Promise<void> =>
  invoke("save_settings", { settings });
export const getTimelog = (): Promise<TimelogResult> => invoke("get_timelog");
export const setNotificationRead = (
  server: string,
  id: number,
  read: boolean,
): Promise<void> => invoke("set_notification_read", { server, id, read });
export const markAllRead = (server: string): Promise<number> =>
  invoke("mark_all_read", { server });
export const addComment = (
  server: string,
  workPackage: number,
  text: string,
): Promise<void> => invoke("add_comment", { server, workPackage, text });
// Task-detail screen: one work package with description + custom fields, and its
// comment thread (oldest first). OpenProject only (supports_task_detail).
export const getTaskDetail = (server: string, id: number): Promise<Task> =>
  invoke("get_task_detail", { server, id });
export const listTaskComments = (
  server: string,
  id: number,
): Promise<Notification[]> => invoke("list_task_comments", { server, id });
// Push the aggregate attention count (unread + red-tab tasks) to the tray icon.
export const setTrayStatus = (count: number): Promise<void> =>
  invoke("set_tray_status", { count });
export const listActivities = (server: string): Promise<Activity[]> =>
  invoke("list_activities", { server });
export const createTimeEntry = (
  server: string,
  workPackage: number,
  duration: string,
  comment: string | null,
  activity: string | null,
): Promise<void> =>
  invoke("create_time_entry", {
    server,
    workPackage,
    duration,
    comment,
    activity,
  });
export const pickCandidates = (): Promise<Candidate[]> =>
  invoke("pick_candidates");
