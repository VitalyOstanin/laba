// Interface scale factor, mirroring core's clamp (settings.rs: MIN/MAX/DEFAULT_UI_SCALE).
// 1 = no scaling (100%).
export const MIN_UI_SCALE = 0.5;
export const MAX_UI_SCALE = 2;
export const DEFAULT_UI_SCALE = 1;
export const UI_SCALE_STEP = 0.05;

/** Clamp a scale factor to the accepted range; 0/non-finite (blank) -> default. */
export function clampUiScale(scale: number): number {
  if (!Number.isFinite(scale) || scale === 0) return DEFAULT_UI_SCALE;
  return Math.min(MAX_UI_SCALE, Math.max(MIN_UI_SCALE, scale));
}

/**
 * Apply the interface scale to the document root font size. Everything sized in
 * `rem` scales with it; a factor of `1` restores the browser default.
 */
export function applyUiScale(scale: number): void {
  document.documentElement.style.fontSize = `${clampUiScale(scale) * 100}%`;
}
