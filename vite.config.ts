import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        // Keep the animation runtime independently cacheable and prevent the
        // application entry chunk from absorbing every GSAP consumer.
        manualChunks: {
          gsap: ["gsap"],
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
