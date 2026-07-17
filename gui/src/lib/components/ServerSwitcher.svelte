<script lang="ts">
  import { servers, activeServer, summaries } from "../store";
  import { t, locale, plural } from "../i18n";

  // Unread count retained per server (even for non-active servers), so the
  // switcher doubles as an at-a-glance cross-server summary.
  function unreadOf(name: string): number {
    return $summaries[name]?.unread ?? 0;
  }
  function unreadTitle(n: number): string {
    return `${n} ${plural($locale, n, $t, "server.unreadCount")}`;
  }
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
      {s.display_name}
      <span class="bk {s.backend === 'github' ? 'gh' : 'op'}">
        {s.backend === "github" ? "GitHub" : "OpenProject"}
      </span>
      {#if !s.enabled}
        <span class="off-tag">{$t("server.off")}</span>
      {:else if unreadOf(s.name) > 0}
        <span class="unread-badge" title={unreadTitle(unreadOf(s.name))}
          >{unreadOf(s.name)}</span
        >
      {/if}
    </button>
  {/each}
</div>
