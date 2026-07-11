# laboro-gui

Desktop GUI for laboro — a multi-backend task tracker client (OpenProject,
GitHub). Built with Tauri v2 and SvelteKit (Svelte 5, adapter-static SPA). The
GUI drives the shared `laboro-core` crate for backend access and reuses the
same configuration as the CLI.

## Build and test

The GUI crate requires a WebKit toolchain and is excluded from the host cargo
workspace default members. Build and test it in the project container:

```sh
scripts/tauri-container.sh 'cd gui && npm run check'   # svelte-check
scripts/tauri-container.sh 'cd gui && npm test'        # vitest unit tests
scripts/tauri-container.sh 'cd gui && npm run build'   # SPA build
```

End-to-end (WebDriver) tests run under xvfb with `TAURI_E2E=1`; see
`gui/wdio.conf.js` and the repository `README.md` for the full workflow.
