export const en = {
  "col.tasks": "My tasks",
  "col.notifications": "Notifications",
  "filter.placeholder": "Filter tasks — id, subject, status, due…",
  "empty.tasks": "No tasks",
  "empty.notifications": "No notifications",
  "error.prefix": "Error",
  "tray.show": "Show",
  "tray.quit": "Quit",
  "nav.dashboard": "Dashboard",
  "nav.settings": "Settings",
  "settings.title": "Settings",
  "settings.theme": "Theme",
  "settings.theme.system": "System",
  "settings.theme.dark": "Dark",
  "settings.theme.light": "Light",
  "settings.language": "Language",
  "settings.language.system": "System",
  "settings.language.en": "English",
  "settings.language.ru": "Русский",
  "settings.tray": "Hide to tray when closing the window",
  "settings.poll": "Poll interval per server (seconds)",
  "settings.poll.hint": "Leave blank to use the backend default.",
  "settings.saved": "Saved",
} as const;

export type Key = keyof typeof en;
export type Dict = Record<Key, string>;
