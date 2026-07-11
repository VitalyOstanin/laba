# Build image for Linux artifacts in podman (keeps toolchains off the host).
# Base image MUST match the host distro release so glibc/ABI line up — the
# resulting dynamically-linked binary then runs on the host.
#
#   podman build --http-proxy=false -t laboro-build -f Containerfile .
#   podman run --rm --http-proxy=false -v "$PWD":/work \
#     -v opc-cargo-registry:/root/.cargo/registry \
#     -v opc-cargo-target:/work/target \
#     laboro-build \
#     cargo build --release --bin laboro
FROM ubuntu:26.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

# podman build does not set HOME; set CARGO/RUSTUP homes explicitly.
ENV HOME=/root CARGO_HOME=/root/.cargo RUSTUP_HOME=/root/.rustup
ENV PATH="/root/.cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable \
    && cargo --version

WORKDIR /work
