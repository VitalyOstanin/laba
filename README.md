# taskstream

Desktop tray client and command-line interface for [OpenProject](https://www.openproject.org/),
built on [Tauri](https://tauri.app/) with a shared Rust core.

> Status: early work in progress. The design and implementation plan are being
> drafted; APIs and commands are not yet stable.

## Goals

- A single Rust workspace providing:
  - a **core** library — OpenProject API v3 client (authentication, HAL
    normalization, credential storage, configuration);
  - a **CLI** binary for scripting and automation (JSON output by default);
  - a **desktop tray application** (Tauri) for Windows, macOS and Linux.
- The desktop application talks to the OpenProject API directly through the
  core library — it does not shell out to the CLI.

## Planned capabilities

- Work packages, comments, attachments, relations, time entries and
  notifications.
- Multiple OpenProject servers with a selectable default; per-server
  credentials and proxy settings (SOCKS5 / HTTP).
- Tray summaries for assigned work packages and logged time.

## License

MIT. See [LICENSE](LICENSE).
