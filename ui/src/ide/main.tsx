import { createRoot } from "react-dom/client";

import { IdeApp } from "./IdeApp";
import "../styles/tokens.css";
// Ring UI 的整套样式（它是控件库，不带自己的样式就等于没装）
import "@jetbrains/ring-ui-built/components/style.css";
import "./ide.css";

/**
 * 开发者页是**独立的前端入口**（ide.html），和用户侧的 Vue 应用不共用运行时。
 * 两边只通过 IPC 说话 —— 这条边界是硬规矩，别在 Vue 里 import 这里的任何东西。
 */
const params = new URLSearchParams(location.search);
if (params.get("theme") === "dark") {
  document.documentElement.dataset.theme = "dark";
}

createRoot(document.getElementById("ide")!).render(
  <IdeApp file={params.get("file") ?? "text"} />,
);
