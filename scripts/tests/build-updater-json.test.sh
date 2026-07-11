#!/usr/bin/env bash
# Regression tests for scripts/build-updater-json.sh. Self-contained: no bats or
# other runner needed, just bash + jq. Exits non-zero on the first failure.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
script="$here/../build-updater-json.sh"
repo="VitalyOstanin/laba"

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

# A fresh work dir with the given "<name>.sig" files, each holding a fake sig.
make_sigs() {
  local dir
  dir="$(mktemp -d)"
  local name
  for name in "$@"; do
    printf 'SIG(%s)' "$name" >"$dir/$name.sig"
  done
  echo "$dir"
}

# 1. Linux + Windows both map; macOS app bundle is skipped.
d="$(make_sigs \
  "laba_0.1.0_amd64.AppImage" \
  "laba_0.1.0_x64-setup.exe" \
  "laba_0.1.0_aarch64.app.tar.gz")"
out="$d/latest.json"
PUB_DATE=2026-07-11T10:00:00Z "$script" "$repo" v0.1.0 0.1.0 "$d" "$out"
[[ "$(jq -r '.version' "$out")" == "0.1.0" ]] || fail "version not propagated"
[[ "$(jq -r '.pub_date' "$out")" == "2026-07-11T10:00:00Z" ]] || fail "pub_date not propagated"
[[ "$(jq -r '.platforms | keys | join(",")' "$out")" == "linux-x86_64,windows-x86_64" ]] \
  || fail "unexpected platform set"
[[ "$(jq -r '.platforms["linux-x86_64"].signature' "$out")" == "SIG(laba_0.1.0_amd64.AppImage)" ]] \
  || fail "linux signature mismatch"
[[ "$(jq -r '.platforms["linux-x86_64"].url' "$out")" \
   == "https://github.com/$repo/releases/download/v0.1.0/laba_0.1.0_amd64.AppImage" ]] \
  || fail "linux url mismatch"
[[ "$(jq -r '.platforms["windows-x86_64"].url' "$out")" \
   == "https://github.com/$repo/releases/download/v0.1.0/laba_0.1.0_x64-setup.exe" ]] \
  || fail "windows url mismatch"
[[ "$(jq 'has("platforms") and (.platforms | has("darwin-aarch64") | not)' "$out")" == "true" ]] \
  || fail "macOS should not appear"
rm -rf "$d"

# 2. The NSIS .nsis.zip name also maps to windows.
d="$(make_sigs "laba_0.1.0_x64.nsis.zip")"
"$script" "$repo" v0.1.0 0.1.0 "$d" "$d/latest.json"
[[ "$(jq -r '.platforms | keys | join(",")' "$d/latest.json")" == "windows-x86_64" ]] \
  || fail ".nsis.zip should map to windows"
rm -rf "$d"

# 3. A name with a space is URL-encoded in the download url.
d="$(mktemp -d)"
printf 'SIG' >"$d/laba 0.1.0.AppImage.sig"
"$script" "$repo" v0.1.0 0.1.0 "$d" "$d/latest.json"
[[ "$(jq -r '.platforms["linux-x86_64"].url' "$d/latest.json")" \
   == "https://github.com/$repo/releases/download/v0.1.0/laba%200.1.0.AppImage" ]] \
  || fail "space not percent-encoded in url"
rm -rf "$d"

# 4. An empty sig dir exits 3 and writes no file.
d="$(mktemp -d)"
rc=0
"$script" "$repo" v0.1.0 0.1.0 "$d" "$d/latest.json" || rc=$?
[[ "$rc" -eq 3 ]] || fail "empty dir should exit 3 (got $rc)"
[[ ! -e "$d/latest.json" ]] || fail "empty dir should not write latest.json"
rm -rf "$d"

echo "all build-updater-json tests passed"
