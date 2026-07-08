import type { Theme } from "./types";

/**
 * Apply a theme choice to the document root.
 *
 * `system` removes the `data-theme` attribute so the CSS `prefers-color-scheme`
 * rules take over; `dark`/`light` force the matching token set.
 */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", theme);
  }
}
