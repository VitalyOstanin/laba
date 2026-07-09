// Shared keyboard/focus conventions for the app.
//
// ESC convention (see TODO "UX consistency conventions"):
//   - In a focused text-like input, ESC discards the in-progress edit and
//     blurs the field. It never bubbles up to also close a panel.
//   - With no input focused, ESC closes the topmost transient surface
//     (each such surface registers its own Escape-to-close via `onGlobalEscape`).
//   - Enter in a single-line input commits and blurs (the browser default that
//     fires `onchange`); it must not submit a form or navigate.

// A text-like editable element: ESC and Enter carry field semantics here.
export function isTextInput(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  if (el instanceof HTMLTextAreaElement) return true;
  if (el instanceof HTMLInputElement) {
    return [
      "text",
      "search",
      "number",
      "date",
      "email",
      "url",
      "tel",
      "password",
    ].includes(el.type);
  }
  return el.isContentEditable === true;
}

// Svelte action for a text input applying the field key convention:
//   - ESC restores the committed value and blurs, swallowing the event so a
//     surrounding panel does not also close.
//   - Enter (single-line only, not during IME composition) blurs to commit,
//     firing `onchange`; preventDefault stops any form submit.
//
// `revert` returns the value to restore on ESC. For deferred-commit inputs
// (committed on `onchange`) it returns the store value, discarding a half-typed
// edit. For a search box it may return "" to clear. Omit it to only blur
// (nothing to revert on live-bound inputs).
export function fieldKeys(
  node: HTMLInputElement | HTMLTextAreaElement,
  revert?: () => string,
) {
  let getValue = revert;
  const multiline = node instanceof HTMLTextAreaElement;
  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.stopPropagation();
      e.preventDefault();
      if (getValue) {
        node.value = getValue();
        // Notify Svelte bindings and `onchange`/`oninput` consumers.
        node.dispatchEvent(new Event("input", { bubbles: true }));
        node.dispatchEvent(new Event("change", { bubbles: true }));
      }
      node.blur();
    } else if (e.key === "Enter" && !multiline && !e.isComposing) {
      e.preventDefault();
      node.blur();
    }
  }
  node.addEventListener("keydown", onKey as EventListener);
  return {
    update(next?: () => string): void {
      getValue = next;
    },
    destroy(): void {
      node.removeEventListener("keydown", onKey as EventListener);
    },
  };
}

// Register a window-level ESC handler for a transient surface (panel/dialog).
// Fires only when focus is not in a text input, because inputs handle ESC
// themselves and stop its propagation. Returns an unsubscribe function.
export function onGlobalEscape(close: () => void): () => void {
  function onKey(e: KeyboardEvent): void {
    if (e.key !== "Escape") return;
    if (isTextInput(document.activeElement)) return;
    close();
  }
  window.addEventListener("keydown", onKey);
  return () => window.removeEventListener("keydown", onKey);
}
