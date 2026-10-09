import type { Component } from "vue";
import { createApp } from "vue";

import App from "./App.vue";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/ripple.css";

/**
 * 界面方案预览（见 AGENTS.md §17）。
 *
 * `?preview=<名字>` 会挂 `src/preview/<名字>.vue` 而不是真界面 —— 那是出方案用的
 * 一次性渲染件，**不入库**（.gitignore 里有），所以这里用 glob 而不是 import：
 * 别人克隆下来没有那个目录时，glob 匹配到空，构建照常过。
 */
type MockModule = { default: Component };

const wanted = import.meta.env.DEV
  ? new URLSearchParams(location.search).get("preview")
  : null;
const mocks = import.meta.env.DEV
  ? (import.meta.glob("./preview/*.vue") as Record<string, () => Promise<MockModule>>)
  : {};
const load = wanted ? mocks[`./preview/${wanted}.vue`] : undefined;

if (load) {
  const { default: Mock } = await load();
  createApp(Mock).mount("#app");
} else {
  createApp(App).mount("#app");
}
