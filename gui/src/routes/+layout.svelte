<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { getSettings } from "$lib/api";
  import { settings } from "$lib/store";
  import { applyTheme } from "$lib/theme";
  import { language } from "$lib/i18n";

  let { children } = $props();

  onMount(async () => {
    try {
      const s = await getSettings();
      settings.set(s);
      applyTheme(s.theme);
      language.set(s.language);
    } catch {
      // Keep defaults when settings cannot be loaded.
    }
  });
</script>

{@render children()}
