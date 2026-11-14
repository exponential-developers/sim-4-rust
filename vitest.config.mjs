// Vitest configuration for the website's TypeScript tests. Run with `npm test`.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // jsdom: a browser-like DOM, so page code can run against index.html
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    setupFiles: ["src/test/setup.ts"],
  },
});
