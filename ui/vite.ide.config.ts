import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

/**
 * 开发者页（React + Ring UI）**单独构建**。
 *
 * 为什么不在主构建里加第二个入口：rolldown 会把两个入口的公共块拆出来，
 * 而那个块会被 index.html **预加载** —— 结果用户侧去执行 `createRoot(#ide)`，
 * 容器不存在，直接白屏。两个入口本来就是独立的前端（AGENTS.md §18），
 * 分开构建才是它们的真实关系。
 *
 * `emptyOutDir: false` 是必须的：不能把主构建的产物删掉。
 */
export default defineConfig({
  plugins: [
    // Ring UI 发的是**带 JSX 的 .js**，默认只认 .jsx/.tsx 的转译器碰不到它；
    // exclude 清空是因为默认那条 /node_modules/ 会把 pnpm 的真实路径一起挡掉。
    react({
      include: [/\.[jt]sx$/, /@jetbrains[\\/]ring-ui.*\.js$/],
      exclude: [],
    }),
  ],
  clearScreen: false,
  build: {
    outDir: "dist",
    emptyOutDir: false,
    target: "chrome110",
    sourcemap: false,
    rollupOptions: {
      input: {
        ide: "ide.html",
      },
    },
  },
});
