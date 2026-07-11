# 12. Signed in-app self-update via the Tauri updater

Date: 2026-07-11

## Status

Accepted

## Context

ADR 0005 established GitHub Releases as the only distribution channel and noted
that the Tauri GUI installers would slot into the release matrix later. The GUI
now needs to tell users when a newer release exists and, where the platform
allows it, install that release without a manual download.

The Tauri updater verifies each update with a minisign signature, so a signing
keypair is required and the public half must be embedded in the app. On Linux
the updater can only replace an AppImage; `.deb` and `.rpm` cannot self-update.
Users still expect a system package and a no-install archive, independent of the
updater.

## Decision

- Ship the Tauri updater plugin. The public minisign key and the
  `releases/latest/download/latest.json` endpoint are embedded in
  `tauri.conf.json`; the private key lives only in the CI secrets
  `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- The release workflow builds the Linux GUI in three formats: **AppImage** (the
  only self-updatable form, signed, with `.sig` and referenced from
  `latest.json`), **`.deb`** (system install), and a **`.tar.gz`** of the
  AppImage (portable, no install). AppImage is built on the oldest supported
  Ubuntu for wider glibc compatibility.
- The update *notification* is platform-independent: a core module reads the
  anonymous GitHub Releases API and shows the cumulative changelog since the
  running version. Only the *install* action depends on the updater and is thus
  limited to platforms the updater supports.
- The app version has a single source of truth — the Cargo workspace version,
  which the release gate already checks against the tag. `tauri.conf.json` no
  longer pins a separate `version`.
- The release is created once by a dedicated job; the CLI and GUI package jobs
  upload into it, removing the earlier `gh release create` race between matrix
  jobs.

## Consequences

- A signing keypair must exist and its private half be kept only in CI secrets;
  losing it forces a new keypair and a new embedded public key, which breaks
  updates for already-installed apps until they are reinstalled.
- Linux `.deb`/`.tar.gz` users are notified of updates but install manually; only
  AppImage users get in-app self-update.
- macOS/Windows GUI bundles are not produced yet; when added they follow the
  same signed-updater pattern on their native runners.
- The updater endpoint resolves to the latest non-prerelease release, so
  prereleases must not be published as the latest release.
