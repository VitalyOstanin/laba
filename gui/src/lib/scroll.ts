/**
 * Svelte action: call `onEnter` whenever the node scrolls into the viewport.
 * Used as a bottom sentinel to drive incremental reveal and paged fetching.
 *
 * `IntersectionObserver` is absent under jsdom, so there the action degrades to
 * a no-op and the visible "load more" button remains the working fallback.
 */
export function onVisible(
  node: HTMLElement,
  onEnter: () => void,
): { update(next: () => void): void; destroy(): void } {
  let cb = onEnter;
  if (typeof IntersectionObserver === "undefined") {
    return {
      update(next: () => void) {
        cb = next;
      },
      destroy() {},
    };
  }
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) if (e.isIntersecting) cb();
  });
  io.observe(node);
  return {
    update(next: () => void) {
      cb = next;
    },
    destroy() {
      io.disconnect();
    },
  };
}
