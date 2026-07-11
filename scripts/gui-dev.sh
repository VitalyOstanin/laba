#!/usr/bin/env bash
# Live full-app development loop: `tauri dev` inside the container, rendering the
# real webview window on the host display. The frontend hot-reloads (Vite HMR) on
# every save with no Rust rebuild; Rust changes trigger an incremental *debug*
# recompile (seconds, not the minutes a release build takes).
#
# This forwards the host compositor into the container (Wayland preferred, X11
# fallback), the session D-Bus (for the tray), and the GPU (/dev/dri, with a
# software-rendering fallback). Display forwarding is environment-specific; if the
# window does not appear, adjust the mounts/env below. For a pure UI loop with no
# container at all, prefer `npm run dev` (browser + the dev-mock invoke bridge).
#
# Usage: scripts/gui-dev.sh
set -euo pipefail

IMAGE="${TAURI_IMAGE:-ivangabriele/tauri:debian-bookworm-22}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DISPLAY_ARGS=()
if [ -n "${WAYLAND_DISPLAY:-}" ] && [ -n "${XDG_RUNTIME_DIR:-}" ] &&
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then
  # Wayland: mount the compositor socket and point GTK/webkit at it.
  DISPLAY_ARGS+=(
    -e "XDG_RUNTIME_DIR=/tmp/xdg"
    -e "WAYLAND_DISPLAY=$WAYLAND_DISPLAY"
    -e "GDK_BACKEND=wayland"
    -v "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY:/tmp/xdg/$WAYLAND_DISPLAY"
  )
elif [ -n "${DISPLAY:-}" ]; then
  # X11 (or XWayland): share the X socket and cookie.
  DISPLAY_ARGS+=(
    -e "DISPLAY=$DISPLAY"
    -e "GDK_BACKEND=x11"
    -v "/tmp/.X11-unix:/tmp/.X11-unix"
  )
else
  echo "no WAYLAND_DISPLAY or DISPLAY set; cannot forward a window" >&2
  exit 1
fi

# Session D-Bus for the SNI tray (best-effort; the app still runs without it).
DBUS_ARGS=()
if [ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
  DBUS_SOCK="${DBUS_SESSION_BUS_ADDRESS#unix:path=}"
  DBUS_SOCK="${DBUS_SOCK%%,*}"
  if [ -S "$DBUS_SOCK" ]; then
    DBUS_ARGS+=(
      -e "DBUS_SESSION_BUS_ADDRESS=unix:path=$DBUS_SOCK"
      -v "$DBUS_SOCK:$DBUS_SOCK"
    )
  fi
fi

# GPU for webkit; falls back to software rendering if /dev/dri is absent.
GPU_ARGS=()
if [ -d /dev/dri ]; then
  GPU_ARGS+=(--device /dev/dri)
else
  GPU_ARGS+=(-e "LIBGL_ALWAYS_SOFTWARE=1" -e "WEBKIT_DISABLE_COMPOSITING_MODE=1")
fi

exec podman run --rm -it --init \
  --security-opt seccomp=unconfined \
  -v "$REPO_ROOT":/work -w /work \
  -v laba-cargo:/usr/local/cargo/registry \
  -v laba-rustup:/usr/local/rustup \
  -v laba-npm:/root/.npm \
  -e PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
  "${DISPLAY_ARGS[@]}" "${DBUS_ARGS[@]}" "${GPU_ARGS[@]}" \
  "$IMAGE" \
  bash -c '
    unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY
    TCBIN="$(rustc --print sysroot 2>/dev/null)/bin"
    export PATH="$TCBIN:$PATH"
    cd gui && npm run tauri -- dev
  '
