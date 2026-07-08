import { defineConfig } from "vitest/config";

// Unit tests for pure TS modules (locales, store). Component tests (Task 6)
// switch the environment to jsdom and add the Svelte plugin.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
