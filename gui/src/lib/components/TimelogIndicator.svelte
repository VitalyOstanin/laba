<script lang="ts">
  import { t, locale } from "../i18n";
  import { fmtMinutes, fmtSigned } from "../format";
  import { timelog } from "../store";
  import { onGlobalEscape } from "../keys";
  import TimelogPanel from "./TimelogPanel.svelte";
  import TimelogCandidates from "./TimelogCandidates.svelte";

  let open = $state(false);

  // Current aggregated work-log status (null hides the indicator: either no data
  // yet, or no enabled server supports time tracking).
  const timelogState = $derived($timelog);

  // While expanded, ESC (outside a text field) collapses the panel.
  $effect(() => {
    if (!open) return;
    return onGlobalEscape(() => (open = false));
  });
</script>

{#if timelogState}
  <section class="timelog" aria-label={$t("timelog.title")}>
    <button
      type="button"
      class="timelog-bar status-{timelogState.status.status}"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      <span class="tl-dot"></span>
      <span class="tl-label">{$t("timelog.title")}</span>
      <span class="tl-nums">
        {fmtMinutes(timelogState.status.logged_min, $locale)} / {fmtMinutes(
          timelogState.status.planned_min,
          $locale,
        )}
        {#if timelogState.status.deficit_min > 0}
          <span class="tl-deficit"
            >{fmtSigned(-timelogState.status.deficit_min, $locale)}</span
          >
        {/if}
        {#if timelogState.status.surplus_min > 0}
          <span class="tl-surplus"
            >{fmtSigned(timelogState.status.surplus_min, $locale)}</span
          >
        {/if}
      </span>
    </button>

    {#if !timelogState.configured}
      <p class="tl-hint">{$t("timelog.notConfigured")}</p>
    {:else if timelogState.start_is_default}
      <p class="tl-hint">{$t("timelog.defaultHint")}</p>
    {/if}

    {#if timelogState.excluded.length > 0}
      <p class="tl-hint">
        {$t("timelog.excluded")}
        {timelogState.excluded.join(", ")}
      </p>
    {/if}

    {#if open && timelogState.configured}
      <TimelogPanel timeline={timelogState.timeline} />
      <TimelogCandidates suggestMin={timelogState.status.today_deficit_min} />
    {/if}
  </section>
{/if}
