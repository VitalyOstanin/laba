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
| [0004](0004-cli-free-redesign.md) | CLI is a free redesign, not a drop-in of taskstream-cli | Accepted |
| [0005](0005-distribution-github-releases.md) | Distribution via GitHub Releases; podman for local Linux builds | Accepted |
