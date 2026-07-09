import { describe, it, expect } from "vitest";
import {
  filterTasks,
  totalUnread,
  summarize,
  unreadIn,
  type ServerState,
  type ServerSummary,
} from "./store";

describe("filterTasks", () => {
  const tasks = [
    { id: "#1", subject: "Fix pagination", status: "open" },
    { id: "#2", subject: "Cache avatars", status: "closed" },
  ];
  it("matches across all fields, case-insensitive", () => {
    expect(filterTasks(tasks, "CLOSED").map((t) => t.id)).toEqual(["#2"]);
    expect(filterTasks(tasks, "pagination").map((t) => t.id)).toEqual(["#1"]);
    expect(filterTasks(tasks, "").length).toBe(2);
  });
});

describe("unread accounting (read !== true)", () => {
  const state = (notifications: ServerState["notifications"]): ServerState => ({
    notifications,
    tasks: [],
    error: null,
    taskCursor: null,
    notifCursor: null,
  });

  it("unreadIn counts everything not explicitly read", () => {
    // OpenProject: read boolean present. GitHub: no read field = unread.
    expect(unreadIn(state([{ read: false }, { read: true }]))).toBe(1);
    expect(unreadIn(state([{ reason: "mention" }]))).toBe(1);
  });

  it("summarize reduces a state to error + unread", () => {
    const s: ServerState = { ...state([{ read: false }]), error: "boom" };
    expect(summarize(s)).toEqual({ error: "boom", unread: 1 });
  });

  it("totalUnread sums across server summaries", () => {
    const sums: Record<string, ServerSummary> = {
      a: { error: null, unread: 1 },
      b: { error: null, unread: 1 },
    };
    expect(totalUnread(sums)).toBe(2);
  });
});
