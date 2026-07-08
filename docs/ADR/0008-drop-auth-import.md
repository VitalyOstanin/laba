# 8. Drop the `auth import` command

Date: 2026-07-08

## Status

Accepted

## Context

ADR 0004 added an `auth import` subcommand that read the predecessor
`taskstream-cli` (Python) config and token once into a new server profile, to
avoid a forced re-login when migrating. That predecessor tool is now being
retired, so there is no longer a maintained config to import from. The one-off
migration for the remaining users is done manually via `server add` and
`auth login`.

## Decision

Remove the `auth import` command and its supporting code entirely: the
`Import` variant of `AuthCmd`, its dispatch, and the `commands::import` module.
No replacement is provided; migrating a server is performed with the existing
`server add` and `auth login` commands.

This partially supersedes ADR 0004, which introduced `auth import`. The body of
ADR 0004 is left unchanged (ADR bodies are immutable); this record captures the
reversal of that one element of its decision.

## Consequences

- The CLI no longer depends on the predecessor's config layout or location.
- Users migrating from the retired tool run `server add` + `auth login` once
  by hand instead of `auth import`.
- The rest of ADR 0004's decision (free redesign, familiar resource model,
  multi-server surface) remains in force.
