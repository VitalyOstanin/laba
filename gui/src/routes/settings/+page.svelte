<script lang="ts">
  import { get } from "svelte/store";
  import { settings, servers, activeServer, parsePollSecs } from "$lib/store";
  import {
    saveSettings,
    listServers,
    setServerDisplayName,
    setServerEnabled,
    setServerPollSecs,
    setServerTimelogStart,
    setServerStatusColor,
    renameServer,
  } from "$lib/api";
  import { applyTheme } from "$lib/theme";
  import {
    applyUiScale,
    clampUiScale,
    DEFAULT_UI_SCALE,
    UI_SCALE_STEP,
  } from "$lib/scale";
  import { language, t } from "$lib/i18n";
  import { fieldKeys } from "$lib/keys";
  import type { Theme, Lang, WeekStart, StatusColorToken } from "$lib/types";

  let saved = $state(false);
  let flash: ReturnType<typeof setTimeout> | undefined;

  async function persist(): Promise<void> {
    const s = get(settings);
    applyTheme(s.theme);
    language.set(s.language);
    await saveSettings(s);
    saved = true;
    clearTimeout(flash);
    flash = setTimeout(() => (saved = false), 1500);
  }

  // Reload the server list after a profile edit (server-level settings live in
  // config.json, not in the app settings store).
  async function refreshServers(): Promise<void> {
    try {
      servers.set(await listServers());
    } catch {
      // Keep the current list if the reload fails.
    }
  }

  function setTheme(v: Theme): void {
    settings.update((s) => ({ ...s, theme: v }));
    void persist();
  }
  function setLang(v: Lang): void {
    settings.update((s) => ({ ...s, language: v }));
    void persist();
  }
  function setWeekStart(v: WeekStart): void {
    settings.update((s) => ({ ...s, week_start: v }));
    void persist();
  }
  function setTray(v: boolean): void {
    settings.update((s) => ({ ...s, minimize_to_tray: v }));
    void persist();
  }
  function setUiScale(scale: number): void {
    const v = clampUiScale(scale);
    settings.update((s) => ({ ...s, ui_scale: v }));
    applyUiScale(v);
    void persist();
  }
  function bumpUiScale(delta: number): void {
    setUiScale(get(settings).ui_scale + delta);
  }
  function setTimezone(raw: string): void {
    // Blank means "follow the system"; store the sentinel the backend expects.
    const tz = raw.trim() === "" ? "system" : raw.trim();
    settings.update((s) => ({ ...s, timezone: tz }));
    void persist();
  }

  // Per-server profile editors (config.json).
  async function setDisplayName(name: string, value: string): Promise<void> {
    await setServerDisplayName(name, value.trim() === "" ? null : value.trim());
    await refreshServers();
  }
  async function renameShort(oldName: string, value: string): Promise<void> {
    const next = value.trim();
    if (next === "" || next === oldName) return;
    await renameServer(oldName, next);
    if (get(activeServer) === oldName) activeServer.set(next);
    await refreshServers();
  }
  async function setEnabled(name: string, enabled: boolean): Promise<void> {
    await setServerEnabled(name, enabled);
    await refreshServers();
  }
  async function setPoll(name: string, raw: string): Promise<void> {
    await setServerPollSecs(name, parsePollSecs(raw) ?? null);
    await refreshServers();
  }
  async function setStart(name: string, date: string): Promise<void> {
    await setServerTimelogStart(name, date === "" ? null : date);
    await refreshServers();
  }

  // Per-server status-color editor. Drafts for the "add" row are keyed by server
  // name so each row keeps its own in-progress status text and color.
  const COLOR_TOKENS: StatusColorToken[] = [
    "danger",
    "warn",
    "success",
    "dimmed",
  ];
  let draftStatus = $state<Record<string, string>>({});
  let draftColor = $state<Record<string, StatusColorToken>>({});

  async function addStatusColor(name: string): Promise<void> {
    const status = (draftStatus[name] ?? "").trim();
    if (status === "") return;
    await setServerStatusColor(name, status, draftColor[name] ?? "danger");
    draftStatus[name] = "";
    await refreshServers();
  }
  async function changeStatusColor(
    name: string,
    status: string,
    color: StatusColorToken,
  ): Promise<void> {
    await setServerStatusColor(name, status, color);
    await refreshServers();
  }
  async function removeStatusColor(
    name: string,
    status: string,
  ): Promise<void> {
    await setServerStatusColor(name, status, null);
    await refreshServers();
  }

  // Interface scale is stored as a factor (1 = 100%); show it as a percentage.
  const scalePercent = (factor: number): number => Math.round(factor * 100);

  const themes: Theme[] = ["system", "dark", "light"];
  const langs: Lang[] = ["system", "en", "ru"];
  const weekStarts: WeekStart[] = ["system", "monday", "sunday"];
</script>

<section class="settings" aria-label={$t("settings.title")}>
  <header class="settings-head">
    <a class="back" href="/">← {$t("nav.dashboard")}</a>
    <h1>{$t("settings.title")}</h1>
    {#if saved}<span class="saved" role="status">{$t("settings.saved")}</span
      >{/if}
  </header>

  <fieldset>
    <legend>{$t("settings.theme")}</legend>
    <div class="choices">
      {#each themes as th (th)}
        <label>
          <input
            type="radio"
            name="theme"
            value={th}
            checked={$settings.theme === th}
            onchange={() => setTheme(th)}
          />
          {$t(`settings.theme.${th}`)}
        </label>
      {/each}
    </div>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.language")}</legend>
    <div class="choices">
      {#each langs as lg (lg)}
        <label>
          <input
            type="radio"
            name="language"
            value={lg}
            checked={$settings.language === lg}
            onchange={() => setLang(lg)}
          />
          {$t(`settings.language.${lg}`)}
        </label>
      {/each}
    </div>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.weekStart")}</legend>
    <div class="choices">
      {#each weekStarts as ws (ws)}
        <label>
          <input
            type="radio"
            name="week-start"
            value={ws}
            checked={$settings.week_start === ws}
            onchange={() => setWeekStart(ws)}
          />
          {$t(`settings.weekStart.${ws}`)}
        </label>
      {/each}
    </div>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.timezone")}</legend>
    <label class="tz-field">
      <input
        type="text"
        placeholder={$t("settings.timezone.placeholder")}
        value={$settings.timezone === "system" ? "" : $settings.timezone}
        onchange={(e) => setTimezone(e.currentTarget.value)}
        use:fieldKeys={() =>
          $settings.timezone === "system" ? "" : $settings.timezone}
      />
    </label>
    <p class="hint">{$t("settings.timezone.hint")}</p>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.scale")}</legend>
    <div class="scale-row">
      <button
        type="button"
        class="scale-btn"
        aria-label={$t("settings.scale.decrease")}
        onclick={() => bumpUiScale(-UI_SCALE_STEP)}>−</button
      >
      <span class="scale-value" aria-live="polite"
        >{scalePercent($settings.ui_scale)}%</span
      >
      <button
        type="button"
        class="scale-btn"
        aria-label={$t("settings.scale.increase")}
        onclick={() => bumpUiScale(UI_SCALE_STEP)}>+</button
      >
      <button
        type="button"
        class="scale-reset"
        onclick={() => setUiScale(DEFAULT_UI_SCALE)}
      >
        {$t("settings.scale.reset")}
      </button>
    </div>
    <p class="hint">{$t("settings.scale.hint")}</p>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.tray")}</legend>
    <label class="toggle">
      <input
        type="checkbox"
        checked={$settings.minimize_to_tray}
        onchange={(e) => setTray(e.currentTarget.checked)}
      />
      {$t("settings.tray")}
    </label>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.servers")}</legend>
    <p class="hint">{$t("settings.poll.hint")} {$t("settings.timelog.hint")}</p>
    <ul class="server-settings">
      {#each $servers as s (s.name)}
        <li class:off={!s.enabled}>
          <label class="srv-enable" title={$t("settings.server.enabled")}>
            <input
              type="checkbox"
              checked={s.enabled}
              onchange={(e) => setEnabled(s.name, e.currentTarget.checked)}
            />
          </label>
          <span class="bk {s.backend === 'github' ? 'gh' : 'op'}">
            {s.backend === "github" ? "GH" : "OP"}
          </span>
          <label class="srv-field">
            <span>{$t("settings.server.fullName")}</span>
            <input
              type="text"
              value={s.display_name}
              onchange={(e) => setDisplayName(s.name, e.currentTarget.value)}
              use:fieldKeys={() => s.display_name}
            />
          </label>
          <label class="srv-field">
            <span>{$t("settings.server.shortName")}</span>
            <input
              type="text"
              value={s.name}
              onchange={(e) => renameShort(s.name, e.currentTarget.value)}
              use:fieldKeys={() => s.name}
            />
          </label>
          <label class="srv-field">
            <span>{$t("settings.poll")}</span>
            <input
              type="number"
              min="1"
              inputmode="numeric"
              placeholder={String(s.poll_secs)}
              value={s.poll_override ?? ""}
              onchange={(e) => setPoll(s.name, e.currentTarget.value)}
              use:fieldKeys={() => String(s.poll_override ?? "")}
            />
          </label>
          {#if s.backend !== "github"}
            <label class="srv-field">
              <span>{$t("settings.timelog")}</span>
              <input
                type="date"
                value={s.timelog_start?.date ?? ""}
                onchange={(e) => setStart(s.name, e.currentTarget.value)}
                use:fieldKeys={() => s.timelog_start?.date ?? ""}
              />
              {#if s.timelog_start?.auto}
                <span class="auto-hint">{$t("settings.timelog.auto")}</span>
              {/if}
            </label>
          {/if}
          <div class="srv-colors">
            <span class="srv-colors-title">{$t("settings.statusColors")}</span>
            {#each Object.entries(s.status_colors) as [status, color] (status)}
              <div class="srv-color-row">
                <span class="srv-color-status" title={status}>{status}</span>
                <select
                  value={color}
                  onchange={(e) =>
                    changeStatusColor(
                      s.name,
                      status,
                      e.currentTarget.value as StatusColorToken,
                    )}
                >
                  {#each COLOR_TOKENS as tok (tok)}
                    <option value={tok}>{$t(`settings.color.${tok}`)}</option>
                  {/each}
                </select>
                <span class="swatch tone-{color}" aria-hidden="true"></span>
                <button
                  type="button"
                  class="linkbtn"
                  onclick={() => removeStatusColor(s.name, status)}
                  >{$t("settings.statusColors.remove")}</button
                >
              </div>
            {/each}
            <div class="srv-color-row">
              <input
                type="text"
                class="srv-color-input"
                placeholder={$t("settings.statusColors.status")}
                bind:value={draftStatus[s.name]}
                onkeydown={(e) => {
                  if (e.key === "Enter") addStatusColor(s.name);
                }}
              />
              <select bind:value={draftColor[s.name]}>
                {#each COLOR_TOKENS as tok (tok)}
                  <option value={tok}>{$t(`settings.color.${tok}`)}</option>
                {/each}
              </select>
              <button
                type="button"
                class="btn"
                onclick={() => addStatusColor(s.name)}
                >{$t("settings.statusColors.add")}</button
              >
            </div>
          </div>
        </li>
      {/each}
    </ul>
  </fieldset>
</section>
