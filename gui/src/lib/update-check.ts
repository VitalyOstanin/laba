/**
 * Orchestrates a single update check and publishes its progress to the shared
 * {@link updateState} store. Kept separate from `$lib/updater` (which stays a
 * thin, store-free bridge over the Tauri plugin) and from `$lib/store` (plain
 * state) so neither imports the other. The banner runs this on mount and the
 * header indicator re-runs it to retry after a failed check.
 */
import { get } from "svelte/store";
import { settings, updateState } from "$lib/store";
import { checkForUpdate } from "$lib/updater";

/**
 * Run the update check, honoring the `check_updates` setting. When checking is
 * disabled the state goes straight to `disabled` (no network call). Otherwise it
 * shows `checking`, then resolves to `available` / `current` / `failed`.
 */
export async function runUpdateCheck(): Promise<void> {
  if (!get(settings).check_updates) {
    updateState.set({ phase: "disabled" });
    return;
  }
  updateState.set({ phase: "checking" });
  const res = await checkForUpdate();
  switch (res.status) {
    case "available":
      updateState.set({ phase: "available", update: res.update });
      break;
    case "current":
      updateState.set({ phase: "current" });
      break;
    case "failed":
      updateState.set({ phase: "failed" });
      break;
  }
}
