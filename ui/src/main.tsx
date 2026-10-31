import { createRoot } from "react-dom/client";

import { installWebviewGuards } from "./api/webview";

import { App } from "./react/app/App";
import { installStartupGuards } from "./react/app/startup";
import { applyInitialTheme } from "./react/state/useTheme";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/tailwind.css";
import "./styles/ripple.css";
import "./react/app/shell.css";

// 先装兜底：下面任何一步炸掉（包括 render 里抛），用户看到的都不该是一片白
installStartupGuards();

// 首屏主题要在 render 之前贴上去，否则会先按浅色画一帧
applyInitialTheme(location.search);

const container = document.getElementById("app");
if (!container) throw new Error("找不到挂载点 #app");

// index.html 里那段静态占位（「正在启动…」）该让位了 —— 留着会被 React 顶到下面
container.textContent = "";

// 别让它像浏览器：拦掉刷新 / 打印 / 缩放 / 中键 / 拖放导航这些默认行为
installWebviewGuards();

createRoot(container).render(<App />);
