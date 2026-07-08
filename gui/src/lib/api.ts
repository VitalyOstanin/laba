import { invoke } from "@tauri-apps/api/core";
import type {
  ServerInfo,
  Task,
  Notification,
  Settings,
  TimelogResult,
  Activity,
  Candidate,
} from "./types";

export const listServers = (): Promise<ServerInfo[]> => invoke("list_servers");
export const listTasks = (server: string): Promise<Task[]> =>
  invoke("list_tasks", { server });
export const listNotifications = (server: string): Promise<Notification[]> =>
  invoke("list_notifications", { server });
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
