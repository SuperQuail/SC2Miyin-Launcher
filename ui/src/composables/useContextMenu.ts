/**
 * 右键菜单的全局状态。
 *
 * 做成模块级单例而不是每个组件各自维护：菜单同时只可能有一个，
 * 而且它必须渲染在**所有内容之上** —— 由 App.vue 挂一个 `<ContextMenu />` 即可，
 * 任何地方调 `show()` 就能弹。
 */
import { ref } from "vue";

export interface MenuItem {
  id: string;
  label: string;
  /** 危险动作（删除之类）标红。 */
  danger?: boolean;
  disabled?: boolean;
  /** 分组之间画条线。 */
  separatorBefore?: boolean;
}

const visible = ref(false);
const at = ref({ x: 0, y: 0 });
const items = ref<MenuItem[]>([]);
let handler: ((id: string) => void) | null = null;

/**
 * 上一次弹菜单的时间。
 *
 * 用途：我们在 window 上挂了一个**一律拦掉浏览器原生菜单**的监听，
 * 而组件的 `contextmenu` 会先于它触发并调 `show()`。
 * 没有这个时间戳的话，组件刚弹出来的菜单会立刻被默认菜单顶掉。
 */
let lastShownAt = 0;

/** 最近 100 毫秒内弹过菜单吗（说明已经有组件处理了这次右键）。 */
export function contextMenuHandledRecently(): boolean {
  return performance.now() - lastShownAt < 100;
}

export function useContextMenu() {
  /**
   * 在鼠标位置弹一个菜单。
   *
   * `event.preventDefault()` 是必须的 —— 不拦的话 WebView 会弹自己的
   * 「另存为 / 检查」那一套。
   */
  function show(event: MouseEvent, entries: MenuItem[], onPick: (id: string) => void): void {
    event.preventDefault();
    event.stopPropagation();

    lastShownAt = performance.now();
    at.value = { x: event.clientX, y: event.clientY };
    items.value = entries;
    handler = onPick;
    visible.value = true;
  }

  function pick(id: string): void {
    const callback = handler;
    close();
    callback?.(id);
  }

  function close(): void {
    visible.value = false;
    items.value = [];
    handler = null;
  }

  return { visible, at, items, show, pick, close };
}
