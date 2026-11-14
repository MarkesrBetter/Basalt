import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    outDir: "dist",
    target: ["es2022", "chrome105", "edge105"],
    minify: "esbuild",
    emptyOutDir: true,
  },
});
