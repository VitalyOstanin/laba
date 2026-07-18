#!/usr/bin/env bash
# Launch the laba GUI, always replacing any running instance (singleton where
# the newest launch wins). Bound to the Ctrl+Alt+P hotkey so pressing it after a
# rebuild runs the fresh binary instead of spawning a duplicate.
#
# Which binary runs is decided by a marker file (default <repo>/use-debug-binary):
#   - present  -> the freshly built debug binary (target/debug/laba-gui) runs;
#   - absent   -> the installed deb binary (/usr/bin/laba-gui) runs;
#   - if the deb is not installed, the debug binary runs regardless.
# So the everyday default (no marker) runs the stable deb; create the marker
# while developing to force the debug build. LABA_GUI_BIN overrides the decision
# with an explicit binary path.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

DEBUG_MARKER="${LABA_DEBUG_MARKER:-$REPO_ROOT/use-debug-binary}"
DEBUG_BIN="$REPO_ROOT/target/debug/laba-gui"
DEB_BIN="/usr/bin/laba-gui"

if [[ -n "${LABA_GUI_BIN:-}" ]]; then
  bin="$LABA_GUI_BIN"
elif [[ -e "$DEBUG_MARKER" && -x "$DEBUG_BIN" ]]; then
  # Marker present: force the freshly built debug build.
  bin="$DEBUG_BIN"
elif [[ -x "$DEB_BIN" ]]; then
  # No marker and the deb is installed: run the stable deb build.
  bin="$DEB_BIN"
elif [[ -x "$DEBUG_BIN" ]]; then
  # deb not installed: fall back to the debug build.
  bin="$DEBUG_BIN"
else
  # Neither present: name the deb path in the error below.
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
