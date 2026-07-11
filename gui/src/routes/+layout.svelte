<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { attachConsole } from "@tauri-apps/plugin-log";
  import {
    getSettings,
    saveSettings,
    quitApp,
    closeWindow,
    setTrayStatus,
  } from "$lib/api";
  import {
    settings,
    unreadCount,
    byServer,
    activeServer,
    servers,
  } from "$lib/store";
  import { applyTheme } from "$lib/theme";
  import {
    applyUiScale,
    clampUiScale,
    DEFAULT_UI_SCALE,
    UI_SCALE_STEP,
  } from "$lib/scale";
  import { language } from "$lib/i18n";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import GhDependencyBanner from "$lib/components/GhDependencyBanner.svelte";

  let { children } = $props();

  // Tray attention badge: unread notifications (aggregate) plus the active
  // server's tasks in a red (danger) status. A count > 0 paints a red badge in
  // the system tray; 0 clears it.
  const redTaskCount = $derived.by((): number => {
    const name = $activeServer;
    if (!name) return 0;
    const st = $byServer[name];
    const info = $servers.find((s) => s.name === name);
    if (!st || !info) return 0;
    return (st.tasks ?? []).filter((tk) => {
      const status = tk.status == null ? "" : String(tk.status);
      return info.status_colors?.[status] === "danger";
    }).length;
  });
  const attention = $derived($unreadCount + redTaskCount);
  $effect(() => {
    void setTrayStatus(attention);
  });

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
      // Layout-independent: match the physical key via `code` (KeyQ/KeyW), so
      // Ctrl+Q / Ctrl+W work under a Cyrillic layout too.
      if (e.code === "KeyQ") {
        e.preventDefault();
        void quitApp();
        return;
      }
      if (e.code === "KeyW") {
        e.preventDefault();
        void closeWindow();
        return;
      }
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

  onMount(() => {
    // Pipe Rust `log` records (Webview target) into the browser console.
    let detach: (() => void) | undefined;
    void attachConsole()
      .then((d) => (detach = d))
      .catch(() => {});
    return () => detach?.();
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

<UpdateBanner />
<GhDependencyBanner />
{@render children()}
