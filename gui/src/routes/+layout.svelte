<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { getSettings, saveSettings } from "$lib/api";
  import { settings } from "$lib/store";
  import { applyTheme } from "$lib/theme";
  import {
    applyUiScale,
    clampUiScale,
    DEFAULT_UI_SCALE,
    UI_SCALE_STEP,
  } from "$lib/scale";
  import { language } from "$lib/i18n";

  let { children } = $props();

  // Apply and persist a new interface scale (shared by the keyboard shortcuts).
  function setUiScale(next: number): void {
    const v = clampUiScale(next);
    settings.update((s) => ({ ...s, ui_scale: v }));
    applyUiScale(v);
    void saveSettings(get(settings));
  }

  onMount(() => {
    // Surface otherwise-silent async failures in the webview console.
    const onRejection = (e: PromiseRejectionEvent) => {
      console.error("Unhandled promise rejection:", e.reason);
    };
    window.addEventListener("unhandledrejection", onRejection);

    // Browser-style zoom: Ctrl/Cmd with +, -, 0. preventDefault so our own,
    // persisted scale is used instead of the webview's transient zoom.
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      const cur = get(settings).ui_scale;
      if (e.key === "0") {
        e.preventDefault();
        setUiScale(DEFAULT_UI_SCALE);
      } else if (e.key === "=" || e.key === "+") {
        e.preventDefault();
        setUiScale(cur + UI_SCALE_STEP);
      } else if (e.key === "-" || e.key === "_") {
        e.preventDefault();
        setUiScale(cur - UI_SCALE_STEP);
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      window.removeEventListener("unhandledrejection", onRejection);
      window.removeEventListener("keydown", onKey);
    };
  });

  onMount(async () => {
    try {
      const s = await getSettings();
      settings.set(s);
      applyTheme(s.theme);
      applyUiScale(s.ui_scale);
      language.set(s.language);
    } catch {
      // Keep defaults when settings cannot be loaded.
    }
  });
</script>

{@render children()}
