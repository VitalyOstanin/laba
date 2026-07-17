import { describe, it, expect } from "vitest";
import {
  filterTasks,
  filterNotifications,
  parseFilter,
  contentOpenPlan,
  toCacheEntry,
  fromCacheEntry,
  parseCacheEntry,
  totalUnread,
  summarize,
  unreadIn,
  freshUnread,
  unreadOf,
  notificationsForView,
  type ServerState,
  type ServerSummary,
} from "./store";

describe("unreadOf", () => {
  it("treats an item as unread unless read === true", () => {
    expect(unreadOf({ id: "1" })).toBe(true);
    expect(unreadOf({ id: "2", read: false })).toBe(true);
    expect(unreadOf({ id: "3", read: true })).toBe(false);
  });
});

describe("notificationsForView", () => {
  const list = [
    { id: "1", read: true, subject: "handled" },
    { id: "2", read: false, subject: "pending" },
    { id: "3", subject: "no flag" },
  ];
  it("all view passes everything", () => {
    expect(notificationsForView(list, "all").map((n) => n.id)).toEqual([
      "1",
      "2",
      "3",
    ]);
  });
  it("unread view keeps only unread items", () => {
    expect(notificationsForView(list, "unread").map((n) => n.id)).toEqual([
      "2",
      "3",
    ]);
  });
});

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

describe("filterNotifications", () => {
  const notifs = [
    { id: 1, reason: "mentioned", subject: "Fix login redirect" },
    { id: 2, reason: "ci_activity", subject: "CI workflow run failed" },
  ];
  it("matches across all fields, case-insensitive", () => {
    expect(filterNotifications(notifs, "CI").map((n) => n.id)).toEqual([2]);
    expect(filterNotifications(notifs, "login").map((n) => n.id)).toEqual([1]);
    expect(filterNotifications(notifs, "mentioned").map((n) => n.id)).toEqual([
      1,
    ]);
    expect(filterNotifications(notifs, "").length).toBe(2);
  });
});

describe("parseFilter", () => {
  it("splits include and exclude tokens", () => {
    expect(parseFilter("ci -passed")).toEqual({
      include: ["ci"],
      exclude: ["passed"],
    });
    expect(parseFilter("  CI  -PASSED ")).toEqual({
      include: ["ci"],
      exclude: ["passed"],
    });
    expect(parseFilter("!draft")).toEqual({ include: [], exclude: ["draft"] });
  });
  it("ignores a lone minus or bang", () => {
    expect(parseFilter("- ! ci")).toEqual({ include: ["ci"], exclude: [] });
  });
  it("treats a blank query as no filter", () => {
    expect(parseFilter("   ")).toEqual({ include: [], exclude: [] });
  });
});

describe("contentOpenPlan", () => {
  it("opens in the app for a poor-web-UI server with a detail screen", () => {
    expect(
      contentOpenPlan({ openTarget: "app", canDetail: true, hasHref: true }),
    ).toEqual({ primary: "app", secondaryBrowser: true });
  });
  it("keeps app primary without a browser fallback when no URL", () => {
    expect(
      contentOpenPlan({ openTarget: "app", canDetail: true, hasHref: false }),
    ).toEqual({ primary: "app", secondaryBrowser: false });
  });
  it("opens in the browser for a good-web-UI server", () => {
    expect(
      contentOpenPlan({
        openTarget: "browser",
        canDetail: false,
        hasHref: true,
      }),
    ).toEqual({ primary: "browser", secondaryBrowser: false });
  });
  it("falls back to browser when app is preferred but no detail screen", () => {
    expect(
      contentOpenPlan({ openTarget: "app", canDetail: false, hasHref: true }),
    ).toEqual({ primary: "browser", secondaryBrowser: false });
  });
  it("is inert when neither app nor URL is available", () => {
    expect(
      contentOpenPlan({ openTarget: "app", canDetail: false, hasHref: false }),
    ).toEqual({ primary: "none", secondaryBrowser: false });
  });
});

describe("cache entry round-trip", () => {
  const state = {
    tasks: [{ id: "#1", subject: "A" }],
    notifications: [{ id: 1, subject: "N" }],
    error: null,
    taskCursor: 2,
    notifCursor: null,
  };
  it("reduces state to an entry and back, dropping error", () => {
    const entry = toCacheEntry(state, 3, 1000);
    expect(entry).toEqual({
      tasks: state.tasks,
      notifications: state.notifications,
      taskCursor: 2,
      notifCursor: null,
      unread: 3,
      savedAtMs: 1000,
    });
    expect(fromCacheEntry(entry)).toEqual({ ...state, error: null });
  });
  it("parses a stored entry and round-trips through JSON", () => {
    const entry = toCacheEntry(state, 3, 1000);
    expect(parseCacheEntry(JSON.stringify(entry))).toEqual(entry);
  });
  it("returns null for missing or malformed data", () => {
    expect(parseCacheEntry(null)).toBeNull();
    expect(parseCacheEntry("not json")).toBeNull();
    expect(parseCacheEntry("{}")).toBeNull();
    expect(parseCacheEntry('{"tasks":[]}')).toBeNull();
  });
  it("defaults optional numeric fields when absent", () => {
    const parsed = parseCacheEntry('{"tasks":[],"notifications":[]}');
    expect(parsed).toEqual({
      tasks: [],
      notifications: [],
      taskCursor: null,
      notifCursor: null,
      unread: 0,
      savedAtMs: 0,
    });
  });
});

describe("filter include/exclude semantics", () => {
  const notifs = [
    { id: 1, reason: "ci_activity", subject: "CI workflow run failed" },
    { id: 2, reason: "ci_activity", subject: "CI workflow run passed" },
    { id: 3, reason: "mentioned", subject: "review requested" },
  ];
  it("excludes rows containing a -term", () => {
    expect(filterNotifications(notifs, "ci -passed").map((n) => n.id)).toEqual([
      1,
    ]);
  });
  it("requires every include term", () => {
    expect(filterTasks(notifs, "ci run").map((n) => n.id)).toEqual([1, 2]);
  });
  it("exclude-only removes matches, keeps the rest", () => {
    expect(filterTasks(notifs, "-mentioned").map((n) => n.id)).toEqual([1, 2]);
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

describe("freshUnread", () => {
  it("first poll (no prior seen) establishes a baseline without announcing", () => {
    const { fresh, seen } = freshUnread(undefined, [
      { id: 1, read: false },
      { id: 2, read: false },
    ]);
    expect(fresh).toEqual([]);
    expect(seen).toEqual(new Set(["1", "2"]));
  });

  it("announces only unread ids not seen on the previous poll", () => {
    const prev = new Set(["1"]);
    const { fresh, seen } = freshUnread(prev, [
      { id: 1, read: false },
      { id: 2, read: false },
      { id: 3, read: true },
    ]);
    expect(fresh.map((n) => (n as Record<string, unknown>).id)).toEqual([2]);
    // seen tracks current unread only; the read id 3 is not carried.
    expect(seen).toEqual(new Set(["1", "2"]));
  });

  it("a read notification drops from seen and re-announces if it returns unread", () => {
    const afterRead = freshUnread(new Set(["1"]), [{ id: 1, read: true }]);
    expect(afterRead.fresh).toEqual([]);
    expect(afterRead.seen).toEqual(new Set());
    const backToUnread = freshUnread(afterRead.seen, [{ id: 1, read: false }]);
    expect(
      backToUnread.fresh.map((n) => (n as Record<string, unknown>).id),
    ).toEqual([1]);
  });
});
