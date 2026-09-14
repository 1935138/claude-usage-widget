import { defineConfig } from "vite";

export default defineConfig({
  // Tauri points its dev window at this fixed port and fails fast if it moves.
  server: { port: 1420, strictPort: true },
  build: { target: "chrome105", emptyOutDir: true },
});
