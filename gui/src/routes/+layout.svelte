<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { getSettings } from "$lib/api";
  import { settings } from "$lib/store";
  import { applyTheme } from "$lib/theme";
  import { applyUiScale } from "$lib/scale";
  import { language } from "$lib/i18n";

  let { children } = $props();

  onMount(() => {
    // Surface otherwise-silent async failures in the webview console.
    const onRejection = (e: PromiseRejectionEvent) => {
      console.error("Unhandled promise rejection:", e.reason);
    };
    window.addEventListener("unhandledrejection", onRejection);
    return () => window.removeEventListener("unhandledrejection", onRejection);
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
