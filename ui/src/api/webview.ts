/**
 * 让 webview 不像浏览器 —— **页面这一层**的兜底。
 *
 * 真正的硬开关在 Rust 侧（`src-tauri/src/lib.rs` 的 `tame_webview`）：关掉 WebView2 的
 * 浏览器加速键与原生右键菜单。这里补三件事：
 * 1. 浏览器演示模式（没有 WebView2 硬开关）也拦得住；
 * 2. 硬开关够不着的窗口级组合键（Ctrl+W / Ctrl+T…）尽量拦；
 * 3. 拖文件进窗口、中键点击、Ctrl+滚轮缩放这类"一碰就露馅"的默认行为。
 *
 * 右键菜单不在这里管 —— 两个入口各自有 `onContextMenu` 兜底（AGENTS §18.2）。
 */

const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || "");

/** 要拦掉的组合：修饰键顺序固定为 ctrl → alt → shift → 键名（小写）。 */
const BLOCKED = new Set([
  "ctrl+r",
  "ctrl+shift+r",
  "f5", // 刷新
  "ctrl+p", // 打印
  "ctrl+u", // 查看源码
  "ctrl+f", // 浏览器查找 —— 开发者页要留给编辑器，用 allow 放行
  "ctrl+0",
  "ctrl+=",
  "ctrl++",
  "ctrl+-", // 缩放
  // 前进后退。**注意**：用户侧现在拿 Alt+←/→ 做战役卡片排序（views/campaigns/useSlotOrder.ts），
  // 它是在 window 捕获阶段自己接的 —— 这里用 preventDefault + stopPropagation，同一节点上后注册的
  // 监听仍会执行，所以排序能用。**别把这里改成 stopImmediatePropagation**，否则那个快捷键会变哑。
  "alt+arrowleft",
  "alt+arrowright",
  "f12",
  "ctrl+shift+i",
  "ctrl+shift+j",
  "ctrl+shift+c", // 开发者工具
  "f7", // 光标浏览
]);

/** 一次按键 → 归一化的组合串（Mac 的 ⌘ 当 ctrl 用）。 */
export function comboOf(event: KeyboardEvent): string {
  const parts: string[] = [];
  if (event.ctrlKey || (isMac && event.metaKey)) parts.push("ctrl");
  if (event.altKey) parts.push("alt");
  if (event.shiftKey) parts.push("shift");
  parts.push(event.key.toLowerCase());
  return parts.join("+");
}

/** 该不该拦。allow 里的组合原样放过（大小写不敏感）。 */
export function shouldBlock(event: KeyboardEvent, allow: string[] = []): boolean {
  if (event.defaultPrevented) return false; // 页面自己已经处理过了
  const combo = comboOf(event);
  if (allow.some((item) => item.toLowerCase() === combo)) return false;
  return BLOCKED.has(combo);
}

export type GuardOptions = {
  /** 放行的组合，例如开发者页的 `["Ctrl+F", "F12"]`（编辑器要自己的查找 / 要调工具）。 */
  allow?: string[];
};

/** 装上拦截，返回卸载函数（测试与热更新时用）。 */
export function installWebviewGuards(options: GuardOptions = {}): () => void {
  const allow = options.allow ?? [];

  const onKeyDown = (event: KeyboardEvent) => {
    if (!shouldBlock(event, allow)) return;
    event.preventDefault();
    event.stopPropagation();
  };
  // 把文件拖到窗口空白处 = 浏览器会导航走。页面自己的 drop 目标会先 preventDefault，这里只兜底。
  const onDrag = (event: DragEvent) => {
    if (!event.defaultPrevented) event.preventDefault();
  };
  // 中键点击 = 新窗口 / 新标签
  const onAuxClick = (event: MouseEvent) => {
    if (event.button !== 1) return;
    event.preventDefault();
    event.stopPropagation();
  };
  // Ctrl+滚轮 = 页面缩放
  const onWheel = (event: WheelEvent) => {
    if (event.ctrlKey) event.preventDefault();
  };

  addEventListener("keydown", onKeyDown, true);
  addEventListener("dragover", onDrag, false);
  addEventListener("drop", onDrag, false);
  addEventListener("auxclick", onAuxClick, true);
  addEventListener("wheel", onWheel, { passive: false, capture: true });

  return () => {
    removeEventListener("keydown", onKeyDown, true);
    removeEventListener("dragover", onDrag, false);
    removeEventListener("drop", onDrag, false);
    removeEventListener("auxclick", onAuxClick, true);
    removeEventListener("wheel", onWheel, { capture: true });
  };
}
