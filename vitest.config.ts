import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";

// Dedicated test config: the main vite.config.ts carries Tauri dev-server
// settings (fixed port, file watching) that are irrelevant — and disruptive —
// under Vitest. svelteTesting() wires the browser resolve condition so Svelte 5
// components render client-side in jsdom and auto-cleanup runs between tests.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  resolve: {
    alias: {
      $lib: new URL("./src/lib", import.meta.url).pathname,
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./vitest-setup.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/lib/**"],
    },
  },
});
