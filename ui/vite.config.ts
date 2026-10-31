import { writeFileSync } from "node:fs";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// 每次构建打一个时间戳：写进 dist/BUILD-INFO.txt，也注入到页面里。
// 这样「手上这份是不是最新编译的」不用靠文件名猜（产物名是内容哈希，
// 源码没变时两次构建名字一样，看起来就像没更新）。
const buildStamp = new Date().toISOString();

// 弥音启动器前端构建配置。
// - 固定端口，便于 Tauri 的 devUrl 与浏览器调试模式共用
// - 构建目标对齐 WebView2（Chromium 110+）
export default defineConfig({
  // 这个构建**只管用户侧**（React）。开发者页是独立的一次构建（见 vite.ide.config.ts）——
  // 放在一起的话 rolldown 会把两边拆出的公共块塞进 index.html 预加载，
  // 结果用户侧执行到开发者页的挂载代码，白屏。
  // Tailwind 只管新界面的原子类；preflight 没开（见 styles/tailwind.css），
  // 现有组件样式一个字节不动。
  plugins: [
    tailwindcss(),
    {
      name: "miyin-build-info",
      closeBundle() {
        writeFileSync(
          new URL("./dist/BUILD-INFO.txt", import.meta.url),
          `build=${buildStamp}\nmode=user\n`,
          "utf8",
        );
      },
    },
  ],
  define: { __MIYIN_BUILD__: JSON.stringify(buildStamp) },
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
    // 不在这里清空：dist 里还有开发者页的产物（另一次构建写的）。
    // 要清就统一在 build 脚本开头清一次 —— 见 package.json。
    emptyOutDir: false,
    target: "chrome110",
    sourcemap: false,
  },
});
