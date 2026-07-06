import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "node:path";

function normalizeWindowsDrive(p: string): string {
  if (process.platform === "win32" && /^[a-z]:/.test(p)) {
    return p[0].toUpperCase() + p.slice(1);
  }
  return p;
}

const root = normalizeWindowsDrive(path.resolve("."));

export default defineConfig({
  root,
  plugins: [svelte({ hot: !process.env.VITEST })],
  resolve: {
    conditions: ["browser"],
  },
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["src/lib/testing/vitest-setup.ts"],
    exclude: ["tests/visual/**", "node_modules/**", "dist/**", "src-tauri/**"],
  },
});
