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

export type Theme = "system" | "dark" | "light";
export type Lang = "system" | "en" | "ru";

export interface Settings {
  theme: Theme;
  language: Lang;
  minimize_to_tray: boolean;
  // server name -> poll interval override in seconds
  poll_override: Record<string, number>;
}
