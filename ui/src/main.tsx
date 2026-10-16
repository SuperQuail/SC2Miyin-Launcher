import { createRoot } from "react-dom/client";

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

createRoot(container).render(<App />);
