import { fileURLToPath } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

// 前端单元测试。跑的是组件**接线**这类东西 ——
// 「emit 了但父组件没接」这种 bug 编译和类型都看不出来。
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    // 组件里有 Tauri API，测试里由各用例自己 mock
    globals: false,
  },
});
