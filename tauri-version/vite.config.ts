import { defineConfig } from "vitest/config";
import preact from "@preact/preset-vite";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.dirname(fileURLToPath(import.meta.url));

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    preact(),
    {
      // Vite 7 requires `./`-prefixed globs for `new URL(..., import.meta.url)` asset expansion.
      name: "foliate-pdfjs-glob-fix",
      transform(code, id) {
        if (!id.replace(/\\/g, "/").endsWith("foliate-js/pdf.js")) return null;
        return code.replace(
          "new URL(`vendor/pdfjs/${path}`, import.meta.url)",
          "new URL(`./vendor/pdfjs/${path}`, import.meta.url)",
        );
      },
    },
  ],

  resolve: {
    alias: {
      "foliate-js": path.resolve(rootDir, "vendor/foliate-js"),
    },
  },

  test: {
    css: { include: [/coverPatterns/] },
  },

  build: {
    // Вне исходников приложения — рядом с Cargo/release артефактами.
    outDir: "../.artifacts/dist",
    emptyOutDir: true,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
