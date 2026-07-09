<script lang="ts">
  import { t } from "../i18n";
  import { unreadOf } from "../store";
  import { setNotificationRead, markAllRead } from "../api";
  import { refreshServer } from "../poller";
  import { onVisible } from "../scroll";
  import type { Notification, ServerInfo } from "../types";

  let {
    notifications = [],
    server,
    hasMore = false,
    onLoadMore = () => {},
  }: {
    notifications?: Notification[];
    server?: ServerInfo;
    hasMore?: boolean;
    onLoadMore?: () => void;
  } = $props();

  // Windowed rendering: reveal a page at a time, then fetch the next backend
  // page once the resident list is exhausted.
  const PAGE = 50;
  let limit = $state(PAGE);
  const visible = $derived(notifications.slice(0, limit));
  const canReveal = $derived(limit < notifications.length);

  function loadMore(): void {
    if (canReveal) limit = Math.min(limit + PAGE, notifications.length);
    else if (hasMore) onLoadMore();
  }

  // Read/unread actions exist only on OpenProject backends.
  const canToggle = $derived(server?.backend === "openproject");
  let busy = $state(false);

  async function toggle(n: Notification): Promise<void> {
    if (!server || busy) return;
    busy = true;
    try {
      await setNotificationRead(server.name, Number(n.id), unreadOf(n));
      await refreshServer(server.name);
    } finally {
      busy = false;
    }
  }

  async function markAll(): Promise<void> {
    if (!server || busy) return;
    busy = true;
    try {
      await markAllRead(server.name);
      await refreshServer(server.name);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card" aria-label={$t("col.notifications")}>
  <header>
    <h2>{$t("col.notifications")}</h2>
    {#if canToggle}
      <button type="button" class="linkbtn" disabled={busy} onclick={markAll}
        >{$t("notif.markAll")}</button
      >
    {/if}
  </header>
  {#if notifications.length === 0}
    <p class="empty">{$t("empty.notifications")}</p>
  {:else}
    <ul class="list">
      {#each visible as n (n.id)}
        <li class="notif" class:unread={unreadOf(n)}>
          <span class="reason">{n.reason}</span>
          <span class="subject">{n.wpTitle ?? n.subject}</span>
          {#if canToggle}
            <button
              type="button"
              class="linkbtn"
              disabled={busy}
              title={unreadOf(n)
                ? $t("notif.markRead")
                : $t("notif.markUnread")}
              onclick={() => toggle(n)}
            >
              {unreadOf(n) ? $t("notif.read") : $t("notif.unread")}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if canReveal || hasMore}
      <div class="sentinel" use:onVisible={loadMore}></div>
      <button type="button" class="linkbtn more" onclick={loadMore}>
        {$t("list.loadMore")}
      </button>
    {/if}
  {/if}
</section>
