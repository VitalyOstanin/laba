// Test stub for SvelteKit's `$app/navigation`. The real module is provided by
// the SvelteKit Vite plugin, which the vitest config does not load, so tests
// alias `$app/navigation` here. Only what components touch is stubbed.
export const goto = (): Promise<void> => Promise.resolve();
