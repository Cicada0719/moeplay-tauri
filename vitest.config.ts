import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "node:path";
import { fileURLToPath } from "node:url";

function normalizeWindowsDrive(p: string): string {
  if (process.platform === "win32" && /^[a-z]:/.test(p)) {
    return p[0].toUpperCase() + p.slice(1);
  }
  return p;
}

// Anchor config resolution to this file rather than the caller's current
// directory. This keeps Vite/Vitest module IDs consistent on Windows, where
// drive-letter casing is preserved by child_process cwd handling.
const root = normalizeWindowsDrive(path.resolve(fileURLToPath(new URL(".", import.meta.url))));

export default defineConfig({
  root,
  plugins: [svelte({ hot: !process.env.VITEST })],
  resolve: {
    conditions: ["browser"],
  },
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: [path.join(root, "src/lib/testing/vitest-setup.ts")],
    exclude: ["tests/visual/**", "tests/concept/**", "node_modules/**", "dist/**", "src-tauri/**"],
  },
});
