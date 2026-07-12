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
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      // Gate the pure-logic layer, which is what the unit tests cover. Svelte
      // components are exercised by @testing-library/svelte + the wdio e2e, not
      // line-covered here, so including them would force a meaningless floor.
      include: ["src/lib/**/*.ts"],
      exclude: [
        "src/lib/**/*.test.ts",
        "src/lib/types.ts",
        // Thin wrappers over Tauri `invoke`/plugin APIs that need the native
        // host — exercised by the wdio e2e, not unit-testable under jsdom.
        "src/lib/api.ts",
        "src/lib/invoke.ts",
        "src/lib/external.ts",
        "src/lib/scale.ts",
        // Dev-only fixture data, not shipped application logic.
        "src/lib/dev-mock.ts",
      ],
      // Floors set a few points below the current measured coverage (lines 63,
      // stmts 61, funcs 67, branch 56) so they catch a real regression without
      // a thin margin that a small change would trip.
      thresholds: {
        lines: 58,
        functions: 62,
        statements: 58,
        branches: 52,
      },
    },
  },
});
