import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// 弥音启动器前端构建配置。
// - 固定端口，便于 Tauri 的 devUrl 与浏览器调试模式共用
// - 构建目标对齐 WebView2（Chromium 110+）
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome110",
    sourcemap: false,
  },
});
