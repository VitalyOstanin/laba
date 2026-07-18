<script lang="ts">
  import { servers, activeServer, summaries } from "../store";
  import { refreshServer } from "../poller";
  import { t, locale, plural } from "../i18n";

  // Unread count retained per server (even for non-active servers), so the
  // switcher doubles as an at-a-glance cross-server summary.
  function unreadOf(name: string): number {
    return $summaries[name]?.unread ?? 0;
  }
  function unreadTitle(n: number): string {
    return `${n} ${plural($locale, n, $t, "server.unreadCount")}`;
  }

  // Per-server in-flight flag so the refresh icon shows a spinner while its
  // server resyncs (the styleguide's single animation), and cannot be re-fired.
  let refreshing = $state<Record<string, boolean>>({});

  async function doRefresh(name: string): Promise<void> {
    if (refreshing[name]) return;
    refreshing[name] = true;
    try {
      await refreshServer(name);
    } finally {
      refreshing[name] = false;
    }
  }
</script>

<div class="server-switch" role="group" aria-label={$t("server.switcher")}>
  {#each $servers as s (s.name)}
    <div
      class="server-item"
      class:off={!s.enabled}
      class:active={$activeServer === s.name}
    >
      <button
        type="button"
        class="server-pick"
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
      {#if s.enabled}
        <button
          type="button"
          class="server-refresh"
          title={$t("server.refresh")}
          aria-label={`${$t("server.refresh")}: ${s.display_name}`}
          aria-busy={refreshing[s.name] ?? false}
          disabled={refreshing[s.name] ?? false}
          onclick={() => doRefresh(s.name)}
        >
          {#if refreshing[s.name]}
            <span class="spinner" aria-hidden="true"></span>
          {:else}
            <span class="refresh-glyph" aria-hidden="true">↻</span>
          {/if}
        </button>
      {/if}
    </div>
  {/each}
</div>
