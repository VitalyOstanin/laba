<script lang="ts">
  import { onMount } from "svelte";
  import { t, locale } from "../i18n";
  import { fmtMinutes } from "../format";
  import { pickCandidates, listActivities, createTimeEntry } from "../api";
  import { refreshServer, refreshTimelog } from "../poller";
  import type { Candidate, Activity } from "../types";

  // Suggested duration (minutes) to prefill — the current today deficit.
  let { suggestMin = 0 }: { suggestMin?: number } = $props();

  let candidates = $state<Candidate[]>([]);
  let openKey = $state<string | null>(null);
  let duration = $state("");
  let comment = $state("");
  let activity = $state("");
  let busy = $state(false);

  // Cache of activity types per server, loaded lazily when a form opens.
  // Plain non-reactive map: it is never rendered directly, only read into the
  // `activities` $state, so Svelte reactivity is intentionally not wanted here.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const activityCache = new Map<string, Activity[]>();
  let activities = $state<Activity[]>([]);

  const keyOf = (c: Candidate): string => `${c.server}#${c.wp_id}`;

  async function load(): Promise<void> {
    try {
      candidates = await pickCandidates();
    } catch {
      candidates = [];
    }
  }

  onMount(load);

  async function open(c: Candidate): Promise<void> {
    const key = keyOf(c);
    if (openKey === key) {
      openKey = null;
      return;
    }
    openKey = key;
    duration = suggestMin > 0 ? `${suggestMin}m` : "";
    comment = "";
    activity = "";
    activities = activityCache.get(c.server) ?? [];
    if (!activityCache.has(c.server)) {
      try {
        const list = await listActivities(c.server);
        activityCache.set(c.server, list);
        if (openKey === key) activities = list;
      } catch {
        activityCache.set(c.server, []);
      }
    }
  }

  async function submit(c: Candidate): Promise<void> {
    if (busy || !duration.trim()) return;
    busy = true;
    try {
      await createTimeEntry(
        c.server,
        c.wp_id,
        duration.trim(),
        comment.trim() || null,
        activity || null,
      );
      await Promise.all([refreshServer(c.server), refreshTimelog()]);
      openKey = null;
      await load();
    } finally {
      busy = false;
    }
  }
</script>

<div class="candidates">
  <h3>{$t("timelog.candidates")}</h3>
  {#if candidates.length === 0}
    <p class="empty">{$t("timelog.noCandidates")}</p>
  {:else}
    <ul class="candidate-list">
      {#each candidates as c (keyOf(c))}
        <li>
          <span class="cd-logged" title={$t("timelog.logged")}
            >{fmtMinutes(c.logged_min, $locale)}</span
          >
          <span class="cd-subject">#{c.wp_id} {c.subject}</span>
          <span class="bk op">{c.server}</span>
          <button type="button" class="linkbtn" onclick={() => open(c)}
            >{$t("timelog.logTime")}</button
          >
        </li>
        {#if openKey === keyOf(c)}
          <li class="logform">
            <label class="lf-field">
              <span>{$t("timelog.duration")}</span>
              <input
                bind:value={duration}
                placeholder="1h30m"
                aria-label={$t("timelog.duration")}
              />
            </label>
            <label class="lf-field">
              <span>{$t("timelog.activity")}</span>
              <select bind:value={activity} aria-label={$t("timelog.activity")}>
                <option value="">—</option>
                {#each activities as a (a.id)}
                  <option value={a.name}>{a.name}</option>
                {/each}
              </select>
            </label>
            <label class="lf-field wide">
              <span>{$t("timelog.commentField")}</span>
              <input
                bind:value={comment}
                aria-label={$t("timelog.commentField")}
              />
            </label>
            <div class="lf-actions">
              <button
                type="button"
                class="btn"
                disabled={busy || !duration.trim()}
                onclick={() => submit(c)}>{$t("timelog.log")}</button
              >
              <button
                type="button"
                class="linkbtn"
                onclick={() => (openKey = null)}>{$t("task.cancel")}</button
              >
            </div>
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</div>
