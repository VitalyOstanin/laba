<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import { settings, availableUpdate, updateBannerOpen } from "$lib/store";
  import { saveSettings, getChangelog } from "$lib/api";
  import type { ReleaseNote } from "$lib/types";
  import {
    installUpdate,
    shouldShowUpdate,
    canSelfUpdate,
    RELEASES_URL,
  } from "$lib/updater";
  import { runUpdateCheck } from "$lib/update-check";
  import { openExternal } from "$lib/external";

  // On platforms without self-update (macOS), the primary action opens the
  // release page for a manual download instead of installing in place.
  const selfUpdate = canSelfUpdate();

  // The available update lives in a shared store so the always-visible header
  // indicator and this banner reflect the single startup check.
  const available = $derived($availableUpdate);
  // Cumulative changelog from the running version up to the latest, newest first.
  let changelog = $state<ReleaseNote[]>([]);
  let showNotes = $state(false);
  let installing = $state(false);
  // Session-scoped "remind me later": hides the banner until the app restarts.
  // Component state on the root layout persists across route navigation but
  // resets when the webview reloads on the next launch, so the banner returns.
  let remindedLater = $state(false);
  // Download fraction 0..1, or null when the total size is unknown / not started.
  let progress = $state<number | null>(null);
  let failed = $state<string | null>(null);

  // The banner shows either on its own (an update is available, not dismissed
  // this session, not skipped) or when the header indicator forces it open,
  // which overrides both "remind later" and a persisted "skip".
  const visible = $derived(
    !installing &&
      available != null &&
      ($updateBannerOpen ||
        (shouldShowUpdate(available, $settings.dismissed_update_version) &&
          !remindedLater)),
  );

  // Split a release body into renderable lines; a leading "- "/"* " marks a
  // bullet. Rendered as text (never innerHTML) so notes cannot inject markup.
  function toLines(body: string): { bullet: boolean; text: string }[] {
    return body
      .split(/\r?\n/)
      .map((raw) => {
        const line = raw.trimEnd();
        const m = /^\s*[-*]\s+(.*)$/.exec(line);
        return m
          ? { bullet: true, text: m[1] }
          : { bullet: false, text: line.trim() };
      })
      .filter((l) => l.text.length > 0);
  }

  // Versions to show under "what's new": the fetched changelog, or a single
  // entry from the updater's own notes when the changelog could not be fetched.
  const entries = $derived.by(
    (): { version: string; lines: ReturnType<typeof toLines> }[] => {
      if (changelog.length > 0) {
        return changelog.map((r) => ({
          version: r.version,
          lines: toLines(r.body),
        }));
      }
      if (available?.notes) {
        return [
          { version: available.version, lines: toLines(available.notes) },
        ];
      }
      return [];
    },
  );

  onMount(async () => {
    await runUpdateCheck();
    if (get(availableUpdate)) {
      try {
        changelog = await getChangelog();
      } catch (e) {
        console.error("changelog fetch failed:", e);
      }
    }
  });

  // Hide until the next launch (this session only, not persisted). Also clears
  // the forced-open flag so a header re-open does not keep it visible.
  function remindLater(): void {
    remindedLater = true;
    updateBannerOpen.set(false);
  }

  // Never show this version again (persisted in settings); also clears the
  // forced-open flag so dismissing from a header re-open takes effect.
  function skipVersion(): void {
    if (!available) return;
    settings.update((s) => ({
      ...s,
      dismissed_update_version: available.version,
    }));
    void saveSettings(get(settings));
    updateBannerOpen.set(false);
  }

  async function install(): Promise<void> {
    if (!available) return;
    installing = true;
    failed = null;
    progress = null;
    try {
      await installUpdate((downloaded, total) => {
        progress = total ? downloaded / total : null;
      });
      // On success the app relaunches into the new version; nothing more to do.
    } catch (e) {
      failed = e instanceof Error ? e.message : String(e);
      installing = false;
    }
  }

  const percent = $derived(
    progress == null ? null : Math.round(progress * 100),
  );
</script>

{#if visible || installing}
  <div class="update-banner" role="status" aria-live="polite">
    <div class="update-row">
      <span class="update-msg">
        {$t("update.available")}
        <strong>{available?.version}</strong>
      </span>
      <div class="update-actions">
        {#if entries.length > 0}
          <button
            type="button"
            class="update-link"
            aria-expanded={showNotes}
            onclick={() => (showNotes = !showNotes)}
          >
            {showNotes ? $t("update.hideNotes") : $t("update.whatsNew")}
          </button>
        {/if}
        {#if selfUpdate}
          <button
            type="button"
            class="update-install"
            onclick={install}
            disabled={installing}
            aria-busy={installing}
          >
            {#if installing}
              <span class="spinner" aria-hidden="true"></span>
              {percent == null
                ? $t("update.installing")
                : `${$t("update.installing")} ${percent}%`}
            {:else}
              {$t("update.install")}
            {/if}
          </button>
        {:else}
          <button
            type="button"
            class="update-install"
            onclick={() => openExternal(RELEASES_URL)}
          >
            {$t("update.openReleasePage")}
          </button>
        {/if}
        <button
          type="button"
          class="update-later"
          onclick={remindLater}
          disabled={installing}
        >
          {$t("update.remindLater")}
        </button>
        <button
          type="button"
          class="update-skip"
          onclick={skipVersion}
          disabled={installing}
        >
          {$t("update.skip")}
        </button>
      </div>
    </div>

    {#if showNotes && entries.length > 0}
      <div class="update-notes">
        {#each entries as entry (entry.version)}
          <div class="update-version">
            <h4>{entry.version}</h4>
            <ul>
              {#each entry.lines as line, i (i)}
                <li class:para={!line.bullet}>{line.text}</li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    {/if}

    {#if failed}
      <div class="update-error" role="alert">
        {$t("update.failed")}: {failed}
      </div>
    {/if}
  </div>
{/if}
