/** Format a non-negative minute count as `Xh Ym` / `Xh` / `Ym` / `0m`. */
export function fmtMinutes(min: number): string {
  const m = Math.max(0, Math.round(min));
  const h = Math.floor(m / 60);
  const r = m % 60;
  if (h && r) return `${h}h ${r}m`;
  if (h) return `${h}h`;
  return `${r}m`;
}

/** Format a signed minute delta as `+Xh Ym` / `−Xh Ym` / `0`. */
export function fmtSigned(min: number): string {
  const r = Math.round(min);
  if (r === 0) return "0";
  const sign = r > 0 ? "+" : "−";
  return `${sign}${fmtMinutes(Math.abs(r))}`;
}
