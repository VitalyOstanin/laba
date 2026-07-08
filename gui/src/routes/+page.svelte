<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { activeServer, byServer } from "$lib/store";
  import { startPolling, stopPolling } from "$lib/poller";
  import ServerSwitcher from "$lib/components/ServerSwitcher.svelte";
  import TaskColumn from "$lib/components/TaskColumn.svelte";
  import NotificationColumn from "$lib/components/NotificationColumn.svelte";
  import StatusBanner from "$lib/components/StatusBanner.svelte";

  onMount(startPolling);
  onDestroy(stopPolling);

  const state = $derived($activeServer ? $byServer[$activeServer] : undefined);
</script>

<header class="topbar"><ServerSwitcher /></header>
<StatusBanner error={state?.error ?? null} />
<main class="cols">
  <NotificationColumn notifications={state?.notifications ?? []} />
  <TaskColumn tasks={state?.tasks ?? []} />
</main>
