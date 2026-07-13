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
  <a class="settings-link" href="/settings" aria-label={$t("nav.settings")}
    >{$t("nav.settings")}</a
  >
</header>
<StatusBanner error={srvState?.error ?? null} />
<TimelogIndicator />
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
    {#if activeInfo?.has_notifications ?? true}
      <NotificationColumn
        notifications={srvState?.notifications ?? []}
        server={activeInfo}
        hasMore={srvState?.notifCursor != null}
        onLoadMore={() => $activeServer && loadMoreNotifications($activeServer)}
      />
    {/if}
    <TaskColumn
      tasks={srvState?.tasks ?? []}
      server={activeInfo}
      hasMore={srvState?.taskCursor != null}
      onLoadMore={() => $activeServer && loadMoreTasks($activeServer)}
    />
  </main>
{/if}

{#if showWizard}
  <SetupWizard onClose={() => (wizardDismissed = true)} onDone={onWizardDone} />
{/if}
