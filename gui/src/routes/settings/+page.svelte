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
    setServerStatusFilters,
    setServerDisplayFields,
    setServerProxy,
    getGlobalProxy,
    setGlobalProxy,
    renameServer,
    addServer,
    loginServer,
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
  import { friendlyError } from "$lib/friendly-error";
  import type {
    Theme,
    Lang,
    WeekStart,
    StatusColorToken,
    StatusFilter,
    ServerInfo,
  } from "$lib/types";

  // How long the "Saved" indicator stays up after a successful save.
  const SAVED_FLASH_MS = 1500;
  let saved = $state(false);
  let flash: ReturnType<typeof setTimeout> | undefined;

  // Flash the "Saved" indicator. Shared by the global settings store and the
  // per-server profile edits so both give the same save confirmation.
  function flashSaved(): void {
    saved = true;
    clearTimeout(flash);
    flash = setTimeout(() => (saved = false), SAVED_FLASH_MS);
  }

  async function persist(): Promise<void> {
    const s = get(settings);
    applyTheme(s.theme);
    language.set(s.language);
    await saveSettings(s);
    flashSaved();
  }

  // Reload the server list after a profile edit (server-level settings live in
  // config.json, not in the app settings store) and confirm the save.
  async function refreshServers(): Promise<void> {
    try {
      servers.set(await listServers());
      flashSaved();
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
  function setDesktopNotifications(v: boolean): void {
    settings.update((s) => ({ ...s, desktop_notifications: v }));
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
  async function setProxy(name: string, value: string): Promise<void> {
    await setServerProxy(name, value.trim() === "" ? null : value.trim());
    await refreshServers();
  }

  // Global default proxy (Config-level, applies to servers without an override).
  // Loaded once on mount; edits persist immediately.
  let globalProxy = $state("");
  getGlobalProxy().then((p) => (globalProxy = p ?? ""));
  async function saveGlobalProxy(value: string): Promise<void> {
    const v = value.trim();
    globalProxy = v;
    await setGlobalProxy(v === "" ? null : v);
  }

  // Per-server status-color editor. Drafts for the "add" row are keyed by server
  // name so each row keeps its own in-progress status text and color.
  const COLOR_TOKENS: StatusColorToken[] = [
    "danger",
    "warn",
    "success",
    "progress",
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

  // Per-server status-filter (task-tab) editor. Statuses are entered as a
  // comma-separated list; the whole ordered filter list is saved on each change.
  function parseStatuses(csv: string): string[] {
    return csv
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s !== "");
  }
  async function saveFilters(
    name: string,
    filters: StatusFilter[],
  ): Promise<void> {
    await setServerStatusFilters(name, filters);
    await refreshServers();
  }
  async function editFilterLabel(
    s: ServerInfo,
    i: number,
    label: string,
  ): Promise<void> {
    await saveFilters(
      s.name,
      s.status_filters.map((f, j) => (j === i ? { ...f, label } : f)),
    );
  }
  async function editFilterStatuses(
    s: ServerInfo,
    i: number,
    csv: string,
  ): Promise<void> {
    await saveFilters(
      s.name,
      s.status_filters.map((f, j) =>
        j === i ? { ...f, statuses: parseStatuses(csv) } : f,
      ),
    );
  }
  async function removeFilter(s: ServerInfo, i: number): Promise<void> {
    await saveFilters(
      s.name,
      s.status_filters.filter((_, j) => j !== i),
    );
  }
  let filterDraftLabel = $state<Record<string, string>>({});
  let filterDraftStatuses = $state<Record<string, string>>({});
  async function addFilter(s: ServerInfo): Promise<void> {
    const label = (filterDraftLabel[s.name] ?? "").trim();
    if (label === "") return;
    await saveFilters(s.name, [
      ...s.status_filters,
      { label, statuses: parseStatuses(filterDraftStatuses[s.name] ?? "") },
    ]);
    filterDraftLabel[s.name] = "";
    filterDraftStatuses[s.name] = "";
  }

  // Per-server display fields (extra task-list columns / sort keys). Ordered
  // list of custom-field names; the whole list is saved on each change.
  let displayFieldDraft = $state<Record<string, string>>({});
  async function addDisplayField(s: ServerInfo): Promise<void> {
    const name = (displayFieldDraft[s.name] ?? "").trim();
    if (name === "") return;
    if (!s.display_fields.includes(name)) {
      await setServerDisplayFields(s.name, [...s.display_fields, name]);
      await refreshServers();
    }
    displayFieldDraft[s.name] = "";
  }
  async function removeDisplayField(
    s: ServerInfo,
    name: string,
  ): Promise<void> {
    await setServerDisplayFields(
      s.name,
      s.display_fields.filter((f) => f !== name),
    );
    await refreshServers();
  }

  // Add-server form. GitHub needs no token (uses gh); OpenProject needs a token
  // set separately (keyring/CLI), so the form only creates the profile.
  let newName = $state("");
  let newUrl = $state("");
  let newBackend = $state<"openproject" | "github">("openproject");
  let newDisplay = $state("");
  let newToken = $state("");
  let addError = $state("");

  async function addNewServer(): Promise<void> {
    addError = "";
    const name = newName.trim();
    if (name === "" || newUrl.trim() === "") return;
    try {
      await addServer(
        name,
        newUrl.trim(),
        newBackend,
        newDisplay.trim() === "" ? null : newDisplay.trim(),
      );
      // If an OpenProject token was supplied, validate and store it now so the
      // profile works without a separate CLI step. A bad token surfaces as an
      // error; the profile still exists and can be signed into later.
      if (newBackend === "openproject" && newToken.trim() !== "") {
        await loginServer(name, newToken.trim(), false);
      }
      newName = "";
      newUrl = "";
      newDisplay = "";
      newToken = "";
      newBackend = "openproject";
      await refreshServers();
    } catch (e) {
      addError = friendlyError(String(e), get(t)).text;
    }
  }

  // Per-server sign-in (enter/replace the OpenProject token from the list).
  let signInDraft = $state<Record<string, string>>({});
  let signInError = $state<Record<string, string>>({});
  async function signIn(name: string): Promise<void> {
    const token = (signInDraft[name] ?? "").trim();
    if (token === "") return;
    signInError[name] = "";
    try {
      await loginServer(name, token, false);
      signInDraft[name] = "";
      await refreshServers();
    } catch (e) {
      signInError[name] = friendlyError(String(e), get(t)).text;
    }
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
    <legend>{$t("settings.notifications")}</legend>
    <label class="toggle">
      <input
        type="checkbox"
        checked={$settings.desktop_notifications}
        onchange={(e) => setDesktopNotifications(e.currentTarget.checked)}
      />
      {$t("settings.notifications")}
    </label>
  </fieldset>

  <fieldset>
    <legend>{$t("settings.proxy.global")}</legend>
    <label class="tz-field">
      <input
        type="text"
        placeholder={$t("settings.proxy.placeholder")}
        value={globalProxy}
        onchange={(e) => saveGlobalProxy(e.currentTarget.value)}
        use:fieldKeys={() => globalProxy}
      />
    </label>
    <p class="hint">{$t("settings.proxy.hint")}</p>
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
            {s.backend === "github" ? "GitHub" : "OpenProject"}
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
          {#if s.backend !== "github"}
            <div class="srv-signin">
              <span class="srv-token-state" class:ok={s.has_token}>
                {s.has_token
                  ? $t("settings.server.hasToken")
                  : $t("settings.server.noToken")}
              </span>
              <input
                type="password"
                class="srv-token-input"
                placeholder={$t("settings.server.token")}
                bind:value={signInDraft[s.name]}
                onkeydown={(e) => {
                  if (e.key === "Enter") signIn(s.name);
                }}
              />
              <button type="button" class="btn" onclick={() => signIn(s.name)}
                >{$t("settings.server.signIn")}</button
              >
              {#if signInError[s.name]}
                <span class="add-error" role="alert">{signInError[s.name]}</span
                >
              {/if}
            </div>
          {/if}
          <details class="srv-advanced">
            <summary>{$t("settings.server.advanced")}</summary>
            <label class="srv-field adv">
              <span>{$t("settings.proxy")}</span>
              <input
                type="text"
                placeholder={$t("settings.proxy.placeholder")}
                value={s.proxy ?? ""}
                onchange={(e) => setProxy(s.name, e.currentTarget.value)}
                use:fieldKeys={() => s.proxy ?? ""}
              />
            </label>
            <div class="srv-colors">
              <span class="srv-colors-title">{$t("settings.statusColors")}</span
              >
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
            {#if s.supports_status_filters}
              <div class="srv-colors">
                <span class="srv-colors-title">{$t("settings.filters")}</span>
                <span class="hint">{$t("settings.filters.hint")}</span>
                {#each s.status_filters as f, i (i)}
                  <div class="srv-color-row">
                    <input
                      type="text"
                      class="srv-color-input"
                      placeholder={$t("settings.filters.label")}
                      value={f.label}
                      onchange={(e) =>
                        editFilterLabel(s, i, e.currentTarget.value)}
                    />
                    <input
                      type="text"
                      class="srv-filter-statuses"
                      placeholder={$t("settings.filters.statuses")}
                      value={f.statuses.join(", ")}
                      onchange={(e) =>
                        editFilterStatuses(s, i, e.currentTarget.value)}
                    />
                    <button
                      type="button"
                      class="linkbtn"
                      onclick={() => removeFilter(s, i)}
                      >{$t("settings.statusColors.remove")}</button
                    >
                  </div>
                {/each}
                <div class="srv-color-row">
                  <input
                    type="text"
                    class="srv-color-input"
                    placeholder={$t("settings.filters.label")}
                    bind:value={filterDraftLabel[s.name]}
                  />
                  <input
                    type="text"
                    class="srv-filter-statuses"
                    placeholder={$t("settings.filters.statuses")}
                    bind:value={filterDraftStatuses[s.name]}
                    onkeydown={(e) => {
                      if (e.key === "Enter") addFilter(s);
                    }}
                  />
                  <button type="button" class="btn" onclick={() => addFilter(s)}
                    >{$t("settings.statusColors.add")}</button
                  >
                </div>
              </div>
            {/if}
            {#if s.supports_custom_fields}
              <div class="srv-colors">
                <span class="srv-colors-title"
                  >{$t("settings.displayFields")}</span
                >
                <span class="hint">{$t("settings.displayFields.hint")}</span>
                {#each s.display_fields as f (f)}
                  <div class="srv-color-row">
                    <span class="srv-field-name">{f}</span>
                    <button
                      type="button"
                      class="linkbtn"
                      onclick={() => removeDisplayField(s, f)}
                      >{$t("settings.displayFields.remove")}</button
                    >
                  </div>
                {/each}
                <div class="srv-color-row">
                  <input
                    type="text"
                    class="srv-color-input"
                    placeholder={$t("settings.displayFields.field")}
                    bind:value={displayFieldDraft[s.name]}
                    onkeydown={(e) => {
                      if (e.key === "Enter") addDisplayField(s);
                    }}
                  />
                  <button
                    type="button"
                    class="btn"
                    onclick={() => addDisplayField(s)}
                    >{$t("settings.displayFields.add")}</button
                  >
                </div>
              </div>
            {/if}
          </details>
        </li>
      {/each}
    </ul>

    <div class="add-server">
      <span class="add-server-title">{$t("settings.addServer")}</span>
      <div class="add-server-row">
        <input
          type="text"
          class="as-name"
          placeholder={$t("settings.server.shortName")}
          bind:value={newName}
        />
        <input
          type="text"
          class="as-url"
          placeholder={$t("settings.addServer.url")}
          bind:value={newUrl}
        />
        <select bind:value={newBackend}>
          <option value="openproject">OpenProject</option>
          <option value="github">GitHub</option>
        </select>
        <input
          type="text"
          class="as-display"
          placeholder={$t("settings.server.fullName")}
          bind:value={newDisplay}
        />
        {#if newBackend === "openproject"}
          <input
            type="password"
            class="as-token"
            placeholder={$t("settings.addServer.token")}
            bind:value={newToken}
          />
        {/if}
        <button type="button" class="btn" onclick={addNewServer}
          >{$t("settings.addServer.add")}</button
        >
      </div>
      {#if newBackend === "github"}
        <span class="hint">{$t("settings.addServer.githubHint")}</span>
      {:else}
        <span class="hint">{$t("settings.addServer.openprojectHint")}</span>
      {/if}
      {#if addError}<span class="add-error" role="alert">{addError}</span>{/if}
    </div>
  </fieldset>
</section>
