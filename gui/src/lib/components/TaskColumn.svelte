<script lang="ts">
  import { filterTasks, filterText } from "../store";
  import { t } from "../i18n";
  import FilterRow from "./FilterRow.svelte";
  import type { Task } from "../types";

  let { tasks = [] }: { tasks?: Task[] } = $props();
  const shown = $derived(filterTasks(tasks, $filterText));
</script>

<section class="card" aria-label={$t("col.tasks")}>
  <header><h2>{$t("col.tasks")}</h2></header>
  <FilterRow count={shown.length} />
  {#if shown.length === 0}
    <p class="empty">{$t("empty.tasks")}</p>
  {:else}
    <ul class="list">
      {#each shown as task (task.id)}
        <li class="task">
          <span class="id">{task.id}</span>
          <span class="subject">{task.subject}</span>
          <span class="status">{task.status}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>
