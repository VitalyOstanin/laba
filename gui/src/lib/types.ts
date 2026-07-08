export interface ServerInfo {
  name: string;
  base_url: string;
  backend: "openproject" | "github";
  is_default: boolean;
  poll_secs: number;
}

// Normalized task/notification: open-ended maps from core.
export type Task = Record<string, unknown>;
export type Notification = Record<string, unknown>;
