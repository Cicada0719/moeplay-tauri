import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: {
    outDir: "dist",
    rollupOptions: {
      input: { app: resolve(process.cwd(), "index.html") },
      output: {
        manualChunks: { gsap: ["gsap"], pinyin: ["pinyin-pro"] },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
