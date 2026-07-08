#!/usr/bin/env bash
# Build the Linux release binary inside podman (proxy forwarding disabled;
# cargo registry/target cached in named volumes).
set -euo pipefail

img=taskstream-build
podman build --http-proxy=false -t "$img" -f Containerfile .
podman run --rm --http-proxy=false -v "$PWD":/work \
  -v opc-cargo-registry:/root/.cargo/registry \
  -v opc-cargo-target:/work/target \
  "$img" \
  cargo build --release --locked --bin taskstream
echo "binary: target/release/taskstream (in the opc-cargo-target volume)" >&2
