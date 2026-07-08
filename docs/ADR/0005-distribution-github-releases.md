# 5. Distribution via GitHub Releases; podman for local Linux builds

Date: 2026-07-08

## Status

Accepted

## Context

Releases must be installable on all three target operating systems. Local builds
should not install build toolchains on the host (a project rule); podman is the
sanctioned local build sandbox. However, podman runs Linux containers only, so
macOS and Windows installable artifacts cannot be produced in it — those need
their native runners.

## Decision

Two build paths:

- Local development and Linux artifacts build in podman via a `Containerfile`
  whose base image matches the host distro release (for glibc/ABI parity), with
  proxy forwarding disabled and the cargo registry/target cached in named
  volumes.
- Releases build on a GitHub Actions matrix (ubuntu / macos / windows), each
  runner producing the binary for its own OS. Artifacts (plus `.sha256`) are
  attached to a GitHub Release, gated behind the CI jobs (`needs: [ci]`), tag
  validation and a CHANGELOG check.

The only distribution channel is GitHub Releases; crates.io, AUR and Homebrew
are out of scope for now. GitHub Actions are pinned by major tag (the house
style of the author's other Rust project), not by SHA.

## Consequences

- macOS and Windows installers require GitHub-hosted runners; podman alone
  cannot produce them.
- No `cargo publish` step, so the release workflow keeps the CI gate but omits
  crates.io publication.
- Tauri GUI installers (a later milestone) slot into the same matrix via the
  Tauri packaging action.
