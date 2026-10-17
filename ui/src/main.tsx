import { createRoot } from "react-dom/client";

import { installWebviewGuards } from "./api/webview";

import { App } from "./react/app/App";
import { applyInitialTheme } from "./react/state/useTheme";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/ripple.css";
import "./react/app/shell.css";

// 首屏主题要在 render 之前贴上去，否则会先按浅色画一帧
applyInitialTheme(location.search);

const container = document.getElementById("app");
if (!container) throw new Error("找不到挂载点 #app");

// 别让它像浏览器：拦掉刷新 / 打印 / 缩放 / 中键 / 拖放导航这些默认行为
installWebviewGuards();

createRoot(container).render(<App />);
