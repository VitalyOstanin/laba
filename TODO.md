# TODO

Backlog of ideas to evaluate. Not commitments.

## Desktop integration

- [ ] Integrate with the OS notification system (native desktop notifications):
      surface new/unread items as system notifications (freedesktop/`org.freedesktop.Notifications`
      on Linux, native on macOS/Windows), with click-through to the item.

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
