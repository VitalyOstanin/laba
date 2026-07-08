<script lang="ts">
  import { t } from "../i18n";
  import { fmtMinutes, fmtSigned } from "../format";
  import type { DayCell } from "../types";

  let { timeline = [] }: { timeline?: DayCell[] } = $props();

  // Only weekdays carry a plan; weekends are dropped from the timeline view.
  const days = $derived(timeline.filter((d) => d.weekday));

  function cellClass(d: DayCell): string {
    if (d.deficit_min > 0) return "deficit";
    if (d.surplus_min > 0) return "surplus";
    return "met";
  }

  function cellDelta(d: DayCell): string {
    if (d.deficit_min > 0) return fmtSigned(-d.deficit_min);
    if (d.surplus_min > 0) return fmtSigned(d.surplus_min);
    return fmtMinutes(d.logged_min);
  }
</script>

<div class="timelog-panel">
  <h3>{$t("timelog.timeline")}</h3>
  <ul class="timeline">
    {#each days as d (d.date)}
      <li class={cellClass(d)} title={`${d.date}: ${fmtMinutes(d.logged_min)} / ${fmtMinutes(d.plan_min)}`}>
        <span class="tl-date">{d.date.slice(5)}</span>
        <span class="tl-delta">{cellDelta(d)}</span>
      </li>
    {/each}
  </ul>
</div>
