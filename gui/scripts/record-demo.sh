#!/usr/bin/env bash
# Record a GUI demo against the dev mock: `vite dev` -> Chrome (kiosk) under Xvfb,
# captured by ffmpeg (x11grab), driven by demo-driver.mjs. Produces media/demo.gif.
#
# No Tauri runtime is involved, so the frontend routes through the anonymized
# dev-mock fixtures ($lib/invoke falls back to the mock when __TAURI_INTERNALS__
# is absent). Requirements on PATH: node, Xvfb, xdpyinfo, ffmpeg, and Google
# Chrome (Playwright drives the system browser; no browser is downloaded).
#
# Usage: gui/scripts/record-demo.sh   (run from anywhere)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUI="$(cd "$SCRIPT_DIR/.." && pwd)"
MEDIA="$GUI/media"
WORK="$(mktemp -d)"

DISPLAY_NUM="${DISPLAY_NUM:-:99}"
GEOM="${GEOM:-1280x720}"
MARKER="$WORK/marker.json"
RAW="$WORK/demo-raw.mp4"
FINAL="$WORK/demo.mp4"
PALETTE="$WORK/palette.png"
GIF="$MEDIA/demo.gif"
SHOTS="$MEDIA/screenshots"
# GIF frame rate and width — kept modest to keep the file small for a public repo.
GIF_FPS="${GIF_FPS:-10}"
GIF_WIDTH="${GIF_WIDTH:-800}"

mkdir -p "$MEDIA" "$SHOTS"

pids=()
cleanup() {
  for p in "${pids[@]:-}"; do kill "$p" 2>/dev/null || true; done
  wait 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

# 1. Ensure Playwright is available next to this script (uses system Chrome).
if [ ! -d "$SCRIPT_DIR/node_modules/playwright" ]; then
  echo "[rec] installing playwright (system Chrome, no browser download)"
  ( cd "$SCRIPT_DIR" && npm init -y >/dev/null 2>&1 || true
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm i playwright@^1.61 >/dev/null 2>&1 )
fi

# 2. vite dev server.
echo "[rec] starting vite dev"
( cd "$GUI" && npm run dev >"$WORK/vite.log" 2>&1 ) &
pids+=($!)
for _ in $(seq 1 60); do
  curl -sf "http://localhost:1420" >/dev/null 2>&1 && break
  sleep 0.5
done

# 3. Xvfb.
echo "[rec] starting Xvfb $DISPLAY_NUM ($GEOM)"
Xvfb "$DISPLAY_NUM" -screen 0 "${GEOM}x24" -nolisten tcp -noreset >/dev/null 2>&1 &
pids+=($!)
for _ in $(seq 1 40); do
  DISPLAY="$DISPLAY_NUM" xdpyinfo >/dev/null 2>&1 && break
  sleep 0.15
done

# 4. ffmpeg screen capture (SIGINT finalises the file).
echo "[rec] starting ffmpeg -> $RAW"
FF_START=$(node -e 'console.log(Date.now())')
ffmpeg -loglevel warning -f x11grab -framerate 30 -video_size "$GEOM" \
  -i "$DISPLAY_NUM" -y -c:v libx264 -preset ultrafast -pix_fmt yuv420p "$RAW" &
FF_PID=$!
pids+=($FF_PID)
sleep 0.8

# 5. Drive the walkthrough.
echo "[rec] running walkthrough"
DISPLAY="$DISPLAY_NUM" DEMO_URL="http://localhost:1420" DEMO_MARKER="$MARKER" \
  DEMO_SHOTS="$SHOTS" NODE_PATH="$SCRIPT_DIR/node_modules" \
  node "$SCRIPT_DIR/demo-driver.mjs"

# 6. Stop ffmpeg cleanly (SIGINT finalises the moov atom).
kill -INT "$FF_PID" 2>/dev/null || true
wait "$FF_PID" 2>/dev/null || true

# 7. Trim the idle head using the marker written when the walkthrough started.
TRIM=0
if [ -f "$MARKER" ]; then
  TRIM=$(node -e "const m=require('$MARKER');console.log(Math.max(0,(m.startedAt-$FF_START)/1000).toFixed(2))")
fi
echo "[rec] trim head: ${TRIM}s"
ffmpeg -loglevel warning -ss "$TRIM" -i "$RAW" -y -c:v libx264 -preset veryfast \
  -pix_fmt yuv420p -movflags +faststart "$FINAL"

# 8. GIF via palettegen/paletteuse. `-update 1` marks the palette as a single
#    still image so ffmpeg does not warn about a missing image-sequence pattern.
VF="fps=${GIF_FPS},scale=${GIF_WIDTH}:-1:flags=lanczos"
ffmpeg -loglevel warning -i "$FINAL" -vf "${VF},palettegen" -update 1 -y "$PALETTE"
ffmpeg -loglevel warning -i "$FINAL" -i "$PALETTE" \
  -lavfi "${VF}[x];[x][1:v]paletteuse" -y "$GIF"

echo "[rec] done:"
ls -lh "$GIF" "$SHOTS"/*.png 2>/dev/null
