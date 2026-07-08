<script lang="ts">
  import { t } from "../i18n";
  import { fmtMinutes, fmtSigned } from "../format";
  import { timelog } from "../store";
  import TimelogPanel from "./TimelogPanel.svelte";
  import TimelogCandidates from "./TimelogCandidates.svelte";

  let open = $state(false);

  const tl = $derived($timelog);
</script>

{#if tl}
  <section class="timelog" aria-label={$t("timelog.title")}>
    <button
      type="button"
      class="timelog-bar status-{tl.status.status}"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      <span class="tl-dot"></span>
      <span class="tl-label">{$t("timelog.title")}</span>
      <span class="tl-nums">
        {fmtMinutes(tl.status.logged_min)} / {fmtMinutes(tl.status.planned_min)}
        {#if tl.status.deficit_min > 0}
          <span class="tl-deficit">{fmtSigned(-tl.status.deficit_min)}</span>
        {/if}
        {#if tl.status.surplus_min > 0}
          <span class="tl-surplus">{fmtSigned(tl.status.surplus_min)}</span>
        {/if}
      </span>
    </button>

    {#if !tl.configured}
      <p class="tl-hint">{$t("timelog.notConfigured")}</p>
    {:else if tl.start_is_default}
      <p class="tl-hint">{$t("timelog.defaultHint")}</p>
    {/if}

    {#if tl.excluded.length > 0}
      <p class="tl-hint">{$t("timelog.excluded")} {tl.excluded.join(", ")}</p>
    {/if}

    {#if open && tl.configured}
      <TimelogPanel timeline={tl.timeline} />
      <TimelogCandidates suggestMin={tl.status.today_deficit_min} />
    {/if}
  </section>
{/if}
