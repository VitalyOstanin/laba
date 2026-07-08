# 7. Per-server two-tier cache of stable entities

Date: 2026-07-08

## Status

Accepted

## Context

Several commands repeatedly resolve the same rarely-changing lookups against the
server: enumeration `name -> id` resolutions, `user id -> name` mappings, and form
`schema -> custom field names`. Re-fetching these on every invocation adds latency
and load for data that changes infrequently.

## Decision

A per-server, two-tier cache of stable entities:

- In-memory map (lazily loaded, lives for the process) layered on top of a JSON
  file that survives restarts.
- Scoped per server profile; the backing file is
  `XDG_CACHE_HOME/taskstream/<server>/cache.json` (overridable via
  `OPENPROJECT_CACHE`).
- Three categories: enumeration resolve (`name -> id`), users (`id -> name`) and
  schemas (`href -> custom field names`).
- Entries expire after a TTL of 7 days.
- Writes are best-effort: file IO errors during writes are logged to stderr and
  swallowed, and a corrupt file is treated as empty. Only the explicit clear
  operations surface IO errors.
- A `cache clear` command invalidates the cache, with `--server` (defaulting to
  the active server) or `--all`.

## Consequences

- Repeated resolutions of stable dictionaries avoid server round-trips within the
  TTL window.
- Stale data is bounded by the 7-day TTL; `cache clear` provides manual
  invalidation when a dictionary changes sooner.
- The cache is an optimization, not a source of truth: its failures never abort a
  command, only the explicit clear surfaces errors.
