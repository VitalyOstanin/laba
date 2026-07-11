#!/usr/bin/env bash
# Run a command inside the Tauri build/test container.
#
# Builds and tests for the `gui` crate need webkit2gtk and a WebDriver, which we
# do not install on the host. This wraps the public `ivangabriele/tauri` image.
#
# Notes:
# - The default rootless network already has direct internet egress, so the
#   host's loopback proxy is not needed; proxy env is cleared inside the
#   container (its 127.0.0.1 address is not reachable from the container anyway).
# - Named volumes cache cargo registry and the rustup toolchain across runs so
#   `stable` (with rustfmt/clippy, per rust-toolchain.toml) is fetched once.
# - cargo-nextest (project test runner) is not in the image and its default home
#   (/usr/local/cargo/bin) is not a mounted volume, so it would vanish between
#   runs. We install it into the toolchain bin, which lives on the persisted
#   rustup volume, and put that bin on PATH. The install is self-healing: it runs
#   once, then the check short-circuits.
# - Rootless podman maps container root to the host user, so files created in
#   the mounted repo are owned by the host user.
# - The default seccomp profile is kept for build/test. Set TAURI_E2E=1 to relax
#   it (seccomp=unconfined) only for e2e, where the webkit/WebDriver sandbox
#   needs syscalls the default profile blocks.
#
# Usage: scripts/tauri-container.sh '<shell command>'
#        TAURI_E2E=1 scripts/tauri-container.sh '<e2e command>'
set -euo pipefail

IMAGE="${TAURI_IMAGE:-ivangabriele/tauri:debian-bookworm-22}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SECCOMP=()
if [ "${TAURI_E2E:-0}" = "1" ]; then
  SECCOMP=(--security-opt seccomp=unconfined)
fi

# --init runs a minimal init (catatonit) as PID 1. Without it, bash exec-optimizes
# the container's sole command into itself, so a wrapped `xvfb-run` becomes PID 1
# and its Xvfb-readiness handshake never completes: xvfb-run backgrounds Xvfb and
# blocks on `wait`, which is only released by the SIGUSR1 Xvfb sends its parent
# when ready — signal delivery to PID 1 does not release that `wait`, so the run
# hangs before launching the command. With --init, xvfb-run runs as a non-PID-1
# child and the handshake works.
exec podman run --rm --init \
  "${SECCOMP[@]}" \
  -v "$REPO_ROOT":/work -w /work \
  -v laba-cargo:/usr/local/cargo/registry \
  -v laba-rustup:/usr/local/rustup \
  -v laba-npm:/root/.npm \
  -e PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
  "$IMAGE" \
  bash -c '
    unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY
    TCBIN="$(rustc --print sysroot 2>/dev/null)/bin"
    export PATH="$TCBIN:$PATH"
    if [ -d "$TCBIN" ] && ! command -v cargo-nextest >/dev/null 2>&1; then
      curl -LsSf --retry 3 https://get.nexte.st/latest/linux \
        | tar zxf - -C "$TCBIN" 2>/dev/null || true
    fi
    '"$*"
