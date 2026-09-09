import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  optimizeDeps: {
    include: ["@tauri-apps/plugin-updater", "@tauri-apps/plugin-process", "hls.js", "date-fns/locale"],
  },
  build: {
    outDir: "dist",
    rollupOptions: {
      input: { app: "index.html" },
      output: {
        manualChunks: { gsap: ["gsap"], pinyin: ["pinyin-pro"] },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**", "**/_dev/**", "**/artifacts/**", "**/test-results/**", "**/playwright-report/**"] },
  },
});
