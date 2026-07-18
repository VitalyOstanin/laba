// Playwright walkthrough of the GUI running against the dev mock (no Tauri
// runtime -> $lib/invoke routes to the anonymized dev-mock fixtures). The tour
// starts at `?demo=wizard`, which makes the mock report no servers so the
// first-run setup wizard opens; it walks the wizard, then tours the dashboard
// it fills in. record-demo.sh records the Xvfb screen around this.
//
// A marker file (start epoch) lets the recorder trim the idle head. If DEMO_SHOTS
// is set, clean PNG screenshots of key screens are saved there for the README.
// Each step is best-effort so a missing element never aborts the run.
import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";

const BASE = process.env.DEMO_URL || "http://localhost:1420";
const URL = BASE.includes("?") ? `${BASE}&demo=wizard` : `${BASE}?demo=wizard`;
const MARKER = process.env.DEMO_MARKER || "/tmp/laba-demo-marker.json";
const SHOTS = process.env.DEMO_SHOTS || "";

if (SHOTS) fs.mkdirSync(SHOTS, { recursive: true });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function step(name, fn) {
  try {
    await fn();
  } catch (e) {
    console.warn(`[demo] step '${name}' skipped: ${e.message}`);
  }
}

async function shot(name) {
  if (!SHOTS) return;
  await step(`shot:${name}`, () =>
    page.screenshot({ path: path.join(SHOTS, `${name}.png`) }),
  );
}

const browser = await chromium.launch({
  headless: false,
  channel: "chrome",
  args: ["--kiosk", "--window-position=0,0", "--window-size=1280,720"],
});
const page = await browser.newPage({ viewport: null });

await page.goto(URL, { waitUntil: "networkidle" });
// The wizard opens automatically when no server is configured.
await step("wait-wizard", () =>
  page.waitForSelector(".wizard-overlay", { timeout: 15000 }),
);
// Drop the ?demo=wizard flag from the address bar before recording starts, so
// the demo does not show the dev-only query (the mock has already read it).
await step("strip-demo-flag", () =>
  page.evaluate(() => history.replaceState({}, "", location.pathname)),
);
await sleep(800);

// Mark the real start so the recorder trims the idle head.
fs.writeFileSync(MARKER, JSON.stringify({ startedAt: Date.now() }));

const wizardNext = () =>
  page.click(".wizard-nav .btn.primary", { timeout: 3000 });

// Wizard step 1: pick the GitHub backend (the primary backend). Wait for the
// mock gh probe to report "ready" so the Continue button enables.
await step("wizard-backend", async () => {
  await page.click(".wizard-card:has-text('GitHub')", { timeout: 3000 });
  await page.waitForSelector(".wizard-nav .btn.primary:not([disabled])", {
    timeout: 5000,
  });
  await sleep(1400);
  await shot("wizard-backend");
  await wizardNext();
  await sleep(1000);
});

// Wizard step 2: connection. The GitHub host is prefilled; the seed name "oss"
// brings the GitHub issue/notification fixtures along.
await step("wizard-connection", async () => {
  const fields = page.locator(".wizard-field input");
  await fields.nth(0).fill("oss");
  await fields.nth(1).fill("github.com");
  await fields.nth(2).fill("OSS Issues");
  await sleep(1400);
  await wizardNext();
  await sleep(1000);
});

// Wizard step 3: review and finish (GitHub skips the token step) -> the
// dashboard fills in.
await step("wizard-finish", async () => {
  await sleep(1200);
  await page.click(".wizard-nav .btn.primary", { timeout: 3000 });
  await sleep(1000);
});

// Dashboard.
await step("wait-dashboard", () =>
  page.waitForSelector(".timelog, .cols", { timeout: 10000 }),
);
await sleep(1200);
await shot("dashboard");
await step("show-dashboard", () => sleep(1800));

// Glance through the task column (GitHub issues and pull requests). The GitHub
// backend has no in-app task detail (rows open on github.com) and no time
// tracking, so the demo tours the list instead of opening a detail pane.
await step("scroll-tasks", async () => {
  await page.mouse.move(640, 380);
  await page.mouse.wheel(0, 320);
  await sleep(1200);
  await page.mouse.wheel(0, -320);
  await sleep(900);
});

// Settings screen.
await step("open-settings", async () => {
  await page.click(".settings-link", { timeout: 3000 });
  await page.waitForSelector(".settings", { timeout: 5000 });
  await sleep(1000);
  await shot("settings");
  await sleep(800);
  await page.mouse.wheel(0, 400);
  await sleep(1100);
  await page.mouse.wheel(0, -400);
  await sleep(600);
});

// Back to the dashboard to close on the main view.
await step("back-dashboard", async () => {
  await page.click(".back, a[href='/']", { timeout: 3000 }).catch(async () => {
    await page.goto(BASE);
  });
  await sleep(1200);
});

await browser.close();
console.log("[demo] walkthrough done");
