/**
 * Browser dev-mode mock for the Tauri `invoke` bridge. When the app runs under
 * `vite dev` in a plain browser (no Tauri runtime), `$lib/invoke` routes calls
 * here so UI work iterates with fixtures and hot reload — without the container
 * build. It never ships: `$lib/invoke` only selects it when the Tauri runtime is
 * absent.
 *
 * Fictional sample data only — no real project/domain terms.
 */
import type {
  ServerInfo,
  Settings,
  Task,
  Notification,
  CustomField,
  WorkPackageDetail,
  WpComment,
  TimelogResult,
  Activity,
  Candidate,
  ReleaseNote,
  GhDependency,
} from "./types";

// --- mutable in-memory state (a session's edits are reflected until reload) ---

// Seed profiles the dashboard starts with. Kept as a constant so the setup
// wizard (see `wizardDemo` below) can re-add one by name and get its fixtures.
const SEED_SERVERS: ServerInfo[] = [
  {
    name: "demo",
    display_name: "Demo Tracker",
    base_url: "https://demo.example/op",
    backend: "openproject",
    is_default: true,
    poll_secs: 120,
    poll_override: null,
    enabled: true,
    timelog_start: { date: "2026-07-01", auto: false },
    status_colors: {
      "In progress": "progress",
      "Under review": "warn",
      Blocked: "danger",
      Done: "success",
    },
    capabilities: {
      notifications: true,
      notification_read: "twoway",
      status_filters: true,
      task_detail: "inapp",
      custom_fields: true,
      timelog: "withactivities",
      needs_local_history: true,
      default_open_target: "app",
      default_poll_secs: 120,
    },
    status_filters: [
      { label: "Active", statuses: ["In progress"] },
      { label: "Review", statuses: ["Under review"] },
      { label: "Blocked", statuses: ["Blocked"] },
      { label: "Done", statuses: ["Done"] },
    ],
    open_content_in: "app",
    display_fields: ["Rank"],
    proxy: null,
    has_token: true,
  },
  {
    name: "oss",
    display_name: "OSS Issues",
    base_url: "https://github.com",
    backend: "github",
    is_default: false,
    poll_secs: 900,
    poll_override: null,
    enabled: true,
    timelog_start: null,
    status_colors: {},
    capabilities: {
      notifications: true,
      notification_read: "oneway",
      status_filters: false,
      task_detail: "none",
      custom_fields: false,
      timelog: "none",
      needs_local_history: false,
      default_open_target: "browser",
      default_poll_secs: 900,
    },
    status_filters: [],
    open_content_in: "browser",
    display_fields: [],
    proxy: null,
    has_token: false,
  },
];

// `?demo=wizard` starts with no servers so the first-run setup wizard opens (for
// the recorded demo). Any other load starts from the seed profiles.
const wizardDemo =
  typeof globalThis.location !== "undefined" &&
  /(?:^|[?&])demo=wizard(?:&|$)/.test(globalThis.location.search);

// Strip the flag from the address bar so a recorded demo does not show the
// ?demo=wizard query (it has already been read into `wizardDemo`).
if (wizardDemo && typeof globalThis.history !== "undefined") {
  globalThis.history.replaceState({}, "", globalThis.location.pathname);
}

let servers: ServerInfo[] = wizardDemo
  ? []
  : SEED_SERVERS.map((s) => ({ ...s }));

let settings: Settings = {
  theme: "system",
  language: "system",
  minimize_to_tray: true,
  desktop_notifications: true,
  week_start: "system",
  timezone: "system",
  ui_scale: 1,
  backends_hint_dismissed: false,
  relative_times: false,
  show_notifications: true,
  show_tasks: true,
  show_timelog: true,
  check_updates: true,
};

let globalProxy: string | null = null;

// Cumulative changelog fixture so the update banner renders under `npm run dev`.
// Placeholder version reported in browser dev mode (no real bundle version).
const DEV_APP_VERSION = "0.0.0-dev";

const CHANGELOG: ReleaseNote[] = [
  {
    version: "0.2.0",
    name: "0.2.0 — Timeline & proxies",
    body: "- Per-day timelog timeline panel\n- SOCKS5 / HTTP proxy support, per server and global\n- Faster startup",
    published_at: "2026-02-01T10:00:00Z",
  },
  {
    version: "0.1.5",
    name: "0.1.5 — Fixes",
    body: "- Tray badge no longer flickers\n- Settings save is now instant",
    published_at: "2026-01-15T10:00:00Z",
  },
];

// gh dependency fixture so the hint banner renders under `npm run dev`. Flip
// `status` to "ready" to hide it, or "unauthenticated" for the login variant.
const GH_DEPENDENCY: GhDependency = { used: true, status: "missing" };

// --- fixtures keyed by server ------------------------------------------------

function rank(n: number): CustomField {
  return { key: "customField1", name: "Rank", value: n };
}

// A typed OpenProject-style work-package task for the demo tracker.
function wp(
  id: number,
  title: string,
  status: string,
  assignee: string,
  updatedAt: string,
  cfRank: number,
): Task {
  return {
    id: { display: `#${id}`, raw: String(id) },
    kind: "workPackage",
    reason: "assigned",
    title,
    url: null,
    status,
    statusCategory: "unknown",
    project: "Demo Tracker",
    mine: false,
    assignee,
    author: null,
    createdAt: null,
    updatedAt,
    dueDate: null,
    priority: null,
    labels: [],
    customFields: [rank(cfRank)],
  };
}

// A typed GitHub-style issue task. `mine` marks a repo the user owns, so the
// dev server can show the "My repos" vs "All" scope split.
function ghIssue(
  repo: string,
  n: number,
  title: string,
  updatedAt: string,
  mine = false,
): Task {
  return {
    id: { display: `${repo}#${n}`, raw: String(n) },
    kind: "issue",
    reason: mine ? "own" : "involved",
    title,
    url: `https://github.com/${repo}/issues/${n}`,
    status: "open",
    statusCategory: "open",
    project: repo,
    mine,
    assignee: null,
    author: null,
    createdAt: null,
    updatedAt,
    dueDate: null,
    priority: null,
    labels: [],
    customFields: [],
  };
}

const TASKS: Record<string, Task[]> = {
  demo: [
    wp(
      101,
      "Fix login redirect loop on session expiry",
      "In progress",
      "Sam Rivera",
      "2026-07-10T09:20:00Z",
      1,
    ),
    wp(
      102,
      "Add dark-mode tokens to the settings screen",
      "Under review",
      "Sam Rivera",
      "2026-07-09T14:05:00Z",
      2,
    ),
    wp(
      103,
      "Timeout on large export",
      "Blocked",
      "Lee Park",
      "2026-07-08T11:00:00Z",
      3,
    ),
    wp(
      104,
      "Document the release checklist",
      "Done",
      "Sam Rivera",
      "2026-07-05T16:30:00Z",
      4,
    ),
  ],
  oss: [
    // Two in the user's own repo (mine), two in a repo they only follow.
    ghIssue(
      "octo-dev/laba",
      214,
      "Add a quiet-hours setting",
      "2026-07-11T09:00:00Z",
      true,
    ),
    ghIssue(
      "octo-dev/laba",
      208,
      "Flaky test on CI runner",
      "2026-07-09T16:20:00Z",
      true,
    ),
    ghIssue(
      "acme/tool",
      5521,
      "Crash on empty config file",
      "2026-07-10T08:00:00Z",
    ),
    ghIssue(
      "acme/tool",
      5510,
      "Docs: clarify the proxy precedence",
      "2026-07-07T12:00:00Z",
    ),
  ],
};

// The detail screen (OpenProject only) fetches the raw work package by id; keep
// its description + custom fields here, separate from the normalized list Task.
const DETAILS: Record<number, WorkPackageDetail> = {
  101: {
    subject: "Fix login redirect loop on session expiry",
    description:
      "When a session expires mid-request the app redirects back to the\nexpired page, causing a **loop**.\n\n## Steps to reproduce\n\n1. Let the token lapse\n2. Click any nav item\n3. Observe the redirect bounce\n\nWorkaround: clear `localStorage.session` and reload. See the [tracker](https://example.com/issues/101) for logs.",
    customFields: [rank(1)],
  },
  102: {
    subject: "Add dark-mode tokens to the settings screen",
    description: "Route the remaining literal colors through theme tokens.",
    customFields: [rank(2)],
  },
  103: {
    subject: "Timeout on large export",
    description: "Exports over ~10k rows time out. Blocked on the batch API.",
    customFields: [rank(3)],
  },
  104: {
    subject: "Document the release checklist",
    description: "",
    customFields: [rank(4)],
  },
};

// A typed notification pointing at a demo work package (OpenProject in-app).
function wpNotif(
  id: number,
  reason: string,
  wpId: number,
  title: string,
  read: boolean,
  updatedAt: string,
): Notification {
  return {
    id: String(id),
    reason,
    kind: "workPackage",
    title,
    project: "Demo Tracker",
    url: null,
    updatedAt,
    read,
    outcome: null,
    wpId,
  };
}

const NOTIFICATIONS: Record<string, Notification[]> = {
  demo: [
    wpNotif(
      9001,
      "mentioned",
      101,
      "Fix login redirect loop on session expiry",
      false,
      "2026-07-10T09:25:00Z",
    ),
    wpNotif(
      9002,
      "assigned",
      103,
      "Timeout on large export",
      false,
      "2026-07-09T10:10:00Z",
    ),
    wpNotif(
      9003,
      "commented",
      104,
      "Document the release checklist",
      true,
      "2026-07-05T17:00:00Z",
    ),
  ],
  oss: [
    {
      id: "7001",
      reason: "review_requested",
      kind: "issue",
      title: "Crash on empty config file",
      project: "acme/app",
      url: "https://github.com/acme/app/issues/5521",
      updatedAt: "2026-07-10T08:05:00Z",
      read: false,
      outcome: null,
      wpId: null,
    },
    {
      id: "7002",
      reason: "ci_activity",
      kind: "checkSuite",
      title: "CI workflow run failed for deps/keyring-core branch",
      project: "acme/app",
      url: "https://github.com/acme/app/actions/runs/29414263745",
      updatedAt: "2026-07-10T07:40:00Z",
      read: false,
      outcome: "failure",
      wpId: null,
    },
    {
      id: "7003",
      reason: "ci_activity",
      kind: "checkSuite",
      title: "CI workflow run succeeded for main branch",
      project: "acme/app",
      url: "https://github.com/acme/app/actions/runs/29414200001",
      updatedAt: "2026-07-10T06:15:00Z",
      read: true,
      outcome: "success",
      wpId: null,
    },
  ],
};

const COMMENTS: Record<number, WpComment[]> = {
  101: [
    {
      id: 1,
      user: "Lee Park",
      createdAt: "2026-07-09T15:00:00Z",
      comment: "I can reproduce this on staging.",
    },
    {
      id: 2,
      user: "Sam Rivera",
      createdAt: "2026-07-10T09:20:00Z",
      comment: "Looking into the token refresh path.",
    },
  ],
};

const ACTIVITIES: Activity[] = [
  { id: 1, name: "Development" },
  { id: 2, name: "Review" },
  { id: 3, name: "Meeting" },
];

const CANDIDATES: Candidate[] = [
  {
    server: "demo",
    wp_id: 101,
    subject: "Fix login redirect loop on session expiry",
    logged_min: 90,
  },
  {
    server: "demo",
    wp_id: 102,
    subject: "Add dark-mode tokens to the settings screen",
    logged_min: 0,
  },
];

const TIMELOG: TimelogResult = {
  configured: true,
  start: "2026-07-01",
  start_is_default: false,
  excluded: ["oss"],
  status: {
    logged_min: 1860,
    planned_min: 2400,
    today_deficit_min: 60,
    deficit_min: 540,
    surplus_min: 0,
    status: "yellow",
  },
  timeline: [
    {
      date: "2026-07-06",
      weekday: true,
      plan_min: 480,
      logged_min: 480,
      deficit_min: 0,
      surplus_min: 0,
    },
    {
      date: "2026-07-07",
      weekday: true,
      plan_min: 480,
      logged_min: 450,
      deficit_min: 30,
      surplus_min: 0,
    },
    {
      date: "2026-07-08",
      weekday: true,
      plan_min: 480,
      logged_min: 480,
      deficit_min: 0,
      surplus_min: 0,
    },
    {
      date: "2026-07-09",
      weekday: true,
      plan_min: 480,
      logged_min: 300,
      deficit_min: 180,
      surplus_min: 0,
    },
    {
      date: "2026-07-10",
      weekday: true,
      plan_min: 480,
      logged_min: 150,
      deficit_min: 330,
      surplus_min: 0,
    },
  ],
};

// --- helpers -----------------------------------------------------------------

function server(name: string): ServerInfo | undefined {
  return servers.find((s) => s.name === name);
}

function page<T>(items: T[]) {
  return { items, next_offset: null };
}

function taskDetailById(id: number): WorkPackageDetail | undefined {
  return DETAILS[id];
}

// --- the mock bridge ---------------------------------------------------------

export async function mockInvoke(
  cmd: string,
  args: Record<string, unknown> = {},
): Promise<unknown> {
  const a = args;
  switch (cmd) {
    case "list_servers":
      return servers;
    case "list_tasks":
      return page(TASKS[a.server as string] ?? []);
    case "list_notifications":
      return page(NOTIFICATIONS[a.server as string] ?? []);
    case "get_task_detail":
      return taskDetailById(Number(a.id)) ?? {};
    case "list_task_comments":
      return COMMENTS[Number(a.id)] ?? [];
    case "get_settings":
      return settings;
    case "save_settings":
      settings = args.settings as Settings;
      return null;
    case "get_timelog":
      // Mirror the backend: no enabled time-tracking (OpenProject) server means
      // the indicator does not apply, so return null to hide it.
      return servers.some((s) => s.enabled && s.backend === "openproject")
        ? TIMELOG
        : null;
    case "app_version":
      return DEV_APP_VERSION;
    case "get_changelog":
      return CHANGELOG;
    case "gh_dependency":
      return GH_DEPENDENCY;
    case "get_global_proxy":
      return globalProxy;
    case "set_global_proxy":
      globalProxy = (a.proxy as string | null) ?? null;
      return null;
    case "list_activities":
      return ACTIVITIES;
    case "pick_candidates":
      return CANDIDATES;
    case "mark_all_read": {
      const list = NOTIFICATIONS[a.server as string] ?? [];
      list.forEach((n) => (n.read = true));
      return list.length;
    }
    case "set_notification_read": {
      const list = NOTIFICATIONS[a.server as string] ?? [];
      const n = list.find((x) => Number(x.id) === Number(a.id));
      if (n) n.read = a.read as boolean;
      return null;
    }
    // Per-server profile edits: mutate the in-memory server so the settings
    // screen reflects the change until reload.
    case "set_server_enabled":
      patchServer(
        a.server ?? a.name,
        (s) => (s.enabled = a.enabled as boolean),
      );
      return null;
    case "set_server_display_name":
      patchServer(
        a.name,
        (s) => (s.display_name = (a.displayName as string) || s.name),
      );
      return null;
    case "set_server_poll_secs":
      patchServer(
        a.name,
        (s) => (s.poll_override = (a.pollSecs as number | null) ?? null),
      );
      return null;
    case "set_server_proxy":
      patchServer(
        a.name,
        (s) => (s.proxy = (a.proxy as string | null) ?? null),
      );
      return null;
    case "set_server_open_content_in":
      patchServer(a.name, (s) => {
        const t = a.target as string | null;
        s.open_content_in =
          t === "app" || t === "browser"
            ? t
            : s.capabilities.task_detail !== "none"
              ? "app"
              : "browser";
      });
      return null;
    case "set_server_display_fields":
      patchServer(
        a.name,
        (s) => (s.display_fields = (args.fields as string[]) ?? []),
      );
      return null;
    case "set_server_status_filters":
      patchServer(
        a.name,
        (s) =>
          (s.status_filters =
            (args.filters as ServerInfo["status_filters"]) ?? []),
      );
      return null;
    case "set_server_timelog_start":
      patchServer(a.name, (s) => {
        s.timelog_start = a.date
          ? { date: a.date as string, auto: false }
          : null;
      });
      return null;
    // Setup wizard: actually add the profile so the dashboard fills in after the
    // wizard closes. A seed name (e.g. "demo") brings its fixtures along.
    case "add_server": {
      const nm = String(a.name ?? "");
      if (nm && !server(nm)) {
        const seed = SEED_SERVERS.find((s) => s.name === nm);
        const added: ServerInfo = seed
          ? { ...seed, is_default: servers.length === 0 }
          : {
              name: nm,
              display_name: (a.displayName as string) || nm,
              base_url: (a.url as string) || "",
              backend: (a.backend as ServerInfo["backend"]) || "github",
              is_default: servers.length === 0,
              poll_secs: 120,
              poll_override: null,
              enabled: true,
              timelog_start: null,
              status_colors: {},
              capabilities:
                ((a.backend as ServerInfo["backend"]) || "github") === "github"
                  ? {
                      notifications: true,
                      notification_read: "oneway",
                      status_filters: false,
                      task_detail: "none",
                      custom_fields: false,
                      timelog: "none",
                      needs_local_history: false,
                      default_open_target: "browser",
                      default_poll_secs: 900,
                    }
                  : {
                      notifications: true,
                      notification_read: "twoway",
                      status_filters: true,
                      task_detail: "inapp",
                      custom_fields: true,
                      timelog: "withactivities",
                      needs_local_history: true,
                      default_open_target: "app",
                      default_poll_secs: 120,
                    },
              status_filters: [],
              open_content_in: "browser",
              display_fields: [],
              proxy: null,
              has_token: false,
            };
        servers = [...servers, added];
      }
      return null;
    }
    // Setup wizard: mark the OpenProject profile as signed in.
    case "login_server":
      patchServer(a.name, (s) => (s.has_token = true));
      return null;
    case "remove_server": {
      const nm = String(a.name ?? "");
      const wasDefault = server(nm)?.is_default ?? false;
      servers = servers.filter((s) => s.name !== nm);
      // Hand the default to the first remaining profile, mirroring the backend.
      if (wasDefault && servers.length > 0) {
        servers = servers.map((s, i) => ({ ...s, is_default: i === 0 }));
      }
      return null;
    }
    case "set_default_server": {
      const nm = String(a.name ?? "");
      servers = servers.map((s) => ({ ...s, is_default: s.name === nm }));
      return null;
    }
    case "logout_server":
      patchServer(a.name, (s) => (s.has_token = false));
      return null;
    // Actions with no UI-visible fixture state: acknowledge and move on.
    case "set_server_status_color":
    case "rename_server":
    case "add_comment":
    case "create_time_entry":
    case "set_tray_status":
    case "quit_app":
    case "close_window":
      return null;
    default:
      console.warn(`[dev-mock] unhandled command: ${cmd}`, args);
      return null;
  }
}

function patchServer(name: unknown, f: (s: ServerInfo) => void): void {
  const s = server(String(name));
  if (s) {
    f(s);
    servers = [...servers];
  }
}
