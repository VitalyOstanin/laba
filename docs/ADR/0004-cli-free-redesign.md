# 4. CLI is a free redesign, not a drop-in of openproject-cli

Date: 2026-07-08

## Status

Accepted

## Context

The predecessor `openproject-cli` (Python/Click) has existing consumers: shell
scripts and an agent whose notes pin exact command syntax. The new CLI could aim
for a drop-in-compatible interface or redesign freely. Multi-server support
(ADR 0002) and proxy support already force new surface that the predecessor
lacks, so strict drop-in compatibility is not achievable without contortions.

## Decision

Redesign the CLI freely, but justify every deviation from the predecessor's
interface. Keep the resource model and semantics familiar (`wp`, `comment`,
`attachment`, `relation`, `time`, `notification`, `api`, `auth`; JSON output by
default with `--human`/`--raw`), and add what multi-server requires: a `server`
subcommand and an `auth import` that reads the predecessor's config and token
once into a new profile.

## Consequences

- Existing scripts may need small edits; this is accepted in exchange for a
  coherent multi-server interface.
- `auth import` avoids a forced re-login when migrating from the predecessor.
- Deviations are tracked explicitly (in the design spec) so the interface change
  is auditable rather than incidental.
