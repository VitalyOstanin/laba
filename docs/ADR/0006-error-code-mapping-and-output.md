# 6. Error code mapping and output formats

Date: 2026-07-08

## Status

Accepted

## Context

The Python openproject-cli exposed semantic error categories (`NotFound`, `Api`,
`Auth`) and printed results as JSON. The Rust rewrite has a smaller, sysexits-style
`Error` enum, and the CLI needs a single, predictable output policy across all
commands so that both humans and scripts can consume it.

## Decision

- The former semantic categories are folded into the existing `Error` variants:
  `Usage` maps to exit code 2, `Io` to 74, `Internal` to 70, and everything else
  (including API and auth failures, the former `Api`/`Auth`/`NotFound`) is treated
  as `Api` and exits 70. No new error variants are introduced for this mapping.
- Output has three modes, selected by global flags:
  - default: pretty-printed JSON with key order preserved (`serde_json` built with
    the `preserve_order` feature);
  - `--human`: aligned/tabular plain text (objects aligned by key, arrays of
    objects rendered as tab-separated tables);
  - `--raw`: the raw API response without normalization.
- A `Null` value produces no output at all (matching the Python `emit_result`
  behaviour of skipping `None`).

## Consequences

- Exit codes remain sysexits-style and stable; scripts can branch on 2 (usage),
  74 (io) and 70 (everything else).
- The three-way output policy is uniform across commands, so formatting logic
  lives in one module (`cli/src/output.rs`).
- `--raw` bypasses normalization, so its shape follows the OpenProject API and is
  not guaranteed to match the normalized JSON.
