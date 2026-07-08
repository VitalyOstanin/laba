# 11. Stay on reqwest 0.12 with the ring rustls provider

Date: 2026-07-09

## Status

Accepted

## Context

`core` performs HTTPS through `reqwest` with rustls. A version review flagged
`reqwest` 0.13 as available. The 0.13 line changes the default rustls crypto
provider from `ring` to `aws-lc-rs`. Building `aws-lc-rs` compiles `aws-lc-sys`,
which requires `cmake` and a C toolchain on the build host. The project builds
`core` and `cli` on the host without build toolchains (toolchains are confined to
containers), so adopting the 0.13 default would break the host build.

Keeping `ring` under 0.13 is possible but costly: it needs `rustls-no-provider`,
a direct `rustls` dependency, and installing a process-wide `CryptoProvider` in
every entrypoint (CLI and GUI backend). A missing installation is a latent
runtime panic on the first HTTPS call — a path the http-only wiremock tests do
not exercise.

## Decision

Stay on `reqwest` 0.12 with the `ring` provider. Do not upgrade to 0.13 at this
time. The deferral and its trigger conditions are recorded in `TODO.md`.

## Consequences

- The host build needs no C toolchain or `cmake`; toolchains remain
  container-only.
- No manual `CryptoProvider` wiring is required, so there is no latent
  first-HTTPS-call panic to guard against.
- `reqwest` 0.12 is still maintained, so security fixes remain available on the
  current line.
- Revisit when 0.12 stops receiving fixes, when `ring` support is dropped, or
  when the build already requires a C toolchain for another reason.
