<script lang="ts">
  import { get } from "svelte/store";
  import { t } from "$lib/i18n";
  import { addServer, loginServer, ghProbe, ghAccount } from "$lib/api";
  import { openExternal } from "$lib/external";
  import { friendlyError } from "$lib/friendly-error";
  import type { GhStatus, GhAccount } from "$lib/types";

  // Parent closes the wizard (onClose) and refreshes the dashboard (onDone).
  let { onClose, onDone }: { onClose: () => void; onDone: () => void } =
    $props();

  type Backend = "openproject" | "github";

  const GH_INSTALL = "https://github.com/cli/cli#installation";

  let backend = $state<Backend | null>(null);
  let name = $state("");
  let url = $state("");
  let display = $state("");
  let token = $state("");
  let ghStatus = $state<GhStatus | null>(null);
  let ghChecking = $state(false);
  let ghAcct = $state<GhAccount | null>(null);
  let created = $state(false); // profile already created; a retry only signs in
  let busy = $state(false);
  let error = $state("");
  let stepIdx = $state(0);

  const isGithub = $derived(backend === "github");
  // GitHub skips the OpenProject token step: [backend, connection, verify].
  // OpenProject: [backend, connection, token, verify].
  const steps = $derived<string[]>(
    isGithub
      ? ["backend", "connection", "verify"]
      : ["backend", "connection", "token", "verify"],
  );
  const step = $derived(steps[stepIdx]);

  async function checkGh(): Promise<void> {
    ghChecking = true;
    error = "";
    ghAcct = null;
    try {
      ghStatus = await ghProbe("");
      // When signed in, read which login on which host, so the user confirms
      // who and where before creating the profile. A failure here is not fatal:
      // the status is still "ready"; just omit the account line.
      if (ghStatus === "ready") {
        try {
          ghAcct = await ghAccount("");
        } catch {
          ghAcct = null;
        }
      }
    } catch (e) {
      ghStatus = null;
      error = friendlyError(String(e), get(t)).text;
    } finally {
      ghChecking = false;
    }
  }

  function pickBackend(b: Backend): void {
    backend = b;
    error = "";
    ghStatus = null;
    ghAcct = null;
    if (b === "github") {
      // GitHub has a single host; prefill it so the user need not type it.
      // Only when the field is still empty, to avoid clobbering a typed value.
      if (url.trim() === "") url = "github.com";
      void checkGh();
    }
  }

  const canNext = $derived.by((): boolean => {
    switch (step) {
      case "backend":
        return backend !== null && (!isGithub || ghStatus === "ready");
      case "connection":
        return name.trim() !== "" && url.trim() !== "";
      case "token":
        return token.trim() !== "";
      default:
        return true;
    }
  });

  function next(): void {
    error = "";
    if (stepIdx < steps.length - 1) stepIdx += 1;
  }
  function back(): void {
    error = "";
    if (stepIdx > 0) stepIdx -= 1;
  }

  function openTokenPage(): void {
    const base = url.trim().replace(/\/+$/, "");
    if (base) void openExternal(`${base}/my/access_token`);
  }

  async function finish(): Promise<void> {
    if (backend === null) return;
    busy = true;
    error = "";
    try {
      // Create the profile once; a retry after a bad token only re-signs in.
      if (!created) {
        await addServer(
          name.trim(),
          url.trim(),
          backend,
          display.trim() === "" ? null : display.trim(),
        );
        created = true;
      }
      if (backend === "openproject") {
        await loginServer(name.trim(), token.trim(), false);
      }
      // The profile is created; close the wizard first so a dashboard-refresh
      // failure cannot trap it open, then refresh. A refresh error is not a
      // setup error (the server exists) — swallow it; the next poll retries.
      onClose();
      try {
        onDone();
      } catch {
        // ignore: the dashboard refreshes on the next poll tick
      }
    } catch (e) {
      error = friendlyError(String(e), get(t)).text;
    } finally {
      busy = false;
    }
  }
</script>

<div
  class="wizard-overlay"
  role="dialog"
  aria-modal="true"
  aria-label={$t("wizard.title")}
>
  <div class="wizard">
    <header class="wizard-head">
      <h2>{$t("wizard.title")}</h2>
      <span class="wizard-step">
        {$t("wizard.step")}
        {stepIdx + 1} / {steps.length}
      </span>
    </header>

    <div class="wizard-body">
      {#if step === "backend"}
        <p class="wizard-q">{$t("wizard.backend.title")}</p>
        <div class="wizard-choices">
          <button
            type="button"
            class="wizard-card"
            class:sel={backend === "github"}
            onclick={() => pickBackend("github")}
          >
            <strong>{$t("wizard.backend.gh")}</strong>
            <span>{$t("wizard.backend.ghDesc")}</span>
          </button>
          <button
            type="button"
            class="wizard-card"
            class:sel={backend === "openproject"}
            onclick={() => pickBackend("openproject")}
          >
            <strong>{$t("wizard.backend.op")}</strong>
            <span>{$t("wizard.backend.opDesc")}</span>
          </button>
        </div>

        {#if isGithub}
          <div class="wizard-gh" aria-live="polite">
            <p>{$t("wizard.gh.needed")}</p>
            {#if ghChecking}
              <p><span class="spinner" aria-hidden="true"></span></p>
            {:else if ghStatus === "missing"}
              <p><strong>{$t("gh.missing.title")}</strong></p>
              <p>{$t("gh.missing.body")}</p>
              <div class="wizard-gh-actions">
                <button
                  type="button"
                  class="btn"
                  onclick={() => openExternal(GH_INSTALL)}
                  >{$t("gh.install")}</button
                >
                <button type="button" class="linkbtn" onclick={checkGh}
                  >{$t("gh.recheck")}</button
                >
              </div>
            {:else if ghStatus === "unauthenticated"}
              <p><strong>{$t("gh.unauth.title")}</strong></p>
              <p>{$t("gh.unauth.body")}</p>
              <button type="button" class="linkbtn" onclick={checkGh}
                >{$t("gh.recheck")}</button
              >
            {:else if ghStatus === "ready"}
              <p class="wizard-ok">{$t("wizard.gh.ready")}</p>
              {#if ghAcct}
                <p class="wizard-account">
                  {$t("wizard.gh.as")}
                  <strong>{ghAcct.login}</strong>
                  · {ghAcct.host}
                </p>
              {/if}
            {/if}
            <p class="wizard-hint">{$t("wizard.gh.scopes")}</p>
          </div>
        {/if}
      {:else if step === "connection"}
        <p class="wizard-q">{$t("wizard.connection.title")}</p>
        <label class="wizard-field">
          <span>{$t("wizard.name")}</span>
          <input type="text" bind:value={name} />
        </label>
        <label class="wizard-field">
          <span>URL</span>
          <input
            type="text"
            placeholder={isGithub ? $t("wizard.url.gh") : $t("wizard.url.op")}
            bind:value={url}
          />
        </label>
        <label class="wizard-field">
          <span>{$t("wizard.display")}</span>
          <input type="text" bind:value={display} />
        </label>
      {:else if step === "token"}
        <p class="wizard-q">{$t("wizard.token.title")}</p>
        <label class="wizard-field">
          <span>{$t("wizard.token.title")}</span>
          <input type="password" bind:value={token} />
        </label>
        <button type="button" class="linkbtn" onclick={openTokenPage}
          >{$t("wizard.token.where")}</button
        >
        <p class="wizard-hint">{$t("wizard.token.hint")}</p>
      {:else}
        <p class="wizard-q">{$t("wizard.verify.title")}</p>
        <ul class="wizard-summary">
          <li>
            {isGithub ? $t("wizard.backend.gh") : $t("wizard.backend.op")}
          </li>
          <li>{name.trim()} — {url.trim()}</li>
        </ul>
        <p class="wizard-hint">
          {isGithub ? $t("wizard.verify.gh") : $t("wizard.verify.op")}
        </p>
      {/if}

      {#if error}<p class="wizard-error" role="alert">{error}</p>{/if}
    </div>

    <footer class="wizard-foot">
      <button type="button" class="linkbtn" onclick={onClose}
        >{$t("wizard.cancel")}</button
      >
      <div class="wizard-nav">
        {#if stepIdx > 0}
          <button type="button" class="btn" onclick={back} disabled={busy}
            >{$t("wizard.back")}</button
          >
        {/if}
        {#if step === "verify"}
          <button
            type="button"
            class="btn primary"
            onclick={finish}
            disabled={busy}
            aria-busy={busy}
          >
            {#if busy}<span class="spinner" aria-hidden="true"></span>{/if}
            {busy ? $t("wizard.creating") : $t("wizard.finish")}
          </button>
        {:else}
          <button
            type="button"
            class="btn primary"
            onclick={next}
            disabled={!canNext}>{$t("wizard.next")}</button
          >
        {/if}
      </div>
    </footer>
  </div>
</div>
