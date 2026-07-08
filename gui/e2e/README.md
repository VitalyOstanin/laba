# e2e (WebdriverIO + tauri-driver)

Smoke scaffold for the end-to-end dashboard check. A full suite comes later; this
is a single test: the app starts and both columns (`My tasks`, `Notifications`)
render.

## Contents

- [Requirements](#requirements)
- [Running](#running)
- [How it works](#how-it-works)

## Requirements

- `tauri-driver` (on PATH), `WebKitWebDriver` (Linux), `xvfb` — all present in the
  `ivangabriele/tauri:debian-bookworm-22` image.
- WebdriverIO dev dependencies (`@wdio/cli`, `@wdio/local-runner`,
  `@wdio/mocha-framework`, `@wdio/spec-reporter`, `webdriverio`).

## Running

In the container only, under xvfb (webkit needs an X server):

```bash
TAURI_E2E=1 scripts/tauri-container.sh 'cd gui && xvfb-run -a npx wdio run wdio.conf.js'
```

`TAURI_E2E=1` relaxes seccomp (needed by the webkit/WebDriver sandbox). The
`onPrepare` hook in `wdio.conf.js` builds the frontend (`npm run build`) and the
debug binary (`cargo build`); then `tauri-driver` launches the binary through
`WebKitWebDriver`.

## How it works

- `wdio.conf.js` — WebdriverIO config: the `tauri:options.application` capability
  points at `src-tauri/target/debug/taskstream-gui`; `tauri-driver` is started in
  `beforeSession` and killed in `afterSession`.
- `e2e/*.test.js` — specs (mocha `describe`/`it`).
