# Releasing

## Contents

- [Overview](#overview)
- [One-time: updater signing key](#one-time-updater-signing-key)
- [Cutting a release](#cutting-a-release)
- [Published artifacts](#published-artifacts)

## Overview

Releases are built by `.github/workflows/release.yml` on a `v*` tag push (or
`workflow_dispatch` with a tag). The workflow gates on CI, tag/version/changelog
checks, creates one GitHub Release, then uploads the CLI binaries and the GUI
bundles for Linux, Windows and macOS into it. See
[ADR 0005](ADR/0005-distribution-github-releases.md) and
[ADR 0012](ADR/0012-signed-self-update.md).

The GUI bundle jobs upload their signatures with `uploadUpdaterJson` disabled; a
separate `updater-json` job then assembles `latest.json` once, deterministically,
from the uploaded `.sig` files (`scripts/build-updater-json.sh`). This avoids the
race that per-job `latest.json` writes would otherwise cause.

macOS ships without a signing key (a paid Apple Developer account would be
required to notarize a self-updating build), so its DMGs carry no updater
artifacts and macOS users are notified in-app but install manually — the GUI
banner opens the release page there instead of self-installing. Auto-update
covers Linux (AppImage) and Windows (NSIS).

## One-time: updater signing key

The GUI self-update verifies each update with a minisign signature. The public
key is embedded in `gui/src-tauri/tauri.conf.json`; the private key must be
present in CI as two secrets.

Generate a keypair (once) with the Tauri CLI:

```
npm --prefix gui run tauri signer generate -- -w laba-updater.key
```

This writes the private key to `laba-updater.key` and prints the public
key. If you set a password, remember it; an empty password is allowed.

Set the repository secrets (values never leave CI):

```
gh secret set TAURI_SIGNING_PRIVATE_KEY < laba-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # empty line if no password
```

Keep `laba-updater.key` out of the repository. If the public key in
`tauri.conf.json` is ever regenerated, already-installed apps can no longer
self-update until reinstalled.

## Cutting a release

1. Bump the workspace `version` in `Cargo.toml` and add a matching
   `CHANGELOG.md` section.
2. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The workflow validates that the tag equals the manifest version and that the
   changelog has an entry, then builds and uploads.

## Published artifacts

- CLI: per-target `.tar.gz` / `.zip` archives plus `.sha256`.
- Linux GUI: `.AppImage` (+ `.AppImage.sig`), `.deb`, and a portable `.tar.gz`
  of the AppImage.
- Windows GUI: NSIS installer `*-setup.exe` (+ `.sig`).
- macOS GUI: `.dmg` for `aarch64` and `x86_64` (no updater signature).
- `latest.json` aggregated from the Linux and Windows `.sig` files (platform
  keys `linux-x86_64`, `windows-x86_64`).

The AppImage and the Windows installer self-update in place; `.deb`, the
portable `.tar.gz`, and the macOS `.dmg` are notified of new versions but
installed manually. An unsigned build (signing secrets absent) skips
`latest.json`, so nothing self-updates until a signed release is cut.
