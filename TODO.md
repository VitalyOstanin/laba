# TODO

Backlog of ideas to evaluate. Not commitments.

## Desktop integration

- [ ] Integrate with the OS notification system (native desktop notifications):
      surface new/unread items as system notifications (freedesktop/`org.freedesktop.Notifications`
      on Linux, native on macOS/Windows), with click-through to the item.

## Dates / timezone

- [ ] Work out timezone handling for dates and times. Today only the `spentOn`
      default uses the machine's local date (`chrono::Local::now`); API datetimes
      (`createdAt`/`updatedAt`, notification times) pass through as opaque ISO 8601
      strings with no conversion. Decide and document: which timezone defines the
      "today" / day boundary for the timelog plan (requirements 13-16, 21) —
      user-local, per-server, or UTC — especially when aggregating across servers
      in different zones. Timezone also matters for **displaying** timestamps:
      render API datetimes in the user's zone (with a configurable override),
      consistently across CLI `--human` output and the GUI.
- [ ] Add a setting for the first day of the week: an explicit choice (e.g.
      Monday/Sunday) or `auto`, which derives it from the system locale. This
      affects week-based grouping and any "this week" ranges in the timelog and
      dashboards.

## UX

- [ ] Work out UX consistency conventions and apply them across the app. Example:
      a single, predictable reaction to ESC across screens and input fields
      (e.g. ESC clears/blurs a focused input, then closes the current
      panel/dialog, then falls back to the window's default) rather than ad-hoc
      per-widget behavior. Cover other cross-cutting interactions (Enter to
      submit, focus order, unsaved-changes prompts) in the same convention.

## UI testing

- [ ] End-to-end UI tests via the official Tauri WebDriver path: `tauri-driver`
      driving the built native app through WebdriverIO (or Selenium). This runs
      the real webview + Rust backend + native window, not a browser-only mock.
      Platform drivers: WebKitWebDriver on Linux, the Edge WebView2 driver on
      Windows (macOS has no official WebDriver support yet); run headless in CI
      under xvfb. Keep the fast layers alongside it: Rust/CLI logic tests (already
      present) and frontend component/unit tests. Verify the exact `tauri-driver`
      setup against the pinned Tauri version at implementation time.

## Documentation

- [ ] Produce polished, eye-catching screenshots for the README (dashboard,
      settings, work-log timeline with candidate tasks). Drive them from **mocked
      data** (a demo/fixture mode so no live servers are needed), showing **both
      backends together — an OpenProject server and a public GitHub server**. Use
      neutral, fictional sample data only — no real project/domain terms. Consider
      a light/dark pair and a short animated capture of the log-time flow.
      Implies a mock/demo data source the app can render for captures.
- [ ] Record a short demo video (screencast) in addition to the screenshots:
      walk through the dashboard, the work-log timeline, and logging time from a
      candidate task. Drive it from the same mocked demo data (OpenProject +
      public GitHub), neutral fictional content only.

## Observability / debugging

- [ ] Request tracing in `core`: add `tracing` + `tracing-subscriber` driven by
      `RUST_LOG` and a `-v/-vv` flag. At debug level log method, URL, status and
      timing; at trace level log request/response bodies with the auth token
      redacted. The same logging code serves both the CLI and the GUI backend,
      since both go through `core`. (`--raw` and the `api` passthrough already act
      as built-in diagnostics — compare raw vs normalized output.)
- [ ] GUI (Tauri) debugging: expose the webview Chromium DevTools
      (`open_devtools()` in debug builds / right-click Inspect) and wire
      `tauri-plugin-log` to bridge Rust logs into the webview console. Verify the
      exact API against the pinned Tauri version at implementation time.

## Backends / issue trackers

- [ ] Evaluate supporting other backends / issue trackers beyond an OpenProject
      server (e.g. YouTrack, Jira, GitLab issues, GitHub issues, Redmine). Consider a
      backend abstraction in `core` so the resource/normalization layer can target
      more than one API, with per-server backend selection in the config.
- [ ] YouTrack backend: issues via the YouTrack REST API, permanent-token auth,
      per-server profile with `backend = "youtrack"`. Map issues/comments/work items
      (time tracking) onto the shared resource + normalization layer; feed logged
      work into the cross-backend timelog. Public and self-hosted (custom base URL)
      instances, multiple YouTrack servers.
