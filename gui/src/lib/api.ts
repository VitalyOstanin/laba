import { invoke } from "@tauri-apps/api/core";
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
} from "./types";

export const listServers = (): Promise<ServerInfo[]> => invoke("list_servers");
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
