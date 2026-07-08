# 3. Token storage in the keyring with a file fallback

Date: 2026-07-08

## Status

Accepted

## Context

Each server profile (ADR 0002) needs an API token. Tokens are secrets and should
not sit in the plaintext config file, but the tool must still work in
environments without a system keyring (headless servers, CI).

The `keyring` crate v3 does not enable any backend by default; a backend feature
must be selected. On Linux the Secret Service backend has two forms:
`sync-secret-service` pulls in `libdbus-sys`, which needs the system build
package `libdbus-1-dev`, and `async-secret-service` uses a pure-Rust D-Bus stack
(zbus) with no system build dependency.

## Decision

Store tokens in the OS keyring under service `taskstream` with the
account set to the profile name. When no keyring backend is available, fall back
to a separate `secrets.json` file with mode `0600` (never in the main config).
Token resolution precedence: `--token` flag, `OPENPROJECT_TOKEN`, keyring, file
fallback.

Enable keyring backends `async-secret-service` (Linux, pure Rust) plus the
cfg-gated `apple-native` and `windows-native`. `sync-secret-service` is rejected
because its `libdbus-1-dev` build dependency violates the project rule against
installing build toolchains on the host.

## Consequences

- On the author's GNOME system, tokens land in the GNOME keyring via D-Bus
  without any system library build dependency.
- Headless/CI environments transparently use the `0600` file fallback.
- The account key is the profile name, so renaming a server's base URL does not
  orphan its token (a future `server rename` would need to move the keyring
  entry).
