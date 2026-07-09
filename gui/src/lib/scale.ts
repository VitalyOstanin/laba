// Interface scale, mirroring core's clamp (settings.rs: MIN/MAX/DEFAULT_UI_SCALE).
export const MIN_UI_SCALE = 50;
export const MAX_UI_SCALE = 200;
export const DEFAULT_UI_SCALE = 100;
export const UI_SCALE_STEP = 10;

/** Clamp a scale to the accepted range; 0 (blank) maps to the default. */
export function clampUiScale(scale: number): number {
  if (!Number.isFinite(scale) || scale === 0) return DEFAULT_UI_SCALE;
  return Math.min(MAX_UI_SCALE, Math.max(MIN_UI_SCALE, Math.round(scale)));
}

/**
 * Apply the interface scale to the document root font size. Everything sized in
 * `rem` scales with it; `100` restores the browser default.
 */
export function applyUiScale(scale: number): void {
  document.documentElement.style.fontSize = `${clampUiScale(scale)}%`;
}
