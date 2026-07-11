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

  // Read/unread toggling is a backend capability (only some backends expose a
  // per-notification read write).
  const canToggle = $derived(server?.can_toggle_read ?? false);
  // Async feedback (project rule): show which dot is in flight, disable it while
  // the toggle runs. `busyId` is the notification being toggled; `busyAll` marks
  // the mark-all action. Any in-flight action blocks the others.
  let busyId = $state<number | null>(null);
  let busyAll = $state(false);
  const anyBusy = $derived(busyId !== null || busyAll);

  async function toggle(n: Notification): Promise<void> {
    if (!server || anyBusy) return;
    busyId = Number(n.id);
    try {
      await setNotificationRead(server.name, Number(n.id), unreadOf(n));
      await refreshServer(server.name);
    } finally {
      busyId = null;
    }
  }

  async function markAll(): Promise<void> {
    if (!server || anyBusy) return;
    busyAll = true;
    try {
      await markAllRead(server.name);
      await refreshServer(server.name);
    } finally {
      busyAll = false;
    }
  }
</script>

<section class="card" aria-label={$t("col.notifications")}>
  <header>
    <h2>{$t("col.notifications")}</h2>
    {#if canToggle}
      <button
        type="button"
        class="linkbtn"
        class:busy={busyAll}
        disabled={anyBusy}
        aria-busy={busyAll}
        onclick={markAll}
      >
        {#if busyAll}<span class="spinner" aria-hidden="true"></span>{/if}
        {$t("notif.markAll")}</button
      >
    {/if}
  </header>
  {#if notifications.length === 0}
    <p class="empty">{$t("empty.notifications")}</p>
  {:else}
    <ul class="list">
      {#each visible as n (n.id)}
        <li class="notif" class:unread={unreadOf(n)}>
          {#if canToggle}
            <button
              type="button"
              class="readdot"
              class:unread={unreadOf(n)}
              class:busy={busyId === Number(n.id)}
              disabled={anyBusy}
              aria-busy={busyId === Number(n.id)}
              aria-label={unreadOf(n)
                ? $t("notif.markRead")
                : $t("notif.markUnread")}
              title={unreadOf(n)
                ? $t("notif.markRead")
                : $t("notif.markUnread")}
              onclick={() => toggle(n)}
            ></button>
          {:else}
            <span
              class="readdot"
              class:unread={unreadOf(n)}
              aria-hidden="true"
              title={unreadOf(n) ? $t("notif.isUnread") : $t("notif.isRead")}
            ></span>
          {/if}
          <span class="reason">{n.reason}</span>
          <span class="subject">{n.wpTitle ?? n.subject}</span>
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
