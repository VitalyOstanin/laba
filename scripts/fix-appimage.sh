#!/usr/bin/env bash
# Post-process a Tauri-built AppImage so it renders on modern distributions.
#
# Tauri bundles libwayland-client.so.0 into the AppImage. The AppImage
# excludelist (AppImageCommunity/pkg2appimage) mandates that this library be
# provided by the host, not bundled: a bundled libwayland-client conflicts with
# the host's Mesa/EGL stack and the webview fails to initialise
# (`Could not create default EGL display: EGL_BAD_PARAMETER`), leaving a blank
# window on newer systems. See https://gitlab.freedesktop.org/mesa/mesa/-/issues/11316
#
# This drops that one library, repacks the AppImage over the same path, and
# re-signs it for the updater (the signature must match the repacked bytes).
# Everything else (glib and friends) stays bundled, per the current excludelist.
#
# Usage: fix-appimage.sh <appimage-path>
# Signing requires TAURI_SIGNING_PRIVATE_KEY[_PASSWORD] and @tauri-apps/cli in
# gui/. appimagetool is downloaded unless APPIMAGETOOL points at a binary.
set -euo pipefail

appimage="${1:?usage: fix-appimage.sh <appimage-path>}"
test -f "$appimage" || { echo "no such AppImage: $appimage" >&2; exit 1; }
appimage="$(readlink -f "$appimage")"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
workdir="$(dirname "$appimage")"

tool="${APPIMAGETOOL:-}"
if [ -z "$tool" ]; then
  tool="$workdir/appimagetool"
  curl -fsSL -o "$tool" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x "$tool"
fi

# Extract, drop the excludelisted library, repack over the same filename so the
# asset name and the updater URL stay unchanged.
rm -rf "$workdir/squashfs-root"
( cd "$workdir" && "$appimage" --appimage-extract >/dev/null )
find "$workdir/squashfs-root" -name 'libwayland-client.so*' -delete
rm -f "$appimage"
ARCH=x86_64 "$tool" --appimage-extract-and-run "$workdir/squashfs-root" "$appimage" >/dev/null
rm -rf "$workdir/squashfs-root"

# Re-sign for the updater; the previous signature is over the pre-repack bytes.
( cd "$repo_root/gui" && npx --no-install tauri signer sign \
    -k "${TAURI_SIGNING_PRIVATE_KEY}" \
    -p "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" \
    "$appimage" >/dev/null )

echo "fixed and re-signed: $appimage"
