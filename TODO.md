# TODO

Backlog of ideas to evaluate. Not commitments.

## Desktop integration

- [ ] Integrate with the OS notification system (native desktop notifications):
      surface new/unread items as system notifications (freedesktop/`org.freedesktop.Notifications`
      on Linux, native on macOS/Windows), with click-through to the item.

## Backends / issue trackers

- [ ] Evaluate supporting other backends / issue trackers beyond an OpenProject
      server (e.g. Jira, GitLab issues, GitHub issues, Redmine). Consider a
      backend abstraction in `core` so the resource/normalization layer can target
      more than one API, with per-server backend selection in the config.
