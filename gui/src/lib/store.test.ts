import { describe, it, expect } from "vitest";
import { filterTasks, totalUnread, type ByServer } from "./store";

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

describe("totalUnread", () => {
  it("sums unread across servers (read !== true)", () => {
    const byServer: ByServer = {
      // OpenProject: read boolean present.
      a: {
        notifications: [{ read: false }, { read: true }],
        tasks: [],
        error: null,
      },
      // GitHub: no read field — counts as unread.
      b: { notifications: [{ reason: "mention" }], tasks: [], error: null },
    };
    expect(totalUnread(byServer)).toBe(2);
  });
});
