#!/usr/bin/env bash
# Fast iteration build of the GUI: the *debug* profile (no release optimization)
# via the container. The first build is slow, but subsequent Rust-only changes
# link incrementally in seconds because `target/` is mounted from the host and
# the mold linker (set up by tauri-container.sh) speeds linking.
#
# Output: target/debug/laba-gui, which runs on the host. For a release
# build use: scripts/tauri-container.sh 'cd gui && npm run tauri -- build --no-bundle'
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$DIR/tauri-container.sh" 'cd gui && npm run tauri -- build --no-bundle --debug'
