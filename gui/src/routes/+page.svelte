<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { activeServer, byServer, servers, settings } from "$lib/store";
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
  import SyncBar from "$lib/components/SyncBar.svelte";
  import TimelogIndicator from "$lib/components/TimelogIndicator.svelte";
  import SetupWizard from "$lib/components/SetupWizard.svelte";
  import { t } from "$lib/i18n";

  onMount(startPolling);
  onDestroy(stopPolling);

  const srvState = $derived(
    $activeServer ? $byServer[$activeServer] : undefined,
  );
  const activeInfo = $derived($servers.find((s) => s.name === $activeServer));
  // First load of the active server: it is selected but its lists are not yet
  // resident (the poller evicts other servers on switch and fetches the new one).
  // Show a spinner rather than empty columns until the first page arrives.
  const loading = $derived(!!$activeServer && srvState === undefined);
  // No server profiles configured at all: show an explicit call to action
  // (independent of the dismissible onboarding banner) instead of empty columns.
  const noServers = $derived($servers.length === 0);

  // First-run setup wizard: open automatically once when no server is
  // configured, and reopenable from the empty state. `dismissed` keeps it from
  // reopening after the user closes it without finishing.
  let wizardDismissed = $state(false);
  const showWizard = $derived(noServers && !wizardDismissed);

  // A freshly created profile: restart polling so the new server is picked up
  // (startPolling is not idempotent — stop first to clear timers/listeners).
  function onWizardDone(): void {
    stopPolling();
    void startPolling();
  }
</script>

<header class="topbar">
  <ServerSwitcher />
  <a
    class="settings-link"
    href="/settings"
    aria-label={$t("nav.settings")}
    title={$t("nav.settings")}
  >
    <svg
      class="settings-icon"
      viewBox="0 0 24 24"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3" />
      <path
        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
      />
    </svg>
  </a>
</header>
<SyncBar />
<StatusBanner error={srvState?.error ?? null} />
{#if $settings.show_timelog}
  <TimelogIndicator />
{/if}
{#if noServers}
  <section class="empty-state" aria-label={$t("empty.title")}>
    <strong>{$t("empty.title")}</strong>
    <p>{$t("empty.hint")}</p>
    <button
      type="button"
      class="empty-add"
      onclick={() => (wizardDismissed = false)}>{$t("empty.setup")}</button
    >
  </section>
{:else if loading}
  <p class="cols-loading" aria-live="polite">
    <span class="spinner" aria-hidden="true"></span>
    {$t("detail.loading")}
  </p>
{:else}
  <main class="cols">
    {#if $settings.show_notifications && (activeInfo?.has_notifications ?? true)}
      <NotificationColumn
        notifications={srvState?.notifications ?? []}
        server={activeInfo}
        hasMore={srvState?.notifCursor != null}
        onLoadMore={() => $activeServer && loadMoreNotifications($activeServer)}
      />
    {/if}
    {#if $settings.show_tasks}
      <TaskColumn
        tasks={srvState?.tasks ?? []}
        server={activeInfo}
        hasMore={srvState?.taskCursor != null}
        onLoadMore={() => $activeServer && loadMoreTasks($activeServer)}
      />
    {/if}
  </main>
{/if}

{#if showWizard}
  <SetupWizard onClose={() => (wizardDismissed = true)} onDone={onWizardDone} />
{/if}
