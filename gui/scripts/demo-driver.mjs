// Playwright walkthrough of the GUI running against the dev mock (no Tauri
// runtime -> $lib/invoke routes to the anonymized dev-mock fixtures). Drives a
// scripted tour with pauses; record-demo.sh records the Xvfb screen around it.
// Writes a marker file with the start epoch so the recorder can trim the idle
// head. Each step is best-effort so a missing element never aborts the run.
import { chromium } from "playwright";
import fs from "node:fs";

const URL = process.env.DEMO_URL || "http://localhost:1420";
const MARKER = process.env.DEMO_MARKER || "/tmp/laba-demo-marker.json";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function step(name, fn) {
  try {
    await fn();
  } catch (e) {
    console.warn(`[demo] step '${name}' skipped: ${e.message}`);
  }
}

const browser = await chromium.launch({
  headless: false,
  channel: "chrome",
  args: ["--kiosk", "--window-position=0,0", "--window-size=1280,720"],
});
const page = await browser.newPage({ viewport: null });

await page.goto(URL, { waitUntil: "networkidle" });
// Wait for the dashboard to render (timelog bar or columns).
await step("wait-dashboard", () =>
  page.waitForSelector(".timelog, .cols", { timeout: 15000 }),
);
await sleep(800);

// Mark the real start so the recorder trims the idle head.
fs.writeFileSync(MARKER, JSON.stringify({ startedAt: Date.now() }));

await step("show-dashboard", () => sleep(2600));

// Expand the timelog panel.
await step("timelog-expand", async () => {
  await page.click(".timelog-bar", { timeout: 3000 });
  await sleep(2800);
});
await step("timelog-collapse", async () => {
  await page.click(".timelog-bar", { timeout: 3000 });
  await sleep(1000);
});

// Glance through the task column.
await step("scroll-tasks", async () => {
  await page.mouse.move(640, 380);
  await page.mouse.wheel(0, 320);
  await sleep(1600);
  await page.mouse.wheel(0, -320);
  await sleep(800);
});

// Open a task detail if a task row is clickable.
await step("open-task", async () => {
  const row = page.locator(".list > li, .task, .card li").first();
  await row.click({ timeout: 3000 });
  await sleep(2600);
  await page.goBack({ timeout: 3000 }).catch(() => {});
  await sleep(1000);
});

// Settings screen.
await step("open-settings", async () => {
  await page.click(".settings-link", { timeout: 3000 });
  await page.waitForSelector(".settings", { timeout: 5000 });
  await sleep(2400);
  await page.mouse.wheel(0, 400);
  await sleep(2000);
  await page.mouse.wheel(0, -400);
  await sleep(600);
});

// Back to the dashboard to close on the main view.
await step("back-dashboard", async () => {
  await page.click(".back, a[href='/']", { timeout: 3000 }).catch(async () => {
    await page.goto(URL);
  });
  await sleep(2200);
});

await browser.close();
console.log("[demo] walkthrough done");
