<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { activeServer, byServer, servers } from "$lib/store";
  import {
    startPolling,
    stopPolling,
    loadMoreTasks,
    loadMoreNotifications,
  } from "$lib/poller";
  import ServerSwitcher from "$lib/components/ServerSwitcher.svelte";
  import TaskColumn from "$lib/components/TaskColumn.svelte";
  import NotificationColumn from "$lib/components/NotificationColumn.svelte";
  import StatusBanner from "$lib/components/StatusBanner.svelte";
  import TimelogIndicator from "$lib/components/TimelogIndicator.svelte";
  import { t } from "$lib/i18n";

  onMount(startPolling);
  onDestroy(stopPolling);

  const state = $derived($activeServer ? $byServer[$activeServer] : undefined);
  const activeInfo = $derived($servers.find((s) => s.name === $activeServer));
  // First load of the active server: it is selected but its lists are not yet
  // resident (the poller evicts other servers on switch and fetches the new one).
  // Show a spinner rather than empty columns until the first page arrives.
  const loading = $derived(!!$activeServer && state === undefined);
</script>

<header class="topbar">
  <ServerSwitcher />
  <a class="settings-link" href="/settings" aria-label={$t("nav.settings")}
    >{$t("nav.settings")}</a
  >
</header>
<StatusBanner error={state?.error ?? null} />
<TimelogIndicator />
{#if loading}
  <p class="cols-loading" aria-live="polite">
    <span class="spinner" aria-hidden="true"></span>
    {$t("detail.loading")}
  </p>
{:else}
  <main class="cols">
    {#if activeInfo?.has_notifications ?? true}
      <NotificationColumn
        notifications={state?.notifications ?? []}
        server={activeInfo}
        hasMore={state?.notifCursor != null}
        onLoadMore={() => $activeServer && loadMoreNotifications($activeServer)}
      />
    {/if}
    <TaskColumn
      tasks={state?.tasks ?? []}
      server={activeInfo}
      hasMore={state?.taskCursor != null}
      onLoadMore={() => $activeServer && loadMoreTasks($activeServer)}
    />
  </main>
{/if}
