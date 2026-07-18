/**
 * Self-update bridge over the Tauri updater plugin, with a browser fallback.
 *
 * In a real Tauri window this drives `@tauri-apps/plugin-updater`: `check()`
 * against the GitHub releases endpoint (configured in `tauri.conf.json`), then
 * `downloadAndInstall()` + `relaunch()` on an explicit user action. Under
 * `vite dev` in a plain browser there is no Tauri runtime, so it returns a
 * fixture so the banner can be developed with hot reload; installing is a no-op
 * there. Detection mirrors `$lib/invoke`.
 */
import type { Update } from "@tauri-apps/plugin-updater";

/** Fake install delay in the browser dev stub, so a spinner is briefly visible. */
const DEV_INSTALL_DELAY_MS = 300;

/** An available update the banner can present. */
export interface AvailableUpdate {
  version: string;
  notes: string | null;
}

/**
 * Outcome of an update check, kept distinct so the header indicator can tell
 * "you're on the latest" from "the check failed" — both of which the banner
 * treats the same (nothing to install). `available` carries the found update.
 */
export type UpdateCheck =
  | { status: "available"; update: AvailableUpdate }
  | { status: "current" }
  | { status: "failed" };

/** The latest-release page, opened on platforms that cannot self-update. */
export const RELEASES_URL =
  "https://github.com/VitalyOstanin/laba/releases/latest";

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Whether the running platform can install updates in place. macOS is excluded:
 * without a paid Apple signing key the release ships unsigned, so Gatekeeper
 * blocks a self-installed swap; there the banner points at the release page for
 * a manual download instead. Pure and cheap, so it is unit-tested.
 */
export function canSelfUpdate(): boolean {
  if (typeof navigator === "undefined") return true;
  return !/Mac OS X|Macintosh/i.test(navigator.userAgent);
}

// The `Update` handle from the last successful `check()`, reused by
// `installUpdate` so the download targets the version the user was shown.
let pending: Update | null = null;

/**
 * Whether the update banner should be shown: an update is available and its
 * version is not the one the user already dismissed. Pure, so it is unit-tested.
 */
export function shouldShowUpdate(
  available: AvailableUpdate | null,
  dismissedVersion: string | null | undefined,
): boolean {
  if (!available) return false;
  return available.version !== dismissedVersion;
}

/**
 * Check the configured endpoint for a newer release. Distinguishes three
 * outcomes: an update is `available`, the running version is `current`, or the
 * check `failed` (network/endpoint error). The banner surfaces only
 * `available`; the header indicator reflects all three. In the browser dev
 * environment returns a fixture.
 */
export async function checkForUpdate(): Promise<UpdateCheck> {
  if (!hasTauri) {
    // Dev fixture so the banner renders under `npm run dev`.
    return {
      status: "available",
      update: {
        version: "0.2.0",
        notes: "Example release notes for the dev mock.",
      },
    };
  }
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    pending = update;
    if (!update) return { status: "current" };
    return {
      status: "available",
      update: { version: update.version, notes: update.body ?? null },
    };
  } catch (e) {
    console.error("update check failed:", e);
    return { status: "failed" };
  }
}

/**
 * Download and install the pending update, reporting coarse progress, then
 * relaunch. Only call after {@link checkForUpdate} returned a version. A no-op
 * in the browser dev environment (no bundle to swap).
 */
export async function installUpdate(
  onProgress?: (downloadedBytes: number, contentLength: number | null) => void,
): Promise<void> {
  if (!hasTauri) {
    // Nothing to install in the browser; resolve after a tick so any spinner
    // is visible during UI development.
    await new Promise((r) => setTimeout(r, DEV_INSTALL_DELAY_MS));
    return;
  }
  if (!pending) throw new Error("no pending update — check first");
  let downloaded = 0;
  let total: number | null = null;
  await pending.downloadAndInstall((event) => {
    switch (event.event) {
      case "Started":
        total = event.data.contentLength ?? null;
        onProgress?.(0, total);
        break;
      case "Progress":
        downloaded += event.data.chunkLength;
        onProgress?.(downloaded, total);
        break;
      case "Finished":
        onProgress?.(downloaded, total);
        break;
    }
  });
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}
