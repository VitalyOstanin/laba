import { describe, it, expect } from "vitest";
import {
  hasNotifications,
  canToggleRead,
  supportsStatusFilters,
  supportsCustomFields,
  supportsTaskDetail,
} from "./capabilities";
import type { Capabilities, ServerInfo } from "./types";

function server(caps: Partial<Capabilities>): ServerInfo {
  const capabilities: Capabilities = {
    notifications: true,
    notification_read: "twoway",
    status_filters: true,
    task_detail: "inapp",
    custom_fields: true,
    timelog: "withactivities",
    needs_local_history: true,
    default_open_target: "app",
    default_poll_secs: 120,
    ...caps,
  };
  return {
    name: "s",
    display_name: "s",
    base_url: "https://example.test",
    backend: "openproject",
    is_default: true,
    poll_secs: 120,
    poll_override: null,
    enabled: true,
    timelog_start: null,
    status_colors: {},
    capabilities,
    status_filters: [],
    open_content_in: "app",
    display_fields: [],
    proxy: null,
    has_token: false,
  };
}

describe("capabilities readers", () => {
  it("read booleans straight through", () => {
    expect(hasNotifications(server({ notifications: false }))).toBe(false);
    expect(supportsStatusFilters(server({ status_filters: false }))).toBe(
      false,
    );
    expect(supportsCustomFields(server({ custom_fields: false }))).toBe(false);
  });

  it("map the read-toggle enum to a boolean", () => {
    expect(canToggleRead(server({ notification_read: "none" }))).toBe(false);
    expect(canToggleRead(server({ notification_read: "oneway" }))).toBe(true);
    expect(canToggleRead(server({ notification_read: "twoway" }))).toBe(true);
  });

  it("map the task-detail enum to a boolean", () => {
    expect(supportsTaskDetail(server({ task_detail: "none" }))).toBe(false);
    expect(supportsTaskDetail(server({ task_detail: "inapp" }))).toBe(true);
  });

  it("fall back safely for an undefined server", () => {
    expect(hasNotifications(undefined)).toBe(true);
    expect(canToggleRead(undefined)).toBe(false);
    expect(supportsStatusFilters(undefined)).toBe(false);
    expect(supportsCustomFields(undefined)).toBe(false);
    expect(supportsTaskDetail(undefined)).toBe(false);
  });
});
