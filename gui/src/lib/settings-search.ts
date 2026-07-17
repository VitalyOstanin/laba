/**
 * Whether a settings section (its whole visible text — legend, labels, hints)
 * matches a search query, for the Chrome-style settings filter. The query is
 * split into whitespace-separated words and every word must appear somewhere in
 * the section text (case-insensitive), so "dark theme" matches a section
 * containing both words in any order. A blank query matches every section.
 */
export function settingsSectionMatches(text: string, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const hay = text.toLowerCase();
  return words.every((w) => hay.includes(w));
}
