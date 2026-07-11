#!/usr/bin/env bash
# Build the Tauri updater latest.json deterministically from the .sig files
# produced by the per-platform GUI bundle jobs.
#
# Each GUI job uploads its bundle and the matching <bundle>.sig to the release
# with uploadUpdaterJson disabled, so latest.json is assembled here in a single
# job after all platforms finish. This avoids the race that uploadUpdaterJson
# would cause when several jobs write latest.json concurrently.
#
# Platform keys follow the Tauri updater target names. macOS is intentionally
# excluded: without an Apple signing key the app cannot self-update, so macOS
# ships as a notification-only download (see docs/RELEASING.md).
#
# Usage: build-updater-json.sh REPO TAG VERSION SIG_DIR OUT
#   REPO     owner/name (e.g. VitalyOstanin/laba)
#   TAG      release tag (e.g. v0.1.0)
#   VERSION  bare version without the leading v (e.g. 0.1.0)
#   SIG_DIR  directory containing the downloaded *.sig files
#   OUT      path to write latest.json
# Optional env:
#   PUB_DATE  RFC3339 timestamp (default: current UTC time)
#   NOTES     release notes string (default: "See CHANGELOG.md")
set -euo pipefail

repo="${1:?usage: build-updater-json.sh REPO TAG VERSION SIG_DIR OUT}"
tag="${2:?missing TAG}"
version="${3:?missing VERSION}"
sig_dir="${4:?missing SIG_DIR}"
out="${5:?missing OUT}"
pub_date="${PUB_DATE:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
notes="${NOTES:-See CHANGELOG.md}"

platform_for() {
  # Map a bundle filename to its Tauri updater platform key, or empty to skip.
  case "$1" in
    *.AppImage)              echo "linux-x86_64" ;;
    *-setup.exe|*.nsis.zip)  echo "windows-x86_64" ;;
    *.msi)                   echo "windows-x86_64" ;;
    *) echo "" ;;
  esac
}

platforms='{}'
found=0
shopt -s nullglob
for sig in "$sig_dir"/*.sig; do
  bundle="$(basename "${sig%.sig}")"
  platform="$(platform_for "$bundle")"
  if [[ -z "$platform" ]]; then
    echo "skip: no platform mapping for ${bundle}" >&2
    continue
  fi
  signature="$(cat "$sig")"
  # jq @uri encodes the file name so spaces or unusual characters stay valid.
  url="https://github.com/${repo}/releases/download/${tag}/$(jq -rn --arg n "$bundle" '$n|@uri')"
  platforms="$(jq \
    --arg platform "$platform" \
    --arg signature "$signature" \
    --arg url "$url" \
    '.[$platform] = {signature: $signature, url: $url}' <<<"$platforms")"
  found=$((found + 1))
  echo "add: ${platform} <- ${bundle}" >&2
done

if [[ "$found" -eq 0 ]]; then
  echo "no updater signatures found in ${sig_dir}; latest.json not written" >&2
  exit 3
fi

jq -n \
  --arg version "$version" \
  --arg notes "$notes" \
  --arg pub_date "$pub_date" \
  --argjson platforms "$platforms" \
  '{version: $version, notes: $notes, pub_date: $pub_date, platforms: $platforms}' \
  >"$out"

echo "wrote ${out} with ${found} platform(s)" >&2
