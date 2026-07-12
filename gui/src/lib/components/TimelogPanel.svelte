<script lang="ts">
  import { t, locale, type Locale } from "../i18n";
  import { fmtMinutes, fmtSigned, fmtDayMonth } from "../format";
  import type { DayCell } from "../types";

  let { timeline = [] }: { timeline?: DayCell[] } = $props();

  // Only weekdays carry a plan; weekends are dropped from the timeline view.
  const days = $derived(timeline.filter((d) => d.weekday));

  function cellClass(d: DayCell): string {
    if (d.deficit_min > 0) return "deficit";
    if (d.surplus_min > 0) return "surplus";
    return "met";
  }

  function cellDelta(d: DayCell, loc: Locale): string {
    if (d.deficit_min > 0) return fmtSigned(-d.deficit_min, loc);
    if (d.surplus_min > 0) return fmtSigned(d.surplus_min, loc);
    return fmtMinutes(d.logged_min, loc);
  }
</script>

<div class="timelog-panel">
  <h3>{$t("timelog.timeline")}</h3>
  <ul class="timeline">
    {#each days as d (d.date)}
      <li
        class={cellClass(d)}
        title={`${d.date}: ${fmtMinutes(d.logged_min, $locale)} / ${fmtMinutes(d.plan_min, $locale)}`}
      >
        <span class="tl-date">{fmtDayMonth(d.date, $locale)}</span>
        <span class="tl-delta">{cellDelta(d, $locale)}</span>
      </li>
    {/each}
  </ul>
</div>
