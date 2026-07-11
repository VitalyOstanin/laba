#!/usr/bin/env bash
# Build the Linux release binary inside podman (proxy forwarding disabled;
# cargo registry/target cached in named volumes).
set -euo pipefail

img=laba-build
podman build --http-proxy=false -t "$img" -f Containerfile .
podman run --rm --http-proxy=false -v "$PWD":/work \
  -v opc-cargo-registry:/root/.cargo/registry \
  -v opc-cargo-target:/work/target \
  "$img" \
  cargo build --release --locked --bin laba
echo "binary: target/release/laba (in the opc-cargo-target volume)" >&2
