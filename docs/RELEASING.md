# Releasing

## Contents

- [Overview](#overview)
- [One-time: updater signing key](#one-time-updater-signing-key)
- [Cutting a release](#cutting-a-release)
- [Published artifacts](#published-artifacts)

## Overview

Releases are built by `.github/workflows/release.yml` on a `v*` tag push (or
`workflow_dispatch` with a tag). The workflow gates on CI, tag/version/changelog
checks, creates one GitHub Release, then uploads the CLI binaries and the Linux
GUI bundle into it. See [ADR 0005](ADR/0005-distribution-github-releases.md) and
[ADR 0012](ADR/0012-signed-self-update.md).

## One-time: updater signing key

The GUI self-update verifies each update with a minisign signature. The public
key is embedded in `gui/src-tauri/tauri.conf.json`; the private key must be
present in CI as two secrets.

Generate a keypair (once) with the Tauri CLI:

```
npm --prefix gui run tauri signer generate -- -w taskstream-updater.key
```

This writes the private key to `taskstream-updater.key` and prints the public
key. If you set a password, remember it; an empty password is allowed.

Set the repository secrets (values never leave CI):

```
gh secret set TAURI_SIGNING_PRIVATE_KEY < taskstream-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # empty line if no password
```

Keep `taskstream-updater.key` out of the repository. If the public key in
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
  of the AppImage, plus `latest.json` for the updater.

Only the AppImage self-updates in place; `.deb` and `.tar.gz` users are notified
of new versions but install manually.
