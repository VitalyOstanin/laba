import { describe, it, expect } from "vitest";
import { renderMarkdown, isExternalHref } from "./markdown";

describe("renderMarkdown", () => {
  it("returns empty string for empty or whitespace input", () => {
    expect(renderMarkdown("")).toBe("");
    expect(renderMarkdown(null)).toBe("");
    expect(renderMarkdown(undefined)).toBe("");
    expect(renderMarkdown("   \n  ")).toBe("");
  });

  it("renders basic Markdown constructs", () => {
    const html = renderMarkdown("# Title\n\n- one\n- two");
    expect(html).toContain("<h1>Title</h1>");
    expect(html).toContain("<li>one</li>");
    expect(html).toContain("<li>two</li>");
  });

  it("escapes raw HTML instead of passing it through", () => {
    const html = renderMarkdown("hello <script>alert(1)</script> world");
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });

  it("drops dangerous link schemes but keeps http/https", () => {
    const html = renderMarkdown(
      "[bad](javascript:alert(1)) [ok](https://e.com)",
    );
    // The dangerous scheme is never turned into an anchor href; the source text
    // may remain, but there is no clickable javascript: link.
    expect(html).not.toContain('href="javascript:');
    expect(html).toContain('href="https://e.com"');
  });

  it("adds rel=noopener to rendered links", () => {
    const html = renderMarkdown("[ok](https://e.com)");
    expect(html).toContain('rel="noopener noreferrer"');
  });

  it("turns single newlines into line breaks", () => {
    expect(renderMarkdown("a\nb")).toContain("<br>");
  });
});

describe("isExternalHref", () => {
  it("accepts http, https and mailto", () => {
    expect(isExternalHref("https://e.com")).toBe(true);
    expect(isExternalHref("http://e.com")).toBe(true);
    expect(isExternalHref("mailto:a@e.com")).toBe(true);
    expect(isExternalHref("  HTTPS://E.com  ")).toBe(true);
  });

  it("rejects empty, in-app anchors and dangerous schemes", () => {
    expect(isExternalHref(null)).toBe(false);
    expect(isExternalHref("")).toBe(false);
    expect(isExternalHref("#section")).toBe(false);
    expect(isExternalHref("javascript:alert(1)")).toBe(false);
    expect(isExternalHref("file:///etc/passwd")).toBe(false);
  });
});
