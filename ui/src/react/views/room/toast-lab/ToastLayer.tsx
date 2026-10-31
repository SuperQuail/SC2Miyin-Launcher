import { createPortal } from "react-dom";
import type { ReactNode } from "react";

/**
 * 候选都挂到 `document.body` 上 —— 房间只是"调用方"，提示本身不该待在页面里。
 *
 * **为什么必须这样（这个坑值得记一笔）**：外层 `.view` 带着
 * `animation: view-in ... both`，动画放完后计算值是 `transform: matrix(1,0,0,1,0,0)`
 * —— 只要不是 `none`，它就成了 `position: fixed` 的包含块，于是 `bottom: 20px`
 * 是相对那个 3568px 高的容器算的，提示直接掉到屏幕外（实测 top=3556）。
 * 生产那个 Toast 之所以没事，是因为它挂在 AppShell 上、根本不在 `.view` 里。
 *
 * shortcut: 返回值做了个类型断言 —— 树里有个 `@types/react@19`（react-dom 的
 * `@types/react: "*"` 拉进来的），它的 `ReactPortal` 和 18 的不是同一个类型。
 * 等 ui 的依赖对齐（把 `@types/react` pin 成 18）之后这行断言就能删。
 */
export function ToastLayer({ children }: { children: ReactNode }) {
  if (typeof document === "undefined") return null;
  return createPortal(children, document.body) as unknown as ReactNode;
}
