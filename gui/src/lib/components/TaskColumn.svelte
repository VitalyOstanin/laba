<script lang="ts">
  import { filterTasks, filterText } from "../store";
  import { t } from "../i18n";
  import { addComment } from "../api";
  import { refreshServer } from "../poller";
  import { onVisible } from "../scroll";
  import FilterRow from "./FilterRow.svelte";
  import type { Task, ServerInfo } from "../types";

  let {
    tasks = [],
    server,
    hasMore = false,
    onLoadMore = () => {},
  }: {
    tasks?: Task[];
    server?: ServerInfo;
    hasMore?: boolean;
    onLoadMore?: () => void;
  } = $props();
  const shown = $derived(filterTasks(tasks, $filterText));

  // Windowed rendering: reveal rows a page at a time, then fetch the next
  // backend page once the resident list is exhausted.
  const PAGE = 50;
  let limit = $state(PAGE);
  const visible = $derived(shown.slice(0, limit));
  const canReveal = $derived(limit < shown.length);

  function loadMore(): void {
    if (canReveal) limit = Math.min(limit + PAGE, shown.length);
    else if (hasMore) onLoadMore();
  }

  // Commenting exists only on OpenProject backends.
  const canComment = $derived(server?.backend === "openproject");
  let openId = $state<number | null>(null);
  let text = $state("");
  let busy = $state(false);

  function startComment(id: number): void {
    openId = openId === id ? null : id;
    text = "";
  }

  async function submit(id: number): Promise<void> {
    if (!server || busy || !text.trim()) return;
    busy = true;
    try {
      await addComment(server.name, id, text.trim());
      await refreshServer(server.name);
      openId = null;
      text = "";
    } finally {
      busy = false;
    }
  }
</script>

<section class="card" aria-label={$t("col.tasks")}>
  <header><h2>{$t("col.tasks")}</h2></header>
  <FilterRow count={shown.length} />
  {#if shown.length === 0}
    <p class="empty">{$t("empty.tasks")}</p>
  {:else}
    <ul class="list">
      {#each visible as task (task.id)}
        <li class="task">
          <span class="id">{task.id}</span>
          <span class="subject">{task.subject}</span>
          <span class="status">{task.status}</span>
          {#if canComment}
            <button
              type="button"
              class="linkbtn"
              onclick={() => startComment(Number(task.id))}
              >{$t("task.comment")}</button
            >
          {/if}
        </li>
        {#if canComment && openId === Number(task.id)}
          <li class="compose">
            <textarea
              bind:value={text}
              rows="2"
              aria-label={$t("task.comment")}
              placeholder={$t("task.commentPlaceholder")}
            ></textarea>
            <div class="compose-actions">
              <button
                type="button"
                class="btn"
                disabled={busy || !text.trim()}
                onclick={() => submit(Number(task.id))}
                >{$t("task.send")}</button
              >
              <button
                type="button"
                class="linkbtn"
                onclick={() => (openId = null)}>{$t("task.cancel")}</button
              >
            </div>
          </li>
        {/if}
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
