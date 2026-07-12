<script lang="ts">
  import { get } from "svelte/store";
  import { settings } from "$lib/store";
  import { saveSettings } from "$lib/api";
  import { openExternal } from "$lib/external";
  import { t } from "$lib/i18n";

  // Backends the app can already talk to. Kept in sync with the `backend` union
  // in types.ts / the core `backend` module; shown so the user sees at a glance
  // what is ready before adding a server.
  const READY_BACKENDS = ["OpenProject", "GitHub"];

  // "Request a backend" opens a prefilled GitHub issue. The issue text is the
  // author-facing English project language; only the UI labels are localized.
  const ISSUE_URL = (() => {
    const base = "https://github.com/VitalyOstanin/laba/issues/new";
    const title = "Backend request: <tracker>";
    const body = [
      "Which tracker (Jira / YouTrack / GitLab / Redmine / other):",
      "Instance type (cloud / self-hosted):",
      "Auth method (token / OAuth / other):",
      "Anything else that would help:",
    ].join("\n");
    return `${base}?title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
  })();

  const visible = $derived(!$settings.backends_hint_dismissed);

  async function dismiss(): Promise<void> {
    settings.update((s) => ({ ...s, backends_hint_dismissed: true }));
    try {
      await saveSettings(get(settings));
    } catch (e) {
      console.error("save settings (dismiss backends hint) failed:", e);
    }
  }
</script>

{#if visible}
  <div class="backends-banner" role="note" aria-label={$t("backends.title")}>
    <div class="backends-text">
      <strong>{$t("backends.title")}</strong>
      <span
        >{$t("backends.ready")}: {READY_BACKENDS.join(" · ")} · {$t(
          "backends.more",
        )}</span
      >
    </div>
    <div class="backends-actions">
      <a class="backends-add" href="/settings">{$t("backends.add")}</a>
      <button
        type="button"
        class="backends-request"
        onclick={() => openExternal(ISSUE_URL)}
      >
        {$t("backends.request")}
      </button>
      <button
        type="button"
        class="backends-dismiss"
        onclick={dismiss}
        aria-label={$t("backends.dismiss")}
        title={$t("backends.dismiss")}>×</button
      >
    </div>
  </div>
{/if}
