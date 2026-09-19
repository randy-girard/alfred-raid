import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts", "scripts/**/*.test.ts"],
    coverage: {
      provider: "v8",
      reporter: ["lcovonly"],
      reportsDirectory: "coverage/raw/js",
      include: ["src/**/*.ts", "scripts/**/*.{ts,mjs,js}"],
      exclude: ["**/*.test.ts", "**/*.d.ts"],
      all: true,
    },
  },
});
