import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import vue from "@vitejs/plugin-vue";

// 弥音启动器前端构建配置。
// - 固定端口，便于 Tauri 的 devUrl 与浏览器调试模式共用
// - 构建目标对齐 WebView2（Chromium 110+）
export default defineConfig({
  // 两个入口：index.html 是用户侧（Vue），ide.html 是开发者页（React）。
  // 两边不共用运行时，只通过 IPC 说话 —— 见 AGENTS.md §18。
  plugins: [
    vue(),
    // Ring UI 发的是**带 JSX 的 .js**，默认只认 .jsx/.tsx 的转译器碰不到它，
    // 所以把 include 扩到 ring-ui 自己的 .js；exclude 清空是因为默认那条
    // /node_modules/ 会把 pnpm 的真实路径一起挡掉。
    react({
      include: [/\.[jt]sx$/, /@jetbrains[\\/]ring-ui.*\\.js$/],
      exclude: [],
    }),
  ],
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
        ide: "ide.html",
      },
    },
    emptyOutDir: true,
    target: "chrome110",
    sourcemap: false,
  },
});
