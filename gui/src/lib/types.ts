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
export type WeekStart = "monday" | "sunday";

export interface TimelogStart {
  date: string; // YYYY-MM-DD
  auto: boolean;
}

export interface Settings {
  theme: Theme;
  language: Lang;
  minimize_to_tray: boolean;
  // first day of the week for week-based grouping
  week_start: WeekStart;
  // IANA timezone name for the day boundary and datetime display; null = local
  timezone: string | null;
  // interface scale in whole percent (100 = default)
  ui_scale: number;
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
