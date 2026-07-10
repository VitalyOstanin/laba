# taskstream — contributor & agent guide

Rules and conventions for working in this repository. Read this before making
changes. It complements `README.md` (what the project is) and `docs/ADR/`
(why decisions were made); this file is about *how to work here*.

## Contents

- [Project shape](#project-shape)
- [Build & verify](#build--verify)
- [Architecture rules](#architecture-rules)
- [Backend capabilities](#backend-capabilities)
- [Visual language](#visual-language)
  - [Theme tokens](#theme-tokens)
  - [Color semantics](#color-semantics)
  - [Read / unread](#read--unread)
  - [Status tinting](#status-tinting)
  - [Typography & spacing](#typography--spacing)
  - [Layout](#layout)
  - [Icons](#icons)
  - [Interaction & keyboard](#interaction--keyboard)
  - [Accessibility](#accessibility)
  - [Anti-patterns](#anti-patterns)
- [Content rules](#content-rules)
- [Commits](#commits)

## Project shape

A single Cargo workspace plus a Tauri desktop app:

| Path            | Crate / package     | Responsibility                                   |
|-----------------|---------------------|--------------------------------------------------|
| `core/`         | `taskstream-core`   | Pure library: API client, normalization, config, timelog, capabilities. No UI. |
| `cli/`          | `taskstream-cli`    | Thin CLI over core (`taskstream` binary, JSON by default). |
| `gui/src-tauri/`| `taskstream-gui`    | Thin Tauri commands over core; window/tray/lifecycle. |
| `gui/src/`      | SvelteKit frontend  | Svelte 5 + TypeScript UI; talks to Rust via `invoke`. |

Business logic lives in `core`. The CLI and GUI are adapters — keep them thin.

## Build & verify

**GUI builds only in a container.** `ksni` pulls `libdbus-sys`, which needs the
`libdbus-1-dev` build package; do not install build toolchains on the host. Use:

```
./scripts/tauri-container.sh 'cd gui && npm run tauri -- build --no-bundle'
```

- **Use `tauri build --no-bundle`, never `cargo build --release` for the GUI.**
  A bare `cargo build` produces a binary that loads the frontend from the dev
  server and shows "Could not connect to localhost". `tauri build` runs
  `beforeBuildCommand` and embeds the frontend via custom-protocol.
- The container-built binary runs on the host (runtime `libdbus-1-3` is present).

**Verify (host):**

- Rust: `cargo nextest run -p taskstream-core -p taskstream-cli`,
  `cargo clippy`, `cargo fmt --all --check`. Prefer `nextest` over `cargo test`.
- Frontend (`gui/`): `npm run check` (svelte-check), `npm run lint`,
  `npm run test` (vitest), `npm run format:check` (prettier).
- `taskstream-gui` clippy needs the container (same libdbus reason).

Never raise parallelism / resource limits (test workers, etc.) to "speed things
up".

## Architecture rules

- **Core is pure and testable.** Timelog, normalization, config, and capability
  logic live in `core` with unit tests; the GUI/CLI only wire them up.
- **Adapters stay thin.** A Tauri command or CLI subcommand should validate
  input, call core, and map the result — no business logic.
- **Backward compatibility of config.** `config.json` is user data. New fields
  get `#[serde(default)]` and `skip_serializing_if` for empty/None, so old
  configs keep loading and unset fields don't clutter the file.
- **Secrets never logged.** Tokens live in the OS keyring (file fallback); never
  print them.

## Backend capabilities

Backend-specific behavior is expressed as capability methods on `Backend`
(`core/src/config.rs`), not as `if backend == "openproject"` scattered through
the UI. Examples: `supports_timelog`, `supports_time_activities`,
`needs_local_history`, `supports_notifications`,
`supports_notification_read_toggle`, `default_poll_secs`.

The GUI surfaces the relevant capabilities on `ServerInfo` and drives the UI
from them (hide a column, enable a control) instead of hardcoding a backend
name. When adding backend-varying behavior, add a capability method + test.

## Visual language

The UI is a calm, information-dense desktop tool: a dashboard that is scanned
and operated, not read top to bottom. Favor clarity and state-at-a-glance over
decoration. All styling goes through the theme tokens in `gui/src/app.css` —
components never hardcode colors.

### Theme tokens

Light and dark are both first-class. The app follows the OS preference via
`@media (prefers-color-scheme)`, and a settings choice forces a theme via
`data-theme="light" | "dark"` on `:root`, which overrides the media query.
Define palette values as CSS custom properties in all three blocks
(`:root`/`[data-theme=light]`, the dark media query, `[data-theme=dark]`) and
style through the tokens only. Structural tokens: `--radius`, `--gap`.

### Color semantics

| Token         | Meaning                                    |
|---------------|--------------------------------------------|
| `--bg` / `--surface` / `--surface-2` | page / card / raised surfaces |
| `--text` / `--text-dim`              | primary / secondary text      |
| `--border`                           | hairlines, control borders    |
| `--accent` / `--accent-text`         | primary action, active state, links |
| `--danger`                           | error, blocking state         |
| `--warn`                             | attention, shortfall          |
| `--ok`                               | success, met target           |
| `--info`                             | in-progress / informational (teal) |

Semantic colors (danger/warn/ok/info) are separate from the accent and from
each other — do not reuse the accent to mean "success".

### Read / unread

Read state is shown by a **dot**, never a checkbox or checkmark (a checkmark
reads as "selected / done", the wrong metaphor for read state):

- **Unread** — a filled `--accent` dot; the row's subject is bold.
- **Read** — a hollow ring (border, transparent fill); normal weight.

The dot is also the toggle: clicking it flips read/unread (with a tooltip),
gated by `supports_notification_read_toggle`. When toggling is unsupported the
dot is a static indicator. Do **not** convey read state by dimming the row.

### Status tinting

Task rows can be tinted by workflow status via a per-server `status_colors` map
(`status -> token`). Tokens are semantic, not raw colors, so they render in both
themes: `danger` → `--danger`, `warn` → `--warn`, `success` → `--ok`,
`progress` → `--info`, `dimmed` → `--text-dim`. An unmapped status is neutral.
Status strings are instance-specific user data — never hardcode them in code,
tests, or mockups.

### Typography & spacing

Body face is Inter with a system fallback; base 14px, line-height ~1.45.
Column/section headers are small, uppercase, letter-spaced, `--text-dim`. Use
`font-variant-numeric: tabular-nums` wherever digits align (ids, hours,
counts). Space sibling groups with flex/grid `gap`, not per-element margins.

### Layout

- A top bar (server switcher + settings), the timelog indicator, then a
  two-column `main` (notifications, tasks) in cards.
- Cards: `--surface`, `--border`, `--radius`; a header with an uppercase title.
- Long lists use windowed reveal + a zero-height sentinel for paging.
- Wide content scrolls inside its own container; the page never scrolls
  sideways.

### Icons

Inline SVG with `stroke="currentColor"` / `fill` from tokens, sized in px, and
`aria-hidden` when decorative. No icon fonts, no external assets (the webview
CSP blocks remote hosts — see `docs/ADR/0010`).

### Interaction & keyboard

- What is interactive looks interactive (cursor, hover, focus).
- Keyboard shortcuts match the **physical key** via `event.code` (e.g. `KeyQ`),
  so they work under any layout (a Cyrillic layout included). Current global
  shortcuts: Ctrl+Q quits, Ctrl+W closes the window, Ctrl +/-/0 scale the UI.
- Task numbers link to the tracker (open in the system browser).
- Server badges show the full backend name ("OpenProject" / "GitHub"), not
  abbreviations.

### Accessibility

Every interactive control has a visible `:focus-visible` ring and an
`aria-label`/`title`. State encoded in color is also encoded in shape or text
(the read dot is filled vs hollow, not only a color change).

### Anti-patterns

- No checkbox/checkmark for read state.
- No dimming a row to mean "read/secondary" — carry state with the dot + weight.
- No hardcoded colors — go through tokens.
- No `backend === "..."` branching in the UI — use a capability.
- No emoji as UI affordances.

## Content rules

- **Committed artifacts are English** (code, comments, docs, commit messages,
  UI strings live in `gui/src/lib/locales/`).
- **No real domain data anywhere in the repo.** Mockups, code, tests, and
  fixtures use fictional example servers, statuses, and work items only — never
  a real organization's names, statuses, URLs, logins, or hosts.
- User-facing copy names things by what the user recognizes; controls say what
  they do; errors say what went wrong and how to fix it.

## Commits

- Conventional-commit style (`feat:`, `fix:`, `feat(gui):`, …).
- Small, focused commits; split unrelated changes.
- Do not add `Co-Authored-By` trailers.
