import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";

// jsdom + the Svelte plugin so both pure-TS unit tests (locales, store) and
// component tests (@testing-library/svelte) run under one config.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  resolve: {
    // The SvelteKit Vite plugin (which provides `$app/*`) is not loaded here,
    // so stub the `$app` modules that components import.
    alias: {
      "$app/navigation": fileURLToPath(
        new URL("./src/test-stubs/app-navigation.ts", import.meta.url),
      ),
      "$app/state": fileURLToPath(
        new URL("./src/test-stubs/app-state.ts", import.meta.url),
      ),
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    setupFiles: ["./vitest-setup.ts"],
    // Cap worker parallelism so the suite never saturates the host CPU
    // (shared machine, may run alongside other builds).
    maxWorkers: 4,
  },
});
