# Architecture Decision Records

This directory records significant architectural decisions using the
[Michael Nygard format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).
The body of an accepted ADR is immutable; only its `Status` field is updated as
a decision is superseded or deprecated. A changed decision is captured by a new
ADR that supersedes the old one.

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-tauri-rust-workspace.md) | Tauri desktop client with a shared Rust core | Accepted |
| [0002](0002-multiple-server-profiles.md) | Multiple server profiles with a JSON config | Accepted |
| [0003](0003-token-storage-keyring-file-fallback.md) | Token storage in the keyring with a file fallback | Accepted |
| [0004](0004-cli-free-redesign.md) | CLI is a free redesign, not a drop-in of openproject-cli | Accepted |
| [0005](0005-distribution-github-releases.md) | Distribution via GitHub Releases; podman for local Linux builds | Accepted |
| [0006](0006-error-code-mapping-and-output.md) | Error code mapping and output formats | Accepted |
| [0007](0007-per-server-stable-entity-cache.md) | Per-server two-tier cache of stable entities | Accepted |
| [0008](0008-drop-auth-import.md) | Drop the `auth import` command | Accepted |
| [0009](0009-token-input-stdin-only.md) | Token input via stdin or `--token` only, no interactive prompt | Accepted |
| [0010](0010-webview-content-security-policy.md) | Restrictive Content-Security-Policy for the webview | Accepted |
| [0011](0011-reqwest-ring-crypto-provider.md) | Stay on reqwest 0.12 with the ring rustls provider | Accepted |
| [0012](0012-signed-self-update.md) | Signed in-app self-update via the Tauri updater | Accepted |
| [0013](0013-backend-trait-capabilities-typed-entities.md) | Backend abstraction: trait, capabilities, and typed cross-backend entities | Accepted |
| [0014](0014-config-forward-only-compatibility.md) | Forward-only backward compatibility for persisted settings | Accepted |
