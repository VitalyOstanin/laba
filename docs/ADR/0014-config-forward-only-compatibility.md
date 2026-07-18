# 14. Forward-only backward compatibility for persisted settings

Date: 2026-07-18

## Status

Accepted

## Context

laba persists user settings the user cannot recover automatically: server profiles,
the default server, and the global proxy in `config.json`; GUI preferences (theme,
language, timezone, dashboard layout, …) in `gui-settings.json`; and access tokens in
the system keyring (with a file fallback). Losing any of these on an upgrade is a
serious regression — the user must re-add servers, re-enter tokens, and re-tune the UI.

A forward-only migration mechanism already exists (`core/src/migrate.rs`): each JSON
file carries a `schema_version`; on load the raw JSON is migrated step by step
(`vN -> vN+1`) to the current version, the original is backed up to `<name>.bak-v<from>`
before rewrite, a file newer than the binary is never downgraded, and the step count is
asserted to match the version. serde defaults make additive fields load old files
unchanged, and `deny_unknown_fields` is used nowhere, so unknown keys are ignored.

What is missing is not code but a **discipline**: a written rule for when a schema
change is additive versus breaking, how breaking changes ship, and how compatibility is
tested. ADR 0013 (the multi-backend refactor) explicitly waived config compatibility
while the schema is in flux; without a stated policy, that waiver could be read as
permanent.

## Decision

Adopt forward-only settings compatibility as a standing discipline, effective from the
first stable schema version after the ADR 0013 refactor settles. From that line onward,
a `config.json` / `gui-settings.json` written by any prior release must load under the
current binary without data loss.

1. **Additive by default.** A new field is optional with a default and
   `skip_serializing_if`; it needs no version bump and no migration. Enum variants may
   be added where readers tolerate unknown variants.
2. **Breaking changes ship a migration.** Renaming, removing, retyping, re-nesting, or
   re-encoding a field — or changing a materialized default or a map-key convention — is
   breaking: it bumps `*_SCHEMA_VERSION` and adds exactly one `vN -> vN+1` step to the
   file's migration list. Migration steps operate on `serde_json::Value` (not the typed
   struct), are idempotent, pure, tolerant of partial data, and tested in the same change.
3. **Never repurpose a field name** for a new meaning, and never introduce
   `deny_unknown_fields` (it breaks forward-compat).
4. **Tokens.** The keyring is keyed by server name and is unversioned; a server rename
   must move its keyring entry, and any change to the token storage scheme migrates
   lazily on first read (as the keyring 3.x -> 4.x move did).
5. **Regenerable state is exempt.** The per-server entity cache (ADR 0007) and the
   localStorage dashboard cache are derived state, not settings; they may be versioned
   and discarded freely.

Compatibility is one-directional: the new binary reads old files; an older binary is not
guaranteed to read a newer file (which it leaves untouched rather than downgrading).

The detailed rules, the additive/breaking table, and the identified gaps live in the
design spec (`docs/superpowers/specs/2026-07-18-config-compatibility-design.md`).

## Consequences

- Upgrades preserve the user's servers, tokens, and preferences by construction; a lost
  setting becomes a policy violation caught in review, not an accepted upgrade cost.
- Every breaking schema change carries a migration step and a test, so the cost of a
  refactor that touches persisted shape is explicit and bounded.
- The discipline does not apply retroactively: the ADR 0013 refactor remains a one-time
  sanctioned exception, and the current in-flux schema is not held to it.
- Known gaps to close when the discipline takes effect: persisted enums (starting with
  `BackendKind`) lack forward-lenient deserialization, so a file naming an unknown enum
  value fails to load the whole config rather than one field; there are no golden
  old-version fixtures yet; and the keyring key scheme is unversioned. These are tracked
  in the spec and are not urgent while compatibility is waived.
- Forward compatibility (an older binary reading a newer file) is explicitly out of
  scope; the newer file is preserved, not degraded.
