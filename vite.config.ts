import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

export default defineConfig(({ mode }) => {
  const concept = mode === "concept";
  return {
    plugins: [svelte()],
    clearScreen: false,
    build: {
      outDir: concept ? "dist-concept" : "dist",
      rollupOptions: {
        input: concept
          ? { concept: resolve(process.cwd(), "concept/index.html") }
          : { app: resolve(process.cwd(), "index.html") },
        output: {
          manualChunks: concept
            ? { gsap: ["gsap"], three: ["three"] }
            : { gsap: ["gsap"] },
        },
      },
    },
    server: {
      port: 1420,
      strictPort: true,
      host: "127.0.0.1",
      watch: { ignored: ["**/src-tauri/**"] },
    },
  };
});
