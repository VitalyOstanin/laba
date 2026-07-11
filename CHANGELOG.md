# Changelog

All notable changes to this project are documented here.
The format follows Keep a Changelog and Semantic Versioning.

## [Unreleased]

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
