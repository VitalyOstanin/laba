import { openUrl } from "@tauri-apps/plugin-opener";

// Open a URL in the system browser. Best-effort: a failure is logged, never
// thrown, so a click on a task link can't break the UI.
export async function openExternal(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (e) {
    console.error("open external url failed", e);
  }
}
