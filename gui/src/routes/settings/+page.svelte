<script lang="ts">
  import { get } from "svelte/store";
  import { settings, servers, setPollOverride } from "$lib/store";
  import { saveSettings } from "$lib/api";
  import { applyTheme } from "$lib/theme";
  import { language, t } from "$lib/i18n";
  import type { Theme, Lang } from "$lib/types";

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
  function setTray(v: boolean): void {
    settings.update((s) => ({ ...s, minimize_to_tray: v }));
    void persist();
  }
  function setPoll(name: string, raw: string): void {
    settings.update((s) => setPollOverride(s, name, raw));
    void persist();
  }

  const themes: Theme[] = ["system", "dark", "light"];
  const langs: Lang[] = ["system", "en", "ru"];
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
    <legend>{$t("settings.poll")}</legend>
    <p class="hint">{$t("settings.poll.hint")}</p>
    <ul class="poll-list">
      {#each $servers as s (s.name)}
        <li>
          <span class="poll-name">{s.name}</span>
          <span class="bk {s.backend === 'github' ? 'gh' : 'op'}">
            {s.backend === "github" ? "GH" : "OP"}
          </span>
          <input
            type="number"
            min="1"
            inputmode="numeric"
            aria-label={`${$t("settings.poll")}: ${s.name}`}
            placeholder={String(s.poll_secs)}
            value={$settings.poll_override[s.name] ?? ""}
            oninput={(e) => setPoll(s.name, e.currentTarget.value)}
          />
        </li>
      {/each}
    </ul>
  </fieldset>
</section>
