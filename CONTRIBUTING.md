# Contributing to laba

Thanks for your interest in improving laba. Bug reports, feature ideas, and pull
requests are all welcome.

## Contents

- [Ways to contribute](#ways-to-contribute)
- [Development setup](#development-setup)
- [Checks before a pull request](#checks-before-a-pull-request)
- [Commit and PR conventions](#commit-and-pr-conventions)
- [Adding a backend](#adding-a-backend)

## Ways to contribute

- **Report a bug or request a backend** — open an
  [issue](https://github.com/VitalyOstanin/laba/issues/new). For a new tracker
  (Jira, YouTrack, GitLab, Redmine, …) say which instance type (cloud or
  self-hosted) and how it authenticates.
- **Send a pull request** — small fixes and docs improvements can go straight to
  a PR. For a larger change (a new backend, a new screen), open an issue first
  so the approach can be agreed before you invest the work.

## Development setup

The project is a single Cargo workspace plus a SvelteKit frontend:

| Path             | Crate / package | Contents                            |
|------------------|-----------------|-------------------------------------|
| `core/`          | `laba-core`     | API clients, config, cache, timelog |
| `cli/`           | `laba-cli`      | `laba` command-line binary          |
| `gui/`           | —               | SvelteKit frontend                  |
| `gui/src-tauri/` | `laba-gui`      | Tauri desktop shell (Rust)          |

`cargo build`/`cargo test` on the host cover `core` and `cli`. The GUI (frontend
and the `laba-gui` crate) is built and tested only inside the Tauri container,
because it needs the webkit2gtk system libraries. See
[README.md](README.md#building) for the exact commands.

## Checks before a pull request

Run the same checks CI runs, and make sure they pass:

- Host: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo nextest run` (or `cargo test`).
- GUI, in the container: `npm run lint`, `npx prettier --check .`,
  `npm run check` (svelte-check), `npm run test` (vitest).

Frontend formatting is enforced separately from linting: run
`npx prettier --check .` before pushing, as ESLint and vitest do not catch a
formatting-only difference.

## Commit and PR conventions

- Conventional-commit style subjects (`feat:`, `fix:`, `docs:`, `chore:`,
  scoped as `feat(gui): …` where useful).
- Keep unrelated changes in separate commits.
- Update `CHANGELOG.md` under `[Unreleased]` for user-visible changes.

## Adding a backend

laba talks to trackers through a small facade. A new backend (say GitLab) means
teaching that facade one more variant. The touch points:

1. **Declare the variant.** Add it to the `Backend` enum in
   [`core/src/config.rs`](core/src/config.rs) and set each capability method
   (`supports_timelog`, `supports_notifications`, `supports_task_detail`,
   `default_poll_secs`, …). The capabilities drive what the UI shows, so an
   unsupported feature is hidden rather than shown empty.
2. **Implement the client.** Add a module beside
   [`core/src/github.rs`](core/src/github.rs) (the GitHub backend is the
   smallest reference) that fetches tasks and notifications and normalizes them
   to the shared `Vec<Value>` shape.
3. **Route it in the facade.** Extend the `match profile.backend` arms in
   [`core/src/backend.rs`](core/src/backend.rs) (`list_tasks_page`,
   notifications, and the other entry points) to call your client.
4. **Expose it in the UI.** Add the backend to the `backend` union in
   [`gui/src/lib/types.ts`](gui/src/lib/types.ts) and to the "add a server" form
   labels/hints in the locale dictionaries
   ([`gui/src/lib/locales/en.ts`](gui/src/lib/locales/en.ts),
   [`ru.ts`](gui/src/lib/locales/ru.ts)); add its display name to
   `READY_BACKENDS` in
   [`gui/src/lib/components/BackendsBanner.svelte`](gui/src/lib/components/BackendsBanner.svelte).
5. **Test it.** Cover normalization with unit tests next to the client, and add
   the backend to any capability-driven UI tests.

If the tracker needs an auth flow the app does not have yet, open an issue to
discuss it before implementing.
