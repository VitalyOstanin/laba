# TODO

Backlog of ideas to evaluate. Not commitments.

## Desktop integration

- [ ] Integrate with the OS notification system (native desktop notifications):
      surface new/unread items as system notifications (freedesktop/`org.freedesktop.Notifications`
      on Linux, native on macOS/Windows), with click-through to the item.

## Dates / timezone

- [ ] Timezone display follow-ups. The day-boundary decision is settled and
      implemented: a single configurable zone (`Settings::timezone`, IANA name,
      default machine-local) drives the timelog "today"/day boundary and the GUI
      log-time `spentOn` default, via `core::datetime::Zone`. Per-server zones were
      rejected in favor of one override. Remaining: (1) render API datetimes
      (`createdAt`/`updatedAt`, notification times) through `Zone::format_datetime`
      once the GUI/CLI actually display them — no timestamp is shown today, so the
      primitive exists but has no call site yet; (2) give the CLI a `--tz` /
      `TASKSTREAM_TZ` override so its `spentOn` default and future datetime display
      match the GUI (the CLI has no persisted settings, so it needs a flag/env).
- [ ] First-day-of-week: add the `auto` (locale-derived) option. The explicit
      Monday/Sunday choice is implemented (`Settings::week_start`, used by the
      timelog week boundary via `week_start_of`). Deriving the first day from the
      system locale needs CLDR/ICU week data and is deferred. When week-based
      "this week" ranges land in the dashboard, route them through `week_start_of`
      too.

## UX

- [ ] Work out UX consistency conventions and apply them across the app. Example:
      a single, predictable reaction to ESC across screens and input fields
      (e.g. ESC clears/blurs a focused input, then closes the current
      panel/dialog, then falls back to the window's default) rather than ad-hoc
      per-widget behavior. Cover other cross-cutting interactions (Enter to
      submit, focus order, unsaved-changes prompts) in the same convention.

## Dependencies

- [ ] Evaluate `reqwest` 0.12 -> 0.13. Deferred deliberately: 0.13 switches the
      default rustls crypto provider to `aws-lc-rs` (needs cmake + a C toolchain
      to build `aws-lc-sys`), which conflicts with building `core`/`cli` on the
      host without build toolchains. Staying on ring requires `rustls-no-provider`
      + a direct `rustls` dependency + installing a process `CryptoProvider` in
      every entrypoint (CLI + GUI), with a latent runtime-panic risk on the first
      HTTPS call that the http-only wiremock tests do not cover. reqwest 0.12 is
      actively maintained (0.12.28), so the upgrade is low value for the risk.
      Revisit if 0.12 stops receiving fixes or ring support is dropped.
- [ ] Evaluate `keyring` 3 -> 4 (breaking): review the API/feature diff before
      bumping; the current feature set (`async-secret-service`, `async-io`,
      `crypto-rust`, `apple-native`, `windows-native`) may have changed.
- [ ] TypeScript 5.6 -> 7: deferred. `@sveltejs/kit` (2.69.2) still declares a
      `typescript` peer of `^5.3.3 || ^6.0.0` — TS 7 (the native compiler line)
      is not yet in range, and there is no stable TS 6. Bump once kit validates
      TS 7. (vite 8 and vite-plugin-svelte 7 were taken; only TS was held back.)

## Deferred review follow-ups

- [ ] NFC-normalize names before matching in `core/src/resolve.rs` (canonical
      Unicode equivalence). Needs a `unicode-normalization` dependency — evaluate
      whether the edge case justifies it.
- [ ] Design-token consistency in `gui/src/app.css`: replace the remaining
      literal `rgba(...)` values with tokens and regularize the spacing / radius /
      font-size scale.
- [ ] Clean up leftover `.part` download temp files on SIGINT/SIGTERM: needs a
      process-wide registry of in-flight temp paths plus a cross-platform signal
      handler. Currently self-corrects (unique pid+counter names, cleaned on the
      normal error paths).
- [ ] Run the GUI e2e (wdio) suite in CI: requires webkit2gtk + tauri-driver +
      xvfb on the runner. The unit (vitest) + svelte-check job is wired; e2e stays
      local/container for now (`npm run test:e2e`).

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

- [ ] Timelog calendar: locale/override follow-ups. The RF public-holiday and
      transferred-workday calendar is implemented (`core/src/holidays.rs`,
      `holidays_ru.json` compiled via `build.rs`; `plan_minutes` uses
      `HolidayCalendar::is_workday`). Remaining open questions: selecting a
      calendar by locale or per server (RF vs other countries) instead of the
      single RF default; how to ship and update the calendar data for future
      years; and letting the user override specific days (mark a given date
      working or non-working). Also revisit the fully-filled-week drop and
      `monday_of` week boundary if a first-day-of-week setting lands.

- [ ] Dashboard data virtualization: lazy-load tasks/notifications on scroll
      (windowed/infinite rendering + paginated backend fetches instead of loading
      whole lists), and evict off-screen / stale per-server data from memory
      (the `byServer` store currently keeps every server's full lists resident).
      Bound resident data by active server + viewport; drop or re-fetch on demand.

- [ ] UI scaling setting: a user-adjustable interface zoom / scale factor
      (e.g. font-size / rem base or a CSS zoom on the root), persisted in GUI
      settings alongside theme and language, with sensible steps and a reset.
      Consider honoring the OS/display scale as the default.
