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

- [x] UX consistency conventions for keyboard/focus, applied across the app
      (`gui/src/lib/keys.ts`): ESC in a focused text field discards the
      in-progress edit and blurs, swallowing the event so a surrounding panel
      does not also close (`fieldKeys` action; deferred-commit fields revert to
      the store value, the search box clears); ESC with no field focused closes
      the topmost transient surface (`onGlobalEscape`, currently the Timelog
      expand panel); Enter in a single-line field commits and blurs (not while
      IME-composing, not in a textarea); a consistent `:focus-visible` ring on
      every control. Non-goal, documented deliberately: unsaved-changes prompts
      — settings autosave on every change, so there is no pending unsaved state;
      any future form that defers saving must add the prompt itself. Follow-up:
      apply the same convention to new input surfaces and any future modal
      dialogs as they land.

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
- [x] Run the GUI e2e (wdio) suite in CI: the `e2e` job runs the same
      `ivangabriele/tauri` image (webkit2gtk + WebKitWebDriver + tauri-driver +
      xvfb bundled) with `seccomp=unconfined` and `xvfb-run`. Root-caused a local
      hang: under `scripts/tauri-container.sh` bash exec-optimized the sole
      command into `xvfb-run`, making it PID 1, where its Xvfb-readiness SIGUSR1
      handshake never releases the internal `wait`; fixed by running the
      container with `--init`. (GitHub's container jobs are unaffected — the step
      shell is not PID 1 there.)
- [ ] Auth-login duplicate check: add an e2e test for rejecting a second profile
      that is the same user (same base URL + `users/me` login/id) as an existing
      one. Blocked on secrets isolation: `Secrets::default_fallback_path()`
      derives from `default_config_path()` and ignores the `--config` flag, so a
      test cannot point token storage at a temp dir. Either make the secrets
      fallback honor the config dir / an env override, then test the rejection,
      or cover it another way. The pure identity extraction is unit-tested.

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
- [x] GUI (Tauri) debugging: webview DevTools open automatically in debug builds
      (`open_devtools()` in `setup`, right-click Inspect also available), and
      `tauri-plugin-log` bridges Rust `log` records to stdout and the webview
      console (`attachConsole()` on the JS side, `log:default` capability). GUI
      diagnostics use `log::warn!` instead of `eprintln!`. Follow-up: broader
      `tracing` in `core` is tracked separately (observability item above).

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

- [x] Dashboard data virtualization: implemented across all layers.
      Core exposes a paged API — `backend::Page { items, next_offset }` with
      `list_tasks_page`/`list_notifications_page` (`PAGE_SIZE` 50): OpenProject
      paginates by 1-based page (`work_packages::list_page` /
      `notification::list_page` return the reported `total`, and `next_offset`
      is the next page or `None` at the end); the GitHub backend returns the
      whole client-merged issue/PR stream in one page (`next_offset: None`),
      because `gh` has no clean cursor across it. The tauri `list_tasks` /
      `list_notifications` commands take `page`/`pageSize` and return `Page`.
      The GUI keeps full arrays + page cursors resident only for the active
      server (`byServer`); every other enabled server is evicted to a cheap
      summary (`summaries`: error flag + unread count) that still feeds the
      server-switcher dot and the aggregate unread count. Switching servers
      loads the new one and evicts the old one's arrays. The columns window the
      resident list (reveal one page at a time via an `onVisible` sentinel +
      "Load more" fallback), then fetch the next backend page when the resident
      page is exhausted (`loadMoreTasks` / `loadMoreNotifications`). Note: a
      non-active server's unread summary reflects only its first page (up to
      `PAGE_SIZE` unread); a full aggregate would need a count endpoint.

- [ ] UI scale: honor the OS/display scale as the default. The manual scale is
      implemented (`Settings::ui_scale`, percent, clamped 50-200; applied by the
      GUI to the root font size via `applyUiScale`, with −/+/reset on the settings
      screen). Remaining: derive a sensible default from the OS/display scale
      instead of a fixed 100 when the user has not chosen one.
