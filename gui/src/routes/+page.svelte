<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { activeServer, byServer, servers } from "$lib/store";
  import { startPolling, stopPolling } from "$lib/poller";
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
</script>

<header class="topbar">
  <ServerSwitcher />
  <a class="settings-link" href="/settings" aria-label={$t("nav.settings")}
    >{$t("nav.settings")}</a
  >
</header>
<StatusBanner error={state?.error ?? null} />
<TimelogIndicator />
<main class="cols">
  <NotificationColumn
    notifications={state?.notifications ?? []}
    server={activeInfo}
  />
  <TaskColumn tasks={state?.tasks ?? []} server={activeInfo} />
</main>
