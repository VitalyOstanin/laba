// Typed fixture factories for tests: build a full Task / Notification from a
// partial override, so a test names only the fields it cares about.
import type { Task, Notification } from "./types";

export function makeTask(over: Partial<Task> = {}): Task {
  return {
    id: { display: "#1", raw: "1" },
    kind: "workPackage",
    reason: "assigned",
    title: "",
    url: null,
    status: null,
    statusCategory: "unknown",
    project: null,
    mine: false,
    assignee: null,
    author: null,
    createdAt: null,
    updatedAt: null,
    dueDate: null,
    priority: null,
    labels: [],
    customFields: [],
    ...over,
  };
}

export function makeNotif(over: Partial<Notification> = {}): Notification {
  return {
    id: "1",
    reason: "",
    kind: "issue",
    title: "",
    project: null,
    url: null,
    updatedAt: null,
    read: false,
    outcome: null,
    wpId: null,
    ...over,
  };
}
