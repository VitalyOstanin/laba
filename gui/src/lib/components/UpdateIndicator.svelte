<script lang="ts">
  import { t } from "../i18n";
  import { updateState, updateBannerOpen } from "$lib/store";
  import { runUpdateCheck } from "$lib/update-check";

  // Always-visible header affordance reflecting the single startup update check,
  // in every phase (unlike the banner, which only surfaces an available update).
  // `available` opens the full banner; `failed` retries the check; `checking`
  // and `current` are informational; `disabled` renders nothing.
  const state = $derived($updateState);

  function openBanner(): void {
    updateBannerOpen.set(true);
  }
  function retry(): void {
    void runUpdateCheck();
  }
</script>

{#if state.phase === "checking"}
  <span class="update-status checking" role="status" aria-live="polite">
    <span class="spinner" aria-hidden="true"></span>
    <span>{$t("update.checking")}</span>
  </span>
{:else if state.phase === "available"}
  <button
    type="button"
    class="update-flag"
    onclick={openBanner}
    title={$t("update.available")}
    aria-label={$t("update.available")}
  >
    <span class="update-flag-icon" aria-hidden="true">↓</span>
    <span>{$t("update.headerAction")} {state.update.version}</span>
  </button>
{:else if state.phase === "current"}
  <span
    class="update-status current"
    role="status"
    title={$t("update.upToDate")}
  >
    <span class="update-status-icon" aria-hidden="true">✓</span>
    <span>{$t("update.upToDate")}</span>
  </span>
{:else if state.phase === "failed"}
  <button
    type="button"
    class="update-status failed"
    onclick={retry}
    title={$t("update.checkFailed")}
    aria-label={$t("update.checkFailed")}
  >
    <span class="update-status-icon" aria-hidden="true">!</span>
    <span>{$t("update.checkFailed")}</span>
  </button>
{/if}
