#!/usr/bin/env bash
# Launch the laba GUI, always replacing any running instance (singleton where
# the newest launch wins). Bound to the Ctrl+Alt+P hotkey so pressing it after a
# rebuild runs the fresh binary instead of spawning a duplicate.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="${LABA_GUI_BIN:-$REPO_ROOT/target/debug/laba-gui}"

# Terminate any running instance and wait briefly for it to release the tray and
# window resources before starting the new one.
pkill -x laba-gui 2>/dev/null || true
for _ in 1 2 3 4 5; do
  pgrep -x laba-gui >/dev/null 2>&1 || break
  sleep 0.2
done

exec "$bin" "$@"
