import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // ضروري على ويندوز: مجلد بناء Rust يحتوي ملفات .exe مؤقتة تُقفَل
      // أثناء الترجمة، فتتعارض مع مراقب ملفات Vite (خطأ EBUSY)
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: true,
  },
});
