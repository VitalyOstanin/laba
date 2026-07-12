# TODO

Backlog of ideas to evaluate. Not commitments.

## Desktop integration

- [x] Integrate with the OS notification system (native desktop notifications):
      surface new/unread items as system notifications with click-through to the
      item. Implemented: Linux uses freedesktop notifications directly
      (notify-rust/zbus) with a `default` action for click-through, because the
      Tauri notification plugin's Actions API is mobile-only; Windows/macOS fall
      back to the plugin (basic banner, no click-through). Gated by the
      `desktop_notifications` setting. Follow-ups: macOS notification permission
      (`requestPermission`) is not requested yet (best-effort fallback), and
      Windows/macOS have no click-through.

## Dates / timezone

- [ ] Timezone display follow-ups. The day-boundary decision is settled and
      implemented: a single configurable zone (`Settings::timezone`, IANA name,
      default machine-local) drives the timelog "today"/day boundary and the GUI
      log-time `spentOn` default, via `core::datetime::Zone`. Per-server zones were
      rejected in favor of one override. Remaining: (1) render API datetimes
      (`createdAt`/`updatedAt`, notification times) through `Zone::format_datetime`
      once the GUI/CLI actually display them — no timestamp is shown today, so the
      primitive exists but has no call site yet. Done: the CLI has a global
      `--tz` / `LABA_TZ` override (the CLI has no persisted settings, so it needs a
      flag/env); it drives the `time create` `spentOn` default via
      `Zone::resolve(...).today()`, matching the GUI. Future CLI datetime display
      should route through the same zone once item (1) lands.
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

- [x] NFC-normalize names before matching in `core/src/resolve.rs` (canonical
      Unicode equivalence). Done: a `fold(s)` helper NFC-normalizes then lowercases,
      applied to both operands at every name comparison (`resolve_by_name` exact
      match; principal exact match and token `contains`), so composed/decomposed
      spellings of the same name compare equal (e.g. Cyrillic "й" NFC vs
      "и"+U+0306). Added the pure-Rust `unicode-normalization` dependency (no C
      deps); test `status_name_matches_across_unicode_normalization` matches an NFD
      server name against an NFC, differently-cased query.
- [~] Design-token consistency in `gui/src/app.css`. Done: there were no literal
      `rgba(...)` left (colors already go through tokens + `color-mix`); the radius
      and font-size scales are now named token sets (`--radius`/`--radius-sm`/`-xs`/
      `-2xs`/`-pill`; `--fs-2xs`..`--fs-2xl`) and every px literal was mapped 1:1
      to a token (byte-identical rendering; `50%` dots and relative `em` markdown
      sizes left as-is). Remaining: regularize the spacing scale
      (padding/margin/gap) — deliberately deferred, as those values are far more
      scattered (8/10/4/6/1/12/5/3/18/2/14/16/7/20/34/48 px across hundreds of
      sites) and consolidating them risks visual drift; needs a spacing token set
      plus per-site review, not a mechanical 1:1 swap.
- [x] Clean up leftover `.part` download temp files on SIGINT/SIGTERM. Done:
      `core` keeps a process-wide registry of in-flight temp paths (`TEMP_DOWNLOADS`)
      and exposes `cleanup_temp_downloads()`; `download_to_path` registers via an
      RAII `TempFile` guard that also removes the file on any early return and is
      committed only after the rename. The CLI installs a `ctrlc` handler
      (SIGINT/SIGTERM/SIGHUP; runs on its own thread, so the fs/lock work is safe)
      that calls `cleanup_temp_downloads()` and exits 130. The library never
      installs a signal handler itself — the application owns signals; the GUI is
      left as-is (its own lifecycle, rarely signalled) but can call the same
      cleanup if needed.
- [x] Run the GUI e2e (wdio) suite in CI: the `e2e` job runs the same
      `ivangabriele/tauri` image (webkit2gtk + WebKitWebDriver + tauri-driver +
      xvfb bundled) with `seccomp=unconfined` and `xvfb-run`. Root-caused a local
      hang: under `scripts/tauri-container.sh` bash exec-optimized the sole
      command into `xvfb-run`, making it PID 1, where its Xvfb-readiness SIGUSR1
      handshake never releases the internal `wait`; fixed by running the
      container with `--init`. (GitHub's container jobs are unaffected — the step
      shell is not PID 1 there.)
- [x] Auth-login duplicate check: e2e test for rejecting a second profile that is
      the same user (same base URL + `users/me` login/id) as an existing one. Done:
      unblocked the secrets isolation. `OPENPROJECT_SECRETS` now overrides the token
      store path (mirroring `OPENPROJECT_CACHE`); when it is set the file is the
      *exclusive* store and the system keyring is skipped (`Secrets::resolve` +
      `file_only`), so a run never touches the real keyring. All entry points (CLI +
      GUI) build the store via `Secrets::resolve()`. The e2e
      (`auth_login_rejects_duplicate_user_unless_forced`) configures two profiles on
      one base URL, logs in the first, and asserts the second login is rejected and
      `--force` overrides it — isolated locally and in CI alike.

- [ ] Concurrent name/schema resolution on first render (deferred). The comment
      list resolves each unknown author sequentially (`comment::list` loops
      `user_name(uid).await` per element → one `GET users/{id}` each) and the task
      list expands `customFields` sequentially (`render_elements` loops
      `custom_field_names(schema_href).await` per unique schema). Both are cached
      (`user_name`/`custom_field_names` hit the in-memory + file cache), so only the
      first render of several distinct authors / task types pays a serial GET each;
      the count of distinct authors/schemas per view is small and the impact is
      unmeasured. A `join_all`/`try_join_all` prewarm would parallelize the cold
      path but shares one `&Client` cache with interior mutability, risking
      duplicate concurrent fetches for the same id and interleaved cache writes —
      more complexity and risk than the unmeasured, cache-mitigated benefit
      justifies. Revisit if a profiling pass shows first-render latency dominated by
      these serial lookups (e.g. many distinct authors in a busy thread).

## UI testing

- [x] End-to-end UI tests via the official Tauri WebDriver path: `tauri-driver`
      driving the built native app through WebdriverIO (or Selenium). This runs
      the real webview + Rust backend + native window, not a browser-only mock.
      Platform drivers: WebKitWebDriver on Linux, the Edge WebView2 driver on
      Windows (macOS has no official WebDriver support yet); run headless in CI
      under xvfb. Keep the fast layers alongside it: Rust/CLI logic tests (already
      present) and frontend component/unit tests. Verify the exact `tauri-driver`
      setup against the pinned Tauri version at implementation time.
      Done: implemented as the same work as the "Run the GUI e2e (wdio) suite in
      CI" item above — `gui/wdio.conf.js` launches the built app via
      `tauri-driver` + WebKitWebDriver under `xvfb`; `gui/e2e/smoke.test.js`
      asserts the real dashboard renders both columns; the CI `e2e` job runs it
      on Linux. Fast layers (Rust/CLI logic tests, frontend unit/component tests)
      remain alongside. Windows WebView2 / macOS drivers not added (Linux-only in
      CI for now); add if cross-platform e2e is wanted later.

## Branding

- [x] Replace the stock Tauri placeholder icon set in `gui/src-tauri/icons/`
      with a real laba icon. Done: `icons/icon.svg` (checklist on an
      accent-blue tile) is the source; all sizes regenerated via `tauri icon`.
      The tray uses `default_window_icon()` in `gui/src-tauri/src/lib.rs`; the
      Linux window `app_id` is `laba-gui` (StartupWMClass must match for
      dev `--no-bundle` runs).

## Updates / self-update

- [~] Periodic update check against the GitHub releases of this project. Poll the
      releases API on an interval, compare the latest tag to the running version,
      and surface an available update **unobtrusively** (a quiet banner/indicator,
      not a modal), with a control to **dismiss/hide** it (and not nag again for
      that version). When an update is available, offer to perform it **fully
      automatically, but only on an explicit user action** (a button) — download
      the new release asset, replace the binary, and restart. Never update
      silently in the background. Respect the no-proxy / network constraints and
      verify the download (checksum/signature) before applying.
      Phase A DONE (code/config/UI): Tauri updater + process plugins, updater
      config (pubkey + GitHub latest.json endpoint), cumulative changelog via the
      anonymous GitHub releases API (`core::update`), `UpdateBanner` with a
      changelog panel and four actions (update & restart / what's new / remind me
      later / skip this version). Phase B TODO (needs release infra): put the
      minisign private key in CI secrets and extend `release.yml` to build, sign,
      and upload the GUI **AppImage** bundle + `latest.json` (the updater plugin
      only self-updates AppImage on Linux; current release.yml ships the CLI only).
- [ ] Fully automatic self-update (separate, opt-in): once the user-triggered
      update flow above is solid, add an opt-in setting to apply available updates
      automatically without a per-update click. Off by default; the manual
      user-action flow remains the default path.
- [~] Config/settings compatibility across updates. An update must never break
      the user's existing `config.json` / settings: newer fields load with
      defaults (already the `#[serde(default)]` convention) and older binaries
      tolerate unknown fields. When a change is not backward-compatible, ship a
      **versioned migration** — stamp the config with a schema version, migrate
      forward on load, and back up the pre-migration file. The update flow should
      verify the migrated config loads before committing to the new binary, and
      be able to roll back (restore the backup + prior binary) if it does not.
      Done (config side): `core/src/migrate.rs` is a forward-only, per-file
      schema-version framework wired into both `Config::load` and
      `Settings::load`. Each file carries `schema_version` (`#[serde(default)]`);
      on load the raw JSON is parsed to a `Value`, migrated step by step
      (`vN -> vN+1`) to the current version, then deserialized — which verifies
      the migrated shape loads — and only when a step actually ran the original
      is backed up as `<name>.bak-v<from>` before the migrated file is rewritten.
      A file newer than the binary is left untouched (never downgraded). Absent
      versions read as `BASE_VERSION`. `config.rs` already ships one real step
      (`m1_normalize_base_urls`); `settings.rs` has an empty step list (no
      breaking change yet). Tests cover version clamping, running only the needed
      steps, no-op at/above current, pre-versioning migration + backup, and a
      newer-file being preserved. Remaining (blocked on updates Phase B / release
      infra): couple this into the self-update flow — verify the migrated config
      loads on the *new* binary before committing to it and roll back (restore
      the backup + prior binary) if it does not. That belongs to the updater,
      which needs the signing/AppImage release pipeline first.

## Networking / proxy

- [x] Proxy support for backend HTTP: both **SOCKS5** and **HTTP(S)** proxies,
      configurable at two levels — a **global** default and a **per-server**
      override (a server may need a different proxy or none). Per-server wins over
      global; allow an explicit "no proxy / direct" per server. Honor standard
      `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY`/`NO_PROXY` env as the fallback when
      nothing is configured. reqwest supports both schemes (`reqwest::Proxy`);
      SOCKS5 needs the `socks` feature. Apply the resolved proxy when building the
      per-server client in `core`. Surface it in server settings (GUI) and as a
      CLI flag/config. Cover proxied vs direct in tests (wiremock + a stub proxy).
      Done: `core::client::{ProxyChoice, resolve_proxy, Client::new_with_global}`
      resolves override > per-server `ServerProfile.proxy` > global `Config.proxy`
      > ambient env > direct; `"direct"`/`"none"`/empty forces a direct connection
      at any level. GUI exposes a per-server proxy field and a global default;
      commands `set_server_proxy`/`get_global_proxy`/`set_global_proxy`. CLI honors
      `Config.proxy` on every command plus the existing `--proxy` override and
      `server add --proxy`. Unit tests cover the resolution ladder. CLI setters
      done: `server proxy` (per-server override: set / clear / show, resolves the
      default server) and `server global-proxy` (global default: set / clear /
      show), both mirroring the GUI's `normalize_proxy` (trim, empty clears,
      `direct` kept literal). Follow-up still open: an integration test that
      actually proxies through a stub SOCKS/HTTP proxy (the unit/CLI tests cover
      resolution and config wiring, not on-the-wire proxying).

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

- [x] Request logging in `core` via the `log` facade (not `tracing`: the GUI
      already routes the `log` facade through `tauri-plugin-log`, so a facade
      avoids a second logging system there — a `tracing` bridge would be extra
      wiring for no gain here). `core` logs method/URL/status/timing at debug and
      request/response bodies at trace in `Client::request_json_query` and
      `delete`; the streaming attachment paths (`stream_download`,
      `download_to_path`, the upload POST) log method/URL/status/timing and byte
      counts at debug — metadata only, never bodies (binary; upload logs the JSON
      response at trace). The auth token lives in the Authorization header and is
      never logged. The CLI installs `env_logger` (stderr, so stdout stays clean
      JSON) with a global `-v`/`-vv` (warn → debug → trace) that `RUST_LOG`
      overrides; the GUI maps a bare `RUST_LOG` level word onto the
      `tauri-plugin-log` level (default info). Optional future work only:
      structured fields / spans if richer tracing is ever needed.
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

- [x] UI scale: honor the OS/display scale as the default. The manual scale is
      implemented (`Settings::ui_scale`, percent, clamped 50-200; applied by the
      GUI to the root font size via `applyUiScale`, with −/+/reset on the settings
      screen). Not-needed as originally framed: the webview already applies the OS
      scale via `devicePixelRatio`, so `ui_scale` is a *logical* multiplier on top
      of correct physical sizing. A fixed 100% logical default is therefore correct
      whenever the webview honors the OS scale (all integer scales; verified the
      dev machine runs an integer 2.0 Wayland scale). Deriving the default from the
      OS scale would double-apply. The only residual case is fractional Wayland
      scales (1.25/1.5) that WebKitGTK historically under-honors — if that ever
      bites, add a conservative `auto` mode that compensates only the *unhonored*
      part (`scale_factor / devicePixelRatio`), never the full OS scale.
