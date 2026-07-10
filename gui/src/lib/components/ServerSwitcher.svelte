<script lang="ts">
  import { servers, activeServer, summaries } from "../store";
  import { t } from "../i18n";
</script>

<div class="server-switch" role="group" aria-label={$t("server.switcher")}>
  {#each $servers as s (s.name)}
    <button
      type="button"
      class:off={!s.enabled}
      aria-current={$activeServer === s.name}
      disabled={!s.enabled}
      title={s.enabled
        ? `${s.name} · ${s.base_url}`
        : `${s.name} · ${$t("server.disabled")}`}
      onclick={() => activeServer.set(s.name)}
    >
      {#if s.enabled}
        <span class="dot" class:err={$summaries[s.name]?.error}></span>
      {/if}
      {s.name}
      <span class="bk {s.backend === 'github' ? 'gh' : 'op'}">
        {s.backend === "github" ? "GitHub" : "OpenProject"}
      </span>
      {#if !s.enabled}
        <span class="off-tag">{$t("server.off")}</span>
      {/if}
    </button>
  {/each}
</div>
