# 1. Tauri desktop client with a shared Rust core

Date: 2026-07-08

## Status

Accepted

## Context

This project replaces two existing tools: a Python CLI for the OpenProject API
and a GNOME Shell extension that shows notifications and tasks in the panel. The
replacement must run on Windows, macOS and Linux, expose the same automation
surface as the CLI, and provide a desktop tray application. The desktop app must
talk to the OpenProject API directly rather than shelling out to the CLI.

Cross-platform desktop options considered were Flutter (Dart), Electron
(JavaScript) and Tauri (Rust + system WebView). All three need an
AppIndicator/StatusNotifierItem bridge on GNOME, so that constraint does not
distinguish them. The API logic must be reimplemented regardless, because the
GNOME extension's UI (GJS/St widgets) does not port to any of them.

## Decision

Build a single Cargo workspace with three crates:

- `core` — the OpenProject API client library (transport, auth, config,
  secrets, normalization). Both other crates depend on it.
- `cli` — the command-line binary.
- `gui` — a Tauri desktop tray application (later milestone).

Rust/Tauri is chosen because Rust is already used in the author's other projects
(so the toolchain and idioms are familiar), the runtime is lighter than
Electron's bundled Chromium, and tray support is part of the framework. The GUI
uses `core` in-process; it never spawns the CLI.

## Consequences

- One language and one API client serve both the CLI and the GUI; no duplicated
  transport or normalization logic.
- The GNOME extension's UI is rewritten, not ported.
- On GNOME, the tray icon requires the AppIndicator extension (a GNOME
  limitation, not specific to Tauri).
- Desktop tray package maturity across all three operating systems must be
  validated by a prototype before the GUI milestone commits to it.
