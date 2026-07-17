// Pure readers over a server's nested `capabilities` object. They replace the
// former flat `supports_*` / `can_toggle_read` booleans on ServerInfo, keeping
// the enum-to-boolean logic (read toggle, task detail) in one tested place.
import type { ServerInfo } from "./types";

// Whether this server exposes a notification inbox (else the column is hidden).
export function hasNotifications(
  server: ServerInfo | undefined | null,
): boolean {
  return server?.capabilities.notifications ?? true;
}

// Whether a notification's read state can be written from the app (any of the
// one-way / two-way toggles). Drives the read dot and "mark all read".
export function canToggleRead(server: ServerInfo | undefined | null): boolean {
  return (server?.capabilities.notification_read ?? "none") !== "none";
}

// Whether tasks carry a workflow status worth filtering by (drives the tabs).
export function supportsStatusFilters(
  server: ServerInfo | undefined | null,
): boolean {
  return server?.capabilities.status_filters ?? false;
}

// Whether tasks carry custom fields (drives the display-fields editor).
export function supportsCustomFields(
  server: ServerInfo | undefined | null,
): boolean {
  return server?.capabilities.custom_fields ?? false;
}

// Whether a task opens an in-app detail screen (description + comments).
export function supportsTaskDetail(
  server: ServerInfo | undefined | null,
): boolean {
  return (server?.capabilities.task_detail ?? "none") !== "none";
}
