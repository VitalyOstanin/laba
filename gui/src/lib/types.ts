export interface ServerInfo {
  name: string;
  base_url: string;
  backend: "openproject" | "github";
  is_default: boolean;
  poll_secs: number;
  enabled: boolean;
}

// Normalized task/notification: open-ended maps from core.
export type Task = Record<string, unknown>;
export type Notification = Record<string, unknown>;

export type Theme = "system" | "dark" | "light";
export type Lang = "system" | "en" | "ru";

export interface TimelogStart {
  date: string; // YYYY-MM-DD
  auto: boolean;
}

export interface Settings {
  theme: Theme;
  language: Lang;
  minimize_to_tray: boolean;
  // server name -> poll interval override in seconds
  poll_override: Record<string, number>;
  // server name -> timelog window start
  timelog_start: Record<string, TimelogStart>;
  // server names temporarily disabled in the GUI
  disabled_servers: string[];
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
