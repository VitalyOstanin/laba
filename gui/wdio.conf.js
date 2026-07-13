import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import fs from "node:fs";
import os from "node:os";

// WebdriverIO + tauri-driver smoke config. Linux uses WebKitWebDriver, which
// tauri-driver locates on PATH. Run headless under xvfb (see e2e/README.md).
let tauriDriver;

// The compiled Tauri binary. A release build is required: debug builds load the
// frontend from devUrl (a running dev server), while release builds serve the
// embedded frontendDist assets the e2e needs. This is a Cargo workspace, so the
// binary lands in the workspace-root target dir (../target), not under src-tauri.
const application = path.resolve("../target/release/laba-gui");

export const config = {
  runner: "local",
  // tauri-driver speaks the WebDriver protocol on 127.0.0.1:4444; point wdio at
  // it explicitly so it does not try to manage its own browser driver.
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  specs: ["./e2e/**/*.test.js"],
  maxInstances: 1,
  capabilities: [{ "tauri:options": { application } }],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 120000 },
  // Build a production binary with embedded frontend assets. `tauri build`
  // enables the `custom-protocol` feature (plain `cargo build` does not, so its
  // binary would load the dev server instead); `--no-bundle` skips packaging
  // (.deb/.appimage) and just produces target/release/laba-gui. It runs
  // the configured beforeBuildCommand (`npm run build`) for the frontend.
  onPrepare: () => {
    const r = spawnSync("npx", ["tauri", "build", "--no-bundle"], {
      stdio: "inherit",
    });
    if (r.status !== 0) throw new Error("tauri build failed");
    // Seed an isolated config dir with one server so the dashboard renders its
    // columns (the empty state only shows when no server is configured). The
    // server has no token, so the poller fails fast with the "not signed in"
    // sentinel — no network — and the columns still render. Point secrets at a
    // temp file so the run never touches the CI keyring. The app inherits these
    // env vars via tauri-driver.
    const cfgHome = fs.mkdtempSync(path.join(os.tmpdir(), "laba-e2e-"));
    const cfgDir = path.join(cfgHome, "laba");
    fs.mkdirSync(cfgDir, { recursive: true });
    fs.writeFileSync(
      path.join(cfgDir, "config.json"),
      JSON.stringify({
        servers: {
          demo: { base_url: "https://demo.invalid/op", backend: "openproject" },
        },
        default_server: "demo",
      }),
    );
    process.env.XDG_CONFIG_HOME = cfgHome;
    process.env.OPENPROJECT_SECRETS = path.join(cfgHome, "secrets.json");
  },
  beforeSession: () => {
    tauriDriver = spawn("tauri-driver", [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },
  afterSession: () => {
    tauriDriver?.kill();
  },
};
