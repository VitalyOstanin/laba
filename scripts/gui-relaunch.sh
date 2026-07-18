#!/usr/bin/env bash
# Launch the laba GUI, always replacing any running instance (singleton where
# the newest launch wins). Bound to the Ctrl+Alt+P hotkey so pressing it after a
# rebuild runs the fresh binary instead of spawning a duplicate.
#
# Which binary runs is decided by a dev marker file (default <repo>/laba.dev):
#   - the installed deb binary (/usr/bin/laba-gui) runs only when it exists AND
#     the marker is present — the normal, stable everyday build;
#   - otherwise the freshly built debug binary (target/debug/laba-gui) runs, i.e.
#     when the deb is not installed, or while developing (delete the marker).
# LABA_GUI_BIN overrides the decision with an explicit binary path.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DEV_MARKER="${LABA_DEV_MARKER:-$REPO_ROOT/laba.dev}"
DEBUG_BIN="$REPO_ROOT/target/debug/laba-gui"
DEB_BIN="/usr/bin/laba-gui"

if [[ -n "${LABA_GUI_BIN:-}" ]]; then
  bin="$LABA_GUI_BIN"
elif [[ -e "$DEV_MARKER" && -x "$DEB_BIN" ]]; then
  # Marker present and the deb is installed: run the stable deb build.
  bin="$DEB_BIN"
elif [[ -x "$DEBUG_BIN" ]]; then
  # Developing (marker deleted) or deb not installed: run the debug build.
  bin="$DEBUG_BIN"
else
  # Debug build missing: fall back to the deb if it is installed.
  bin="$DEB_BIN"
fi

if [[ ! -x "$bin" ]]; then
  echo "laba: no runnable binary found (looked for $bin)" >&2
  exit 1
fi

# Terminate any running instance and wait briefly for it to release the tray and
# window resources before starting the new one.
pkill -x laba-gui 2>/dev/null || true
for _ in 1 2 3 4 5; do
  pgrep -x laba-gui >/dev/null 2>&1 || break
  sleep 0.2
done

exec "$bin" "$@"
