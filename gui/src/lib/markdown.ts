/**
 * Render task descriptions and comments (Markdown from OpenProject's `raw`
 * field) to HTML for display.
 *
 * Security: the renderer runs with `html: false`, so any raw HTML in the source
 * is escaped rather than passed through — the output only ever contains the
 * fixed set of tags Markdown produces (no `<script>`, `<style>`, or inline
 * event handlers). markdown-it's default link validation already drops
 * dangerous schemes (`javascript:` etc.), so no separate HTML sanitizer is
 * needed. Links get `rel="noopener noreferrer"`; the component intercepts
 * clicks and opens them in the system browser instead of navigating the webview
 * (see {@link isExternalHref}). This keeps the strict webview CSP intact.
 */
import MarkdownIt from "markdown-it";

const md = new MarkdownIt({
  html: false, // escape raw HTML in the source (primary XSS defense)
  linkify: true, // turn bare URLs into links
  breaks: true, // single newline -> <br>, matching how OpenProject renders
});

// Harden rendered links: force rel="noopener noreferrer" so an opened link can
// never reach back into the app via window.opener.
const defaultLinkOpen =
  md.renderer.rules.link_open ??
  ((tokens, idx, options, _env, self) =>
    self.renderToken(tokens, idx, options));
md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
  tokens[idx].attrSet("rel", "noopener noreferrer");
  return defaultLinkOpen(tokens, idx, options, env, self);
};

/**
 * Render a Markdown string to a safe HTML string. Empty or whitespace-only
 * input yields an empty string. Pure, so it is unit-tested.
 */
export function renderMarkdown(src: string | null | undefined): string {
  if (!src || !src.trim()) return "";
  return md.render(src);
}

/**
 * Whether a link href should be opened in the system browser: only the schemes
 * safe to hand to the OS opener. Anything else (in-app anchors, unknown or
 * dangerous schemes) is ignored. Pure, so it is unit-tested.
 */
export function isExternalHref(href: string | null | undefined): boolean {
  if (!href) return false;
  return /^(https?|mailto):/i.test(href.trim());
}
