# Changelog

All notable changes to this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

### Added
- GitHub: surface everything that needs attention — issues/PRs I'm involved in (author, assignee, mention, comment), pull requests whose review is requested from me, and everything open in my own repositories — de-duplicated, instead of only items assigned to me.
- First-run setup wizard: a step-by-step modal (backend → connection → token → verify) that opens automatically when no server is configured. The GitHub step checks the `gh` CLI up front — offering an install link when it's missing, a sign-in prompt when it's unauthenticated, and a scope hint (`gh auth refresh -s repo,read:org,notifications`) for read-only logins.

## [0.1.3] - 2026-07-13

### Added
- Settings: sign in to an OpenProject server from the GUI — enter the API token in the add-server form or per server in the list, so a fresh install no longer needs the CLI to store a token. The token is validated against `users/me` and a duplicate account is rejected, mirroring `auth login`.
- Dashboard: an explicit empty state (with an "add a server" action) when no server is configured, instead of blank columns.
- Errors: show a friendly message instead of a raw backend string — strip the technical `kind:` prefix and, when a server has no token, show a "not signed in" notice with a link to Settings.
- Test coverage in CI: the GUI unit suite enforces a coverage threshold on the logic layer (`@vitest/coverage-v8`), and a `cargo llvm-cov` job reports Rust core/cli line coverage with a conservative floor.
- "Add a server" hint: a "Send a PR" link to the contributing guide alongside "Request a backend".
- `CONTRIBUTING.md` with a guide for adding a backend.

### Changed
- Settings: collapse the advanced per-server options (proxy, status colors, filters, display fields) into an "Advanced" section so the common fields stay uncluttered.
- Settings: confirm per-server profile edits with the same "Saved" indicator the global settings already showed.
- Task list: sort direction toggle (defaults to descending; click the active sort key to reverse).
- Localize time units (`ч`/`мин` in Russian) and the notification-count noun (Russian plural forms) instead of hardcoded `h`/`m` and a single suffix.
- Route cache, state, and retry warnings in the core library through the `log` facade (honoring `RUST_LOG`) instead of always printing to stderr.
- Document the `OPENPROJECT_SECRETS` environment variable and add a table of contents to the README.
- Format displayed dates (comment timestamps, timelog days) with the active locale via `Intl.DateTimeFormat` instead of a raw ISO slice.

### Security
- Warn at config load when a server disables TLS verification or uses a non-HTTPS `base_url`, since either can expose the API token.
- Create the fallback secrets file with 0600 permissions up front (and tighten a pre-existing loose file) instead of writing then chmod-ing.
- Pin third-party GitHub Actions (`dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `taiki-e/install-action`, `rustsec/audit-check`, `tauri-apps/tauri-action`) to full commit SHAs with a version comment, so a rewritten upstream tag cannot alter the CI/release pipeline.
- Enforce a dependency license allow-list (`deny.toml`, `cargo deny check licenses` in the Audit workflow), so an update that pulls in a copyleft crate fails CI instead of shipping in a binary release.
- Drop `'unsafe-inline'` from the webview CSP `script-src` (Tauri hashes the bundled inline scripts, so no inline JS is silently trusted); verified the app still renders via the e2e smoke.

### Fixed
- Release: build the Linux CLI on the oldest supported Ubuntu (glibc parity with the GUI bundle) so the downloaded binary runs on older distributions instead of failing with a `GLIBC_2.3x not found` error.
- CLI: exit with the conventional `128 + signum` code on interruption (SIGTERM → 143, SIGHUP → 129) instead of always 130.
- CLI `--human` output: measure the key-column width in characters, not bytes, so a non-ASCII key no longer over-pads and skews the value column.

## [0.1.2] - 2026-07-11

### Fixed
- Linux AppImage: drop the bundled libwayland-client so the webview renders on modern distributions instead of showing a blank window.
- Tray: show the app icon with a small red count badge in the corner instead of replacing it with a bare number.

## [0.1.1] - 2026-07-11

### Changed
- Release bump to exercise the end-to-end self-update path from 0.1.0.

## [0.1.0] - 2026-07-11

### Added
- Cargo workspace: `core` library and `laba` CLI.
- Server profiles (JSON config) with default selection; SOCKS5/HTTP proxy per server.
- Token storage in the system keyring with a file fallback.
- `auth` (login/status/token/logout/import) and `server` (list/add/remove/set-default/show) commands.
- OpenProject and GitHub backends for tasks and time entries.
- Desktop GUI (Tauri v2 + Svelte 5): task list with status filters, task detail, time logging, tray indicator, notifications, and settings.
- Markdown rendering for task descriptions and comments.
- Signed self-update via the Tauri updater; multi-OS release bundles for Linux, Windows and macOS with a deterministic `latest.json` aggregator (macOS is notification-only).
- Settings migrations keyed by schema version.
