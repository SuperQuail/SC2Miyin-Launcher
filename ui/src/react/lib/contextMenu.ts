import type { MouseEvent as ReactMouseEvent } from "react";

/** 一条右键菜单项。 */
export interface MenuItem {
  id: string;
  label: string;
  /** 危险动作（删除之类）标红。 */
  danger?: boolean;
  disabled?: boolean;
  /** 分组之间画条线。 */
  separatorBefore?: boolean;
}

export interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  items: MenuItem[];
}

/**
 * 右键菜单的全局状态。
 *
 * 做成模块级 store 而不是每个组件各自维护：菜单同时只可能有一个，而且它必须渲染在
 * **所有内容之上** —— 由外壳挂一个 `<ContextMenu />` 即可，任何地方调 `showContextMenu()`
 * 就能弹。用 `useSyncExternalStore` 订阅，不引额外的状态库。
 */
let state: ContextMenuState = { visible: false, x: 0, y: 0, items: [] };
let handler: ((id: string) => void) | null = null;

const listeners = new Set<() => void>();

/**
 * 上一次弹菜单的时间。
 *
 * 用途：我们在 window 上挂了一个**一律拦掉浏览器原生菜单**的监听，
 * 而组件的 `contextmenu` 会先于它触发并调 `showContextMenu()`。
 * 没有这个时间戳的话，组件刚弹出来的菜单会立刻被默认菜单顶掉。
 */
let lastShownAt = 0;

/** 最近 100 毫秒内弹过菜单吗（说明已经有组件处理了这次右键）。 */
export function contextMenuHandledRecently(): boolean {
  return performance.now() - lastShownAt < 100;
}

export function subscribeContextMenu(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function contextMenuSnapshot(): ContextMenuState {
  return state;
}

/**
 * 在鼠标位置弹一个菜单。
 *
 * `event.preventDefault()` 是必须的 —— 不拦的话 WebView 会弹自己的
 * 「另存为 / 检查」那一套。
 */
export function showContextMenu(
  event: ReactMouseEvent | MouseEvent,
  items: MenuItem[],
  onPick: (id: string) => void,
): void {
  event.preventDefault();
  event.stopPropagation();

  lastShownAt = performance.now();
  handler = onPick;
  state = { visible: true, x: event.clientX, y: event.clientY, items };
  for (const listener of listeners) listener();
}

/** 选中一项：先关菜单，再回话。 */
export function pickContextMenuItem(id: string): void {
  const callback = handler;
  closeContextMenu();
  callback?.(id);
}

export function closeContextMenu(): void {
  if (!state.visible) return;
  handler = null;
  state = { visible: false, x: 0, y: 0, items: [] };
  for (const listener of listeners) listener();
}
