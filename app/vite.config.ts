/// <reference types="vitest/config" />
import { fileURLToPath, URL } from "node:url";

import ui from "@nuxt/ui/vite";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

import { uiTheme } from "./src/ui.config";

// Config do frontend Vue (View + ViewModel) — Tauri 2.0 lê `frontendDist`
// (build/) e `devUrl` (http://localhost:5173) via ../src-tauri/tauri.conf.json.
//
// `ui()` (Nuxt UI 4, modo Vue) traz Tailwind v4 + Reka UI + ícones (Iconify,
// resolvidos localmente via @iconify-json/lucide — offline/CSP, D3) e
// registra o virtual module `@nuxt/ui/vue-plugin` usado em `main.ts`.
export default defineConfig({
  plugins: [vue(), ui(uiTheme)],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    outDir: "dist",
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  test: {
    environment: "jsdom",
    globals: true,
    include: ["tests/**/*.spec.ts"],
    setupFiles: ["./tests/setup.ts"],
  },
});
