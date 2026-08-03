import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [react()],
  build: {
    sourcemap: true,
    rollupOptions: {
      input: {
        root: resolve(import.meta.dirname, "index.html"),
        es: resolve(import.meta.dirname, "es/index.html"),
        en: resolve(import.meta.dirname, "en/index.html"),
        "es-cli": resolve(import.meta.dirname, "es/cli/index.html"),
        "en-cli": resolve(import.meta.dirname, "en/cli/index.html"),
      },
    },
  },
});
