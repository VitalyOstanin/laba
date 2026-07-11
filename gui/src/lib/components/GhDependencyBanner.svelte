<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n";
  import { ghDependency } from "$lib/api";
  import { openExternal } from "$lib/external";
  import type { GhDependency } from "$lib/types";

  // gh is only needed by the GitHub task backend; the update checker uses the
  // public API anonymously. So this hint appears only when a GitHub server is
  // configured and gh is missing or not signed in.
  let dep = $state<GhDependency | null>(null);
  let dismissed = $state(false);
  let checking = $state(false);

  const visible = $derived(
    !!dep && dep.used && dep.status !== "ready" && !dismissed,
  );

  async function recheck(): Promise<void> {
    checking = true;
    try {
      dep = await ghDependency();
    } catch (e) {
      console.error("gh dependency check failed:", e);
    } finally {
      checking = false;
    }
  }

  onMount(recheck);
</script>

{#if visible && dep}
  <div class="gh-banner" role="status" aria-live="polite">
    <div class="gh-text">
      <strong>
        {dep.status === "missing"
          ? $t("gh.missing.title")
          : $t("gh.unauth.title")}
      </strong>
      <span>
        {dep.status === "missing"
          ? $t("gh.missing.body")
          : $t("gh.unauth.body")}
      </span>
    </div>
    <div class="gh-actions">
      {#if dep.status === "missing"}
        <button
          type="button"
          class="gh-install"
          onclick={() =>
            openExternal("https://github.com/cli/cli#installation")}
        >
          {$t("gh.install")}
        </button>
      {/if}
      <button
        type="button"
        class="gh-recheck"
        onclick={recheck}
        disabled={checking}
        aria-busy={checking}
      >
        {#if checking}
          <span class="spinner" aria-hidden="true"></span>
        {/if}
        {$t("gh.recheck")}
      </button>
      <button type="button" class="gh-later" onclick={() => (dismissed = true)}>
        {$t("gh.later")}
      </button>
    </div>
  </div>
{/if}
