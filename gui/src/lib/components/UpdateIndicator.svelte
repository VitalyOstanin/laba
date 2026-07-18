<script lang="ts">
  import { t } from "../i18n";
  import { availableUpdate, updateBannerOpen } from "$lib/store";

  // Always-visible header affordance: shown whenever the startup check found an
  // update, independent of the update banner's "remind later"/"skip" state, so
  // the update action is never lost once the banner is dismissed. Clicking
  // forces the full banner open (changelog + install/skip/later).
  function openBanner(): void {
    updateBannerOpen.set(true);
  }
</script>

{#if $availableUpdate}
  <button
    type="button"
    class="update-flag"
    onclick={openBanner}
    title={$t("update.available")}
    aria-label={$t("update.available")}
  >
    <span class="update-flag-icon" aria-hidden="true">↓</span>
    <span class="update-flag-label"
      >{$t("update.headerAction")} {$availableUpdate.version}</span
    >
  </button>
{/if}
