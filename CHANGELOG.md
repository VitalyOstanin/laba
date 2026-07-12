# Changelog

All notable changes to this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

### Added
- Task list: sort direction toggle (defaults to descending; click the active sort key to reverse).
- "Add a server" hint: a "Send a PR" link to the contributing guide alongside "Request a backend".
- `CONTRIBUTING.md` with a guide for adding a backend.

### Changed
- Localize time units (`ч`/`мин` in Russian) and the notification-count noun (Russian plural forms) instead of hardcoded `h`/`m` and a single suffix.

### Security
- Warn at config load when a server disables TLS verification or uses a non-HTTPS `base_url`, since either can expose the API token.
### Fixed
- CLI: exit with the conventional `128 + signum` code on interruption (SIGTERM → 143, SIGHUP → 129) instead of always 130.

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
