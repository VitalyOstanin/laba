<script lang="ts">
  import { goto } from "$app/navigation";
  import { filterTasks, filterText } from "../store";
  import { t } from "../i18n";
  import { onVisible } from "../scroll";
  import { openExternal } from "../external";
  import FilterRow from "./FilterRow.svelte";
  import type { Task, ServerInfo, CustomField } from "../types";

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
  // A tab is tinted with the color token of the statuses it selects (the first
  // status' token; grouped statuses in a filter share a tone by convention).
  // The "All" tab (statuses null) stays neutral, and an empty tab (count 0)
  // stays neutral too — there is nothing to draw attention to.
  function tabTone(tab: Tab): string {
    const first = tab.statuses?.[0];
    const token = first ? server?.status_colors?.[first] : undefined;
    if (!token || tabCount(tab) === 0) return "";
    return `tone-${token}`;
  }

  // Extra columns: the server's configured display fields (custom-field names,
  // e.g. Rank), matched against each task's expanded customFields.
  const displayFields = $derived(server?.display_fields ?? []);

  // Sorting of the resident list: by last change (default), task number, or one
  // of the display fields (e.g. Rank). Sorting applies to the loaded tasks; more
  // load in as the user scrolls.
  type SortKey = "updated" | "number" | `field:${string}`;
  let sortKey = $state<SortKey>("updated");
  const sortOptions = $derived<{ key: SortKey; label: string }[]>([
    { key: "updated", label: $t("sort.updated") },
    { key: "number", label: $t("sort.number") },
    ...displayFields.map((f) => ({ key: `field:${f}` as SortKey, label: f })),
  ]);
  function setSort(key: SortKey): void {
    sortKey = key;
    limit = PAGE;
  }
  function num(v: unknown): number | null {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  }
  function sortTasks(list: Task[]): Task[] {
    const s = [...list];
    if (sortKey === "updated") {
      // Most recently changed first.
      s.sort((a, b) =>
        String(b.updatedAt ?? "").localeCompare(String(a.updatedAt ?? "")),
      );
    } else if (sortKey === "number") {
      s.sort((a, b) => (num(a.id) ?? 0) - (num(b.id) ?? 0));
    } else {
      const name = sortKey.slice("field:".length);
      s.sort((a, b) => {
        const va = fieldValue(a, name);
        const vb = fieldValue(b, name);
        const na = num(va);
        const nb = num(vb);
        if (na != null && nb != null) return na - nb;
        return String(va ?? "").localeCompare(String(vb ?? ""));
      });
    }
    return s;
  }

  const shown = $derived(
    sortTasks(
      filterTasks(
        tasks.filter((task) => matchesTab(task, tabs[activeTab] ?? tabs[0])),
        $filterText,
      ),
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

  function customFields(task: Task): CustomField[] {
    const cf = task.customFields;
    return Array.isArray(cf) ? (cf as CustomField[]) : [];
  }
  // Raw value of a display field on a task (by field name), or undefined.
  function fieldValue(task: Task, name: string): unknown {
    const hit = customFields(task).find((c) => c.name === name);
    return hit?.value;
  }
  // Display value of a field: scalars as text, arrays joined, empty as "—".
  function fieldText(task: Task, name: string): string {
    const v = fieldValue(task, name);
    if (v == null || v === "") return "—";
    return Array.isArray(v) ? v.join(", ") : String(v);
  }

  // Opening the detail screen (description + comments) is a backend capability.
  const canDetail = $derived(server?.supports_task_detail ?? false);
  function openDetail(task: Task): void {
    if (!server || task.id == null) return;
    goto(
      `/task?server=${encodeURIComponent(server.name)}&id=${encodeURIComponent(String(task.id))}`,
    );
  }

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
</script>

<section class="card" aria-label={$t("col.tasks")}>
  <header><h2>{$t("col.tasks")}</h2></header>
  {#if tabs.length > 1}
    <nav class="tabs" aria-label={$t("tabs.aria")}>
      {#each tabs as tab, i (tab.label + i)}
        <button
          type="button"
          class="tab {tabTone(tab)}"
          aria-current={activeTab === i}
          onclick={() => selectTab(i)}
        >
          {tab.label}
          <span class="tab-count">{tabCount(tab)}</span>
        </button>
      {/each}
    </nav>
  {/if}
  <div class="sortbar">
    <span class="sort-label">{$t("sort.by")}</span>
    <span class="seg">
      {#each sortOptions as opt (opt.key)}
        <button
          type="button"
          aria-pressed={sortKey === opt.key}
          onclick={() => setSort(opt.key)}>{opt.label}</button
        >
      {/each}
    </span>
  </div>
  <FilterRow count={shown.length} />
  {#if shown.length === 0}
    <p class="empty">{$t("empty.tasks")}</p>
  {:else}
    <ul class="list">
      {#each visible as task (task.id)}
        <li class="task {tone(task)}" class:clickable={canDetail}>
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
          {#if canDetail}
            <button
              type="button"
              class="subject subject-btn"
              onclick={() => openDetail(task)}
              title={$t("task.openDetail")}>{task.subject}</button
            >
          {:else}
            <span class="subject">{task.subject}</span>
          {/if}
          {#each displayFields as f (f)}
            <span class="field" title={f}>{fieldText(task, f)}</span>
          {/each}
          <span class="status">{task.status}</span>
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
