export const en = {
  "col.tasks": "My tasks",
  "col.notifications": "Notifications",
  "filter.placeholder": "Filter tasks — id, subject, status, due…",
  "empty.tasks": "No tasks",
  "empty.notifications": "No notifications",
  "error.prefix": "Error",
  "tray.show": "Show",
  "tray.quit": "Quit",
} as const;

export type Key = keyof typeof en;
export type Dict = Record<Key, string>;
