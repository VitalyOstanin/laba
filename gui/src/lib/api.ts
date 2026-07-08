import { invoke } from "@tauri-apps/api/core";
import type { ServerInfo, Task, Notification } from "./types";

export const listServers = (): Promise<ServerInfo[]> => invoke("list_servers");
export const listTasks = (server: string): Promise<Task[]> =>
  invoke("list_tasks", { server });
export const listNotifications = (server: string): Promise<Notification[]> =>
  invoke("list_notifications", { server });
