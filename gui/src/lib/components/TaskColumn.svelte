<script lang="ts">
  import { goto } from "$app/navigation";
  import { filterTasks, filterText, contentOpenPlan } from "../store";
  import { t } from "../i18n";
  import { onVisible } from "../scroll";
  import { openExternal } from "../external";
  import FilterRow from "./FilterRow.svelte";
  import { supportsStatusFilters, supportsTaskDetail } from "../capabilities";
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
    return task.status ?? "";
  }

  // Status-filter tabs: an "All" tab plus either the server's configured filters
  // or, when none are configured, one auto tab per distinct status present (in
  // first-seen order). Only shown for backends with a rich workflow status.
  type Tab = { label: string; statuses: string[] | null };
  const tabs = $derived.by((): Tab[] => {
    const all: Tab = { label: $t("tabs.all"), statuses: null };
    if (!supportsStatusFilters(server)) return [all];
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

  // Scope tabs: "My repos" (tasks in repositories the user owns) vs "Others"
  // (everything else — repos the user only follows / commented on). The two are
  // disjoint. GitHub-only: repository ownership is meaningless for OpenProject,
  // which already lists just the user's own work packages. Default to "mine".
  const showScope = $derived(server?.backend === "github");
  let scope = $state<"mine" | "others">("mine");
  const mineCount = $derived(tasks.filter((t) => t.mine).length);
  const othersCount = $derived(tasks.filter((t) => !t.mine).length);
  const scopedTasks = $derived.by(() => {
    if (!showScope) return tasks;
    return scope === "mine"
      ? tasks.filter((t) => t.mine)
      : tasks.filter((t) => !t.mine);
  });
  function selectScope(s: "mine" | "others"): void {
    scope = s;
    activeTab = 0;
    limit = PAGE;
  }

  function matchesTab(task: Task, tab: Tab): boolean {
    return tab.statuses == null || tab.statuses.includes(statusOf(task));
  }
  // Counts are over the scoped tasks (not the text filter), so a tab always
  // shows how many tasks are in that status within the current scope.
  function tabCount(tab: Tab): number {
    return tab.statuses == null
      ? scopedTasks.length
      : scopedTasks.filter((task) => matchesTab(task, tab)).length;
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
  // load in as the user scrolls. Direction defaults to descending (newest /
  // highest first) and toggles when the active key is clicked again.
  type SortKey = "updated" | "number" | `field:${string}`;
  type SortDir = "asc" | "desc";
  let sortKey = $state<SortKey>("updated");
  let sortDir = $state<SortDir>("desc");
  const sortOptions = $derived<{ key: SortKey; label: string }[]>([
    { key: "updated", label: $t("sort.updated") },
    { key: "number", label: $t("sort.number") },
    ...displayFields.map((f) => ({ key: `field:${f}` as SortKey, label: f })),
  ]);
  // Clicking the active key flips the direction; a different key selects it and
  // resets to the descending default.
  function setSort(key: SortKey): void {
    if (key === sortKey) {
      sortDir = sortDir === "desc" ? "asc" : "desc";
    } else {
      sortKey = key;
      sortDir = "desc";
    }
    limit = PAGE;
  }
  function num(v: unknown): number | null {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  }
  // Ascending comparator for the active key; direction is applied in sortTasks.
  function cmpAsc(a: Task, b: Task): number {
    if (sortKey === "updated") {
      return String(a.updatedAt ?? "").localeCompare(String(b.updatedAt ?? ""));
    }
    if (sortKey === "number") {
      return (num(a.id) ?? 0) - (num(b.id) ?? 0);
    }
    const name = sortKey.slice("field:".length);
    const va = fieldValue(a, name);
    const vb = fieldValue(b, name);
    const na = num(va);
    const nb = num(vb);
    if (na != null && nb != null) return na - nb;
    return String(va ?? "").localeCompare(String(vb ?? ""));
  }
  function sortTasks(list: Task[]): Task[] {
    const mul = sortDir === "asc" ? 1 : -1;
    return [...list].sort((a, b) => mul * cmpAsc(a, b));
  }

  const shown = $derived(
    sortTasks(
      filterTasks(
        scopedTasks.filter((task) =>
          matchesTab(task, tabs[activeTab] ?? tabs[0]),
        ),
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
    return task.customFields;
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
  const canDetail = $derived(supportsTaskDetail(server));
  function openDetail(task: Task): void {
    if (!server || !task.id.raw) return;
    goto(
      `/task?server=${encodeURIComponent(server.name)}&id=${encodeURIComponent(task.id.raw)}`,
    );
  }

  // Where a task opens on click: the server's effective preference (`app` opens
  // the in-laba detail screen, `browser` the web URL). See `contentOpenPlan`.
  const openTarget = $derived(server?.open_content_in ?? "browser");
  function plan(task: Task) {
    return contentOpenPlan({
      openTarget,
      canDetail,
      hasHref: taskHref(task) != null,
    });
  }
  function openInBrowser(task: Task): void {
    const href = taskHref(task);
    if (href) openExternal(href);
  }
  function openPrimary(task: Task): void {
    if (plan(task).primary === "app") openDetail(task);
    else openInBrowser(task);
  }

  // Semantic row tint for a task, looked up by its exact status string in the
  // server's per-status color map. Unmapped statuses render neutral.
  function tone(task: Task): string {
    const status = task.status ?? "";
    const token = server?.status_colors?.[status];
    return token ? `tone-${token}` : "";
  }

  // Browser URL for a task: GitHub carries an explicit `url`; OpenProject work
  // packages are addressed as `<base_url>/work_packages/<id>`. Null when neither
  // is available (no server or no id), so the number stays plain text.
  function taskHref(task: Task): string | null {
    if (server?.backend === "github") {
      return task.url ?? null;
    }
    const base = server?.base_url;
    if (!base || !task.id.raw) return null;
    return `${base.replace(/\/+$/, "")}/work_packages/${task.id.raw}`;
  }
</script>

<section class="card" aria-label={$t("col.tasks")}>
  <header><h2>{$t("col.tasks")}</h2></header>
  {#if showScope}
    <nav class="tabs scope-tabs" aria-label={$t("scope.aria")}>
      <button
        type="button"
        class="tab"
        aria-current={scope === "mine"}
        onclick={() => selectScope("mine")}
      >
        {$t("scope.mine")}
        <span class="tab-count">{mineCount}</span>
      </button>
      <button
        type="button"
        class="tab"
        aria-current={scope === "others"}
        onclick={() => selectScope("others")}
      >
        {$t("scope.others")}
        <span class="tab-count">{othersCount}</span>
      </button>
    </nav>
  {/if}
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
          title={sortKey === opt.key
            ? $t(sortDir === "desc" ? "sort.dir.desc" : "sort.dir.asc")
            : opt.label}
          onclick={() => setSort(opt.key)}
          >{opt.label}{#if sortKey === opt.key}<span
              class="sort-arrow"
              aria-hidden="true">{sortDir === "desc" ? " ↓" : " ↑"}</span
            >{/if}</button
        >
      {/each}
    </span>
  </div>
  <FilterRow count={shown.length} />
  {#if shown.length === 0}
    <p class="empty">{$t("empty.tasks")}</p>
  {:else}
    <ul class="list">
      {#each visible as task (task.id.display)}
        <li class="task {tone(task)}" class:clickable={canDetail}>
          {#if taskHref(task)}
            <button
              type="button"
              class="id id-link"
              title={taskHref(task)}
              onclick={() => openExternal(taskHref(task) ?? "")}
              >{task.id.display}</button
            >
          {:else}
            <span class="id">{task.id.display}</span>
          {/if}
          {#if plan(task).primary !== "none"}
            <button
              type="button"
              class="subject subject-btn"
              onclick={() => openPrimary(task)}
              title={plan(task).primary === "app"
                ? $t("task.openDetail")
                : (taskHref(task) ?? "")}>{task.title}</button
            >
          {:else}
            <span class="subject">{task.title}</span>
          {/if}
          {#if plan(task).secondaryBrowser}
            <button
              type="button"
              class="openbtn"
              onclick={() => openInBrowser(task)}
              title={$t("content.openInBrowser")}
              aria-label={$t("content.openInBrowser")}>↗</button
            >
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
