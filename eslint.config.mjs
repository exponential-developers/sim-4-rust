// ESLint configuration (ESLint 9 flat config). Run with `npm run lint`.
//
// The TypeScript rules are type-aware: they read the types from tsconfig.test.json, which covers
// all of src (tsconfig.json leaves the tests out for the webpack build).
//
// Existing errors are recorded in eslint-suppressions.json, so only new errors fail the lint.
// After fixing suppressed errors, run `npx eslint src --prune-suppressions` to shrink that file.
import js from "@eslint/js";
import tseslint from "typescript-eslint";
import globals from "globals";

export default tseslint.config(
  { ignores: ["build/", "wasm/", "bench/", "node_modules/"] },
  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked.map((config) => ({ ...config, files: ["src/**/*.ts"] })),
  {
    files: ["src/**/*.ts"],
    languageOptions: {
      globals: { ...globals.browser, ...globals.worker },
      parserOptions: {
        project: "./tsconfig.test.json",
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      // Use real types (or `unknown` and narrow it), not `any`
      "@typescript-eslint/no-explicit-any": "error",
      // Unused variables are errors, except names starting with _
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
    },
  },
);
