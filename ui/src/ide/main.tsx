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
/**
 * 主题跟启动器走：读同一份 localStorage（`miyin.theme`：system / light / dark），
 * 「跟随系统」在两边都是默认。`?theme=dark|light` 只强制这一次（截图与排查用）。
 *
 * 两套开关都要打：我们的令牌走 `data-theme`，Ring UI 的控件走它自己的类名。
 */
const THEME_KEY = "miyin.theme";
const params = new URLSearchParams(location.search);
const forced = params.get("theme");
const prefersDark = matchMedia("(prefers-color-scheme: dark)");

function storedMode(): string {
  try {
    return localStorage.getItem(THEME_KEY) ?? "system";
  } catch {
    return "system";
  }
}

function paint(): void {
  const mode = forced === "dark" || forced === "light" ? forced : storedMode();
  const dark = mode === "dark" || (mode === "system" && prefersDark.matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.classList.toggle("ring-ui-theme-dark", dark);
}

paint();
// 启动器那边改了主题、或者系统换了深浅色，这里都跟着变
addEventListener("storage", (event) => {
  if (event.key === THEME_KEY) paint();
});
prefersDark.addEventListener("change", paint);

// 容器不存在就什么都不做 —— 这一页只该在 ide.html 里跑。
// （曾经因为两个入口共用一个构建，用户侧也执行到了这里，白屏。）
const host = document.getElementById("ide");
if (host) {
  createRoot(host).render(<IdeApp />);
}
