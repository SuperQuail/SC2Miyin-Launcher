import { createRoot } from "react-dom/client";
import type { ComponentType, ReactElement } from "react";

/** 事件回给外壳。 */
export type IslandEmit = (event: string, payload?: unknown) => void;

/** 外壳拿到的把手：改 props 就 update，卸载就 teardown。 */
export interface IslandHandle {
  update: (props: Record<string, unknown>) => void;
  teardown: () => void;
}

/**
 * 把一个 React 视图挂到 DOM 节点上。
 *
 * **这是迁移期的过渡件**：Vue 外壳还在，逐个视图换成 React —— 一个视图**整块**换，
 * 不在同一个文件里混写两套框架（见 docs/frontend-architecture.md）。
 * 迁完之后外壳也换成 React，这个文件就删掉。
 *
 * 数据从 `props` 进（外壳给），动作从 `emit` 出（React 侧只认事件名，不知道外壳是谁）。
 */
export function mountIsland(
  host: HTMLElement,
  emit: IslandEmit,
  props: Record<string, unknown>,
  view: (allProps: Record<string, unknown> & { emit: IslandEmit }) => ReactElement,
): IslandHandle {
  const root = createRoot(host);
  root.render(view({ ...props, emit }));
  return {
    // props 变了就再 render 一次：React 自己 diff，不会重建整棵树
    update: (next) => root.render(view({ ...next, emit })),
    teardown: () => root.unmount(),
  };
}

/** 不需要 props 的简单视图。 */
export function mountReact(host: HTMLElement, render: () => ReactElement): () => void {
  const root = createRoot(host);
  root.render(render());
  return () => root.unmount();
}

export type IslandView = ComponentType<Record<string, unknown> & { emit: IslandEmit }>;
