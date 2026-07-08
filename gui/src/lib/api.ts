import { invoke } from "@tauri-apps/api/core";
import type { ServerInfo, Task, Notification, Settings } from "./types";

export const listServers = (): Promise<ServerInfo[]> => invoke("list_servers");
export const listTasks = (server: string): Promise<Task[]> =>
  invoke("list_tasks", { server });
export const listNotifications = (server: string): Promise<Notification[]> =>
  invoke("list_notifications", { server });
export const getSettings = (): Promise<Settings> => invoke("get_settings");
export const saveSettings = (settings: Settings): Promise<void> =>
  invoke("save_settings", { settings });
