import { createRoot } from "react-dom/client";
import type { ReactElement } from "react";

/**
 * 把一个 React 视图挂到 DOM 节点上，返回卸载函数。
 *
 * **这是迁移期的过渡件**：Vue 外壳还在，逐个视图换成 React —— 一个视图**整块**换，
 * 不在同一个文件里混写两套框架（见 docs/frontend-architecture.md）。
 * 迁完之后外壳也换成 React，这个文件就删掉。
 */
export function mountReact(host: HTMLElement, render: () => ReactElement): () => void {
  const root = createRoot(host);
  root.render(render());
  return () => root.unmount();
}
