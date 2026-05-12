import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    minify: "esbuild",
    rollupOptions: {
      output: {
        manualChunks: {
          "render-vendor": ["dompurify", "markdown-it"],
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    exclude: ["node_modules/**", "dist/**", "src-tauri/**", "tests/e2e/**"],
    setupFiles: "./vitest.setup.ts",
    globals: true,
  },
});
