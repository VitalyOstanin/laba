# 10. Restrictive Content-Security-Policy for the webview

Date: 2026-07-09

## Status

Accepted

## Context

The Tauri desktop client (ADR 0001) renders its UI in a system webview. The
scaffolded `tauri.conf.json` shipped with `"csp": null`, which disables the
Content-Security-Policy: the webview would then permit inline and remote scripts,
remote stylesheets, arbitrary `connect-src` targets and framing. All application
data reaches the frontend through Tauri IPC, not through webview-originated
network calls, so a permissive policy grants reach the app does not need and
widens the impact of any injected markup.

## Decision

Set an explicit CSP in `gui/src-tauri/tauri.conf.json`:

```
default-src 'self';
script-src 'self' 'unsafe-inline';
style-src 'self' 'unsafe-inline';
img-src 'self' data:;
connect-src 'self' ipc: http://ipc.localhost;
object-src 'none';
base-uri 'self';
frame-src 'none'
```

`connect-src` is limited to `self` and the Tauri IPC origins (`ipc:` and
`http://ipc.localhost`) so the frontend can reach the Rust backend but not
arbitrary hosts. `object-src` and `frame-src` are denied. `'unsafe-inline'` is
retained for `script-src`/`style-src` because the SvelteKit static build inlines
bootstrap script and component styles; removing it requires a nonce/hash pipeline
and is deferred.

## Consequences

- Webview-originated requests to third-party hosts are blocked; all server
  traffic continues to flow through `core` on the Rust side.
- Injected markup cannot load remote scripts/styles, frame content, or embed
  objects, narrowing the blast radius of a frontend injection.
- `'unsafe-inline'` remains a known relaxation; tightening it to nonces/hashes is
  tracked as a follow-up and would supersede this element of the policy.
