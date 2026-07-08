# 9. Token input via stdin or --token only, no interactive prompt

Date: 2026-07-09

## Status

Accepted

## Context

`auth login` needs to obtain a server API token and store it in the secrets
backend (ADR 0003). An interactive terminal prompt was considered, and a
prompt-with-hidden-echo dependency was briefly added. Two problems surfaced in
review:

- A dependency added only to hide terminal echo is a large surface for a small
  feature, and interactive prompts do not compose with the CLI's intended use
  (scripts, agents, CI), which is non-interactive.
- A token typed at a prompt or passed as a `--token` argument on the command
  line is exposed in shell history and, for `--token`, in the process list.

## Decision

`auth login` accepts the token from exactly two sources, in this order:

- the global `--token <value>` flag, when present;
- otherwise stdin, when `--with-token` is passed (the token is read to EOF and
  trimmed).

With neither, `auth login` fails with a `Usage` error ("provide the token via
stdin (--with-token) or --token"). An empty token is rejected. There is no
interactive prompt and no prompt dependency. Confirmation lines ("token stored",
"logged out") are written to stderr so stdout stays clean for machine consumers.

## Consequences

- The recommended path is piping the token via stdin
  (`printf %s "$TOKEN" | taskstream auth login --with-token`), which keeps it out
  of the process list; `--token` remains available but is documented as less
  private.
- No extra dependency is carried for terminal echo handling.
- The command is fully non-interactive, so it composes with scripts, agents and
  CI without a TTY.
- This refines the `auth` surface of ADR 0004 (the interactive prompt element is
  not adopted); the rest of ADR 0004 is unaffected.
