<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { servers } from "$lib/store";
  import { getTaskDetail, listTaskComments, addComment } from "$lib/api";
  import { openExternal } from "$lib/external";
  import type { Task, Notification, ServerInfo, CustomField } from "$lib/types";

  // Query params: which server and work package to show.
  const serverName = $derived(page.url.searchParams.get("server") ?? "");
  const id = $derived(Number(page.url.searchParams.get("id") ?? "0"));
  const server = $derived<ServerInfo | undefined>(
    $servers.find((s) => s.name === serverName),
  );

  let detail = $state<Task | null>(null);
  let comments = $state<Notification[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Load the work package and its comments whenever the target changes.
  $effect(() => {
    const s = serverName;
    const wp = id;
    if (!s || !wp) return;
    loading = true;
    error = null;
    detail = null;
    comments = [];
    Promise.all([getTaskDetail(s, wp), listTaskComments(s, wp)])
      .then(([d, c]) => {
        detail = d;
        comments = Array.isArray(c) ? c : [];
      })
      .catch((e) => (error = String(e)))
      .finally(() => (loading = false));
  });

  function back(): void {
    if (history.length > 1) history.back();
    else goto("/");
  }

  function str(v: unknown): string {
    return v == null ? "" : String(v);
  }
  const paragraphs = $derived(
    str(detail?.description)
      .split(/\n{2,}/)
      .map((p) => p.trim())
      .filter(Boolean),
  );

  function customFields(task: Task | null): CustomField[] {
    const cf = task?.customFields;
    return Array.isArray(cf) ? (cf as CustomField[]) : [];
  }
  // Only the fields the server is configured to surface, in that order.
  const fields = $derived(
    (server?.display_fields ?? [])
      .map((name) => {
        const hit = customFields(detail).find((c) => c.name === name);
        const v = hit?.value;
        const text =
          v == null || v === ""
            ? "—"
            : Array.isArray(v)
              ? v.join(", ")
              : String(v);
        return { name, text };
      })
      .filter((f) => f.text !== "—"),
  );

  // Tracker URL for the number link (mirrors the list): GitHub carries `url`,
  // OpenProject is `<base>/work_packages/<id>`.
  const href = $derived.by((): string | null => {
    if (!detail) return null;
    if (server?.backend === "github") {
      return typeof detail.url === "string" ? detail.url : null;
    }
    const base = server?.base_url;
    if (!base || detail.id == null) return null;
    return `${base.replace(/\/+$/, "")}/work_packages/${detail.id}`;
  });

  function tone(status: string): string {
    const token = server?.status_colors?.[status];
    return token ? `tone-${token}` : "";
  }

  // Comment composer, with async feedback (spinner while sending).
  let text = $state("");
  let sending = $state(false);
  async function submit(): Promise<void> {
    if (!server || sending || !text.trim()) return;
    sending = true;
    try {
      await addComment(server.name, id, text.trim());
      const c = await listTaskComments(server.name, id);
      comments = Array.isArray(c) ? c : [];
      text = "";
    } catch (e) {
      error = String(e);
    } finally {
      sending = false;
    }
  }
</script>

<section class="detail-screen">
  <div class="detail-toolbar">
    <button type="button" class="backlink" onclick={back}>
      <span aria-hidden="true">&larr;</span>
      {$t("detail.back")}
    </button>
    <span class="spacer"></span>
    {#if server}
      <span class="muted"
        >{server.display_name} · {server.backend === "github"
          ? "GitHub"
          : "OpenProject"}</span
      >
    {/if}
  </div>

  {#if loading}
    <p class="detail-status">
      <span class="spinner"></span>
      {$t("detail.loading")}
    </p>
  {:else if error}
    <p class="detail-status err">{error}</p>
  {:else if detail}
    <div class="detail-body">
      {#if href}
        <button
          type="button"
          class="num id-link"
          title={href}
          onclick={() => openExternal(href ?? "")}>#{detail.id}</button
        >
      {:else}
        <span class="num">#{detail.id}</span>
      {/if}
      <h1 class="detail-subject">{detail.subject}</h1>

      <div class="metae">
        {#if detail.status}
          <span class="pill status {tone(str(detail.status))}"
            >{detail.status}</span
          >
        {/if}
        {#each fields as f (f.name)}
          <span class="pill"
            ><span class="chip-k">{f.name}</span><span class="chip-v"
              >{f.text}</span
            ></span
          >
        {/each}
        {#if detail.type}
          <span class="pill"
            ><span class="chip-k">{$t("detail.type")}</span><span class="chip-v"
              >{detail.type}</span
            ></span
          >
        {/if}
        {#if detail.assignee}
          <span class="pill"
            ><span class="chip-k">{$t("detail.assignee")}</span><span
              class="chip-v">{detail.assignee}</span
            ></span
          >
        {/if}
      </div>

      <section class="detail-sec">
        <h2>{$t("detail.description")}</h2>
        {#if paragraphs.length === 0}
          <p class="muted">{$t("detail.noDescription")}</p>
        {:else}
          {#each paragraphs as p, i (i)}
            <p class="desc-p">{p}</p>
          {/each}
        {/if}
      </section>

      <section class="detail-sec">
        <h2>{$t("detail.comments")} · {comments.length}</h2>
        {#if comments.length === 0}
          <p class="muted">{$t("detail.noComments")}</p>
        {:else}
          <ul class="cmts">
            {#each comments as c (c.id)}
              <li class="cmt">
                <div class="cmt-h">
                  <span class="who">{c.user ?? ""}</span>
                  <span class="when">{str(c.createdAt).slice(0, 10)}</span>
                </div>
                <div class="cmt-b">{c.comment ?? ""}</div>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="compose">
          <textarea
            bind:value={text}
            rows="3"
            aria-label={$t("task.comment")}
            placeholder={$t("task.commentPlaceholder")}
          ></textarea>
          <div class="compose-actions">
            <button
              type="button"
              class="btn"
              disabled={sending || !text.trim()}
              aria-busy={sending}
              onclick={submit}
            >
              {#if sending}<span class="spinner" aria-hidden="true"></span>{/if}
              {$t("task.send")}</button
            >
          </div>
        </div>
      </section>
    </div>
  {/if}
</section>
