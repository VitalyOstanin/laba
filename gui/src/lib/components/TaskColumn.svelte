<script lang="ts">
  import { filterTasks, filterText } from "../store";
  import { t } from "../i18n";
  import { addComment } from "../api";
  import { refreshServer } from "../poller";
  import { onVisible } from "../scroll";
  import { openExternal } from "../external";
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
  function statusOf(task: Task): string {
    return task.status == null ? "" : String(task.status);
  }

  // Status-filter tabs: an "All" tab plus either the server's configured filters
  // or, when none are configured, one auto tab per distinct status present (in
  // first-seen order). Only shown for backends with a rich workflow status.
  type Tab = { label: string; statuses: string[] | null };
  const tabs = $derived.by((): Tab[] => {
    const all: Tab = { label: $t("tabs.all"), statuses: null };
    if (!(server?.supports_status_filters ?? false)) return [all];
    const configured = server?.status_filters ?? [];
    let groups: Tab[];
    if (configured.length > 0) {
      groups = configured.map((f) => ({
        label: f.label,
        statuses: f.statuses,
      }));
    } else {
      const seen: string[] = [];
      for (const task of tasks) {
        const s = statusOf(task);
        if (s && !seen.includes(s)) seen.push(s);
      }
      groups = seen.map((s) => ({ label: s, statuses: [s] }));
    }
    return [all, ...groups];
  });

  let activeTab = $state(0);
  // Keep the selection in range when the tab set changes (server switch, etc.).
  $effect(() => {
    if (activeTab >= tabs.length) activeTab = 0;
  });

  function matchesTab(task: Task, tab: Tab): boolean {
    return tab.statuses == null || tab.statuses.includes(statusOf(task));
  }
  // Counts are over all loaded tasks (not the text filter), so a tab always
  // shows how many tasks are in that status overall.
  function tabCount(tab: Tab): number {
    return tab.statuses == null
      ? tasks.length
      : tasks.filter((task) => matchesTab(task, tab)).length;
  }
  function selectTab(i: number): void {
    activeTab = i;
    limit = PAGE;
  }

  const shown = $derived(
    filterTasks(
      tasks.filter((task) => matchesTab(task, tabs[activeTab] ?? tabs[0])),
      $filterText,
    ),
  );

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

  // Semantic row tint for a task, looked up by its exact status string in the
  // server's per-status color map. Unmapped statuses render neutral.
  function tone(task: Task): string {
    const status = task.status == null ? "" : String(task.status);
    const token = server?.status_colors?.[status];
    return token ? `tone-${token}` : "";
  }

  // Browser URL for a task: GitHub carries an explicit `url`; OpenProject work
  // packages are addressed as `<base_url>/work_packages/<id>`. Null when neither
  // is available (no server or no id), so the number stays plain text.
  function taskHref(task: Task): string | null {
    if (server?.backend === "github") {
      return typeof task.url === "string" ? task.url : null;
    }
    const base = server?.base_url;
    if (!base || task.id == null) return null;
    return `${base.replace(/\/+$/, "")}/work_packages/${task.id}`;
  }
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
  {#if tabs.length > 1}
    <nav class="tabs" aria-label={$t("tabs.aria")}>
      {#each tabs as tab, i (tab.label + i)}
        <button
          type="button"
          class="tab"
          aria-current={activeTab === i}
          onclick={() => selectTab(i)}
        >
          {tab.label}
          <span class="tab-count">{tabCount(tab)}</span>
        </button>
      {/each}
    </nav>
  {/if}
  <FilterRow count={shown.length} />
  {#if shown.length === 0}
    <p class="empty">{$t("empty.tasks")}</p>
  {:else}
    <ul class="list">
      {#each visible as task (task.id)}
        <li class="task {tone(task)}">
          {#if taskHref(task)}
            <button
              type="button"
              class="id id-link"
              title={taskHref(task)}
              onclick={() => openExternal(taskHref(task) ?? "")}
              >{task.id}</button
            >
          {:else}
            <span class="id">{task.id}</span>
          {/if}
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
