import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// 弥音启动器前端构建配置。
// - 固定端口，便于 Tauri 的 devUrl 与浏览器调试模式共用
// - 构建目标对齐 WebView2（Chromium 110+）
export default defineConfig({
  // 这个构建**只管用户侧**（Vue）。开发者页是独立的一次构建（见 vite.ide.config.ts）——
  // 放在一起的话 rolldown 会把两边拆出的公共块塞进 index.html 预加载，
  // 结果用户侧执行到开发者页的挂载代码，白屏。
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    rollupOptions: {
      input: {
        main: "index.html",
      },
    },
    emptyOutDir: true,
    target: "chrome110",
    sourcemap: false,
  },
});
