<script lang="ts">
  import { activeServer, syncByServer, settings } from "../store";
  import { t, locale } from "../i18n";
  import { fmtDateTime, fmtRelative } from "../format";

  // The active server's sync phase drives the bar. Absent until the first poll
  // starts, in which case nothing is shown.
  const info = $derived(
    $activeServer ? $syncByServer[$activeServer] : undefined,
  );

  // Timestamp of the last successful sync, formatted per the relative-times
  // setting (absolute by default), with the other form offered on hover.
  function stamp(ms: number): string {
    const iso = new Date(ms).toISOString();
    return $settings.relative_times
      ? fmtRelative(iso, $locale)
      : fmtDateTime(iso, $locale, $settings.timezone);
  }
  function stampAlt(ms: number): string {
    const iso = new Date(ms).toISOString();
    return $settings.relative_times
      ? fmtDateTime(iso, $locale, $settings.timezone)
      : fmtRelative(iso, $locale);
  }
</script>

{#if info}
  <div class="syncbar {info.phase}" role="status" aria-live="polite">
    {#if info.phase === "syncing"}
      <span class="spinner" aria-hidden="true"></span>
      <span>{$t("sync.updating")}</span>
      {#if info.lastSyncMs != null}
        <span class="when" title={stampAlt(info.lastSyncMs)}
          >{$t("sync.asOf")} {stamp(info.lastSyncMs)}</span
        >
      {/if}
    {:else if info.phase === "idle"}
      <span class="dot ok" aria-hidden="true"></span>
      <span>{$t("sync.synced")}</span>
      {#if info.lastSyncMs != null}
        <span class="when" title={stampAlt(info.lastSyncMs)}
          >{stamp(info.lastSyncMs)}</span
        >
      {/if}
    {:else}
      <span class="dot warn" aria-hidden="true"></span>
      <span
        >{$t("sync.offline")}{#if info.lastSyncMs != null}
          — {stamp(info.lastSyncMs)}{/if}</span
      >
    {/if}
  </div>
{/if}
