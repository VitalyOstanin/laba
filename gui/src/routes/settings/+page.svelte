<script lang="ts">
  import { get } from "svelte/store";
  import {
    settings,
    servers,
    setPollOverride,
    setServerEnabled,
    setTimelogStart,
  } from "$lib/store";
  import { saveSettings } from "$lib/api";
  import { applyTheme } from "$lib/theme";
  import { language, t } from "$lib/i18n";
  import type { Theme, Lang, WeekStart } from "$lib/types";

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
  function setTimezone(raw: string): void {
    const tz = raw.trim() === "" ? null : raw.trim();
    settings.update((s) => ({ ...s, timezone: tz }));
    void persist();
  }
  function setPoll(name: string, raw: string): void {
    settings.update((s) => setPollOverride(s, name, raw));
    void persist();
  }
  function setEnabled(name: string, enabled: boolean): void {
    settings.update((s) => setServerEnabled(s, name, enabled));
    void persist();
  }
  function setStart(name: string, date: string): void {
    settings.update((s) => setTimelogStart(s, name, date));
    void persist();
  }

  const themes: Theme[] = ["system", "dark", "light"];
  const langs: Lang[] = ["system", "en", "ru"];
  const weekStarts: WeekStart[] = ["monday", "sunday"];
</script>

<section class="settings" aria-label={$t("settings.title")}>
  <header class="settings-head">
    <a class="back" href="/">← {$t("nav.dashboard")}</a>
    <h1>{$t("settings.title")}</h1>
    {#if saved}<span class="saved" role="status">{$t("settings.saved")}</span>{/if}
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
        value={$settings.timezone ?? ""}
        onchange={(e) => setTimezone(e.currentTarget.value)}
      />
    </label>
    <p class="hint">{$t("settings.timezone.hint")}</p>
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
          <span class="srv-name">{s.name}</span>
          <span class="bk {s.backend === 'github' ? 'gh' : 'op'}">
            {s.backend === "github" ? "GH" : "OP"}
          </span>
          <label class="srv-field">
            <span>{$t("settings.poll")}</span>
            <input
              type="number"
              min="1"
              inputmode="numeric"
              placeholder={String(s.poll_secs)}
              value={$settings.poll_override[s.name] ?? ""}
              oninput={(e) => setPoll(s.name, e.currentTarget.value)}
            />
          </label>
          {#if s.backend !== "github"}
            <label class="srv-field">
              <span>{$t("settings.timelog")}</span>
              <input
                type="date"
                value={$settings.timelog_start[s.name]?.date ?? ""}
                onchange={(e) => setStart(s.name, e.currentTarget.value)}
              />
              {#if $settings.timelog_start[s.name]?.auto}
                <span class="auto-hint">{$t("settings.timelog.auto")}</span>
              {/if}
            </label>
          {/if}
        </li>
      {/each}
    </ul>
  </fieldset>
</section>
