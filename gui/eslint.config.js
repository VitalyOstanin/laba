import js from "@eslint/js";
import ts from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import prettier from "eslint-config-prettier";
import globals from "globals";
import svelteConfig from "./svelte.config.js";

// Flat config for the SvelteKit + TypeScript frontend. Type-aware linting via
// typescript-eslint, Svelte 5 support via eslint-plugin-svelte, and
// eslint-config-prettier last so formatting is left to Prettier.
export default ts.config(
  {
    ignores: ["build/", "dist/", ".svelte-kit/", "node_modules/", "src-tauri/"],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  prettier,
  ...svelte.configs["flat/prettier"],
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      // Allow intentionally-unused bindings prefixed with `_` (e.g. the discard
      // half of an object-rest destructuring used only to drop a key).
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      // Desktop SPA (adapter-static) with an empty base path: plain internal
      // hrefs need no resolve() indirection.
      "svelte/no-navigation-without-resolve": "off",
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: [".svelte"],
        parser: ts.parser,
        svelteConfig,
      },
    },
  },
  {
    // WebdriverIO + Mocha e2e specs run in the test runner's global scope.
    files: ["e2e/**"],
    languageOptions: {
      globals: {
        ...globals.mocha,
        $: "readonly",
        $$: "readonly",
        browser: "readonly",
        expect: "readonly",
      },
    },
  },
);
