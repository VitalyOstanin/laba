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

exec podman run --rm \
  "${SECCOMP[@]}" \
  -v "$REPO_ROOT":/work -w /work \
  -v taskstream-cargo:/usr/local/cargo/registry \
  -v taskstream-rustup:/usr/local/rustup \
  -v taskstream-npm:/root/.npm \
  -e PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin \
  "$IMAGE" \
  bash -c 'unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY; '"$*"
