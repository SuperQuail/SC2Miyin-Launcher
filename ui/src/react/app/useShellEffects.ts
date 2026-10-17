import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import { api } from "../../api/bridge";
import { contextMenuHandledRecently, showContextMenu } from "../lib/contextMenu";
import { useAppState, type AppState } from "../state/AppState";

/** 会出水波纹的元素。加新组件时把类名补进来就行。 */
const RIPPLE_TARGETS =
  ".btn, .chip, .tab, .variant, .slot, .map, .target, .ctx__item, .group__head, .update-badge";

/**
 * 点击波纹：在按钮 / 卡片上按一下，从落点扩散一圈。
 *
 * 用一个全局监听而不是给每个组件加指令 —— 界面里的可点元素太多，
 * 与其到处挂，不如在这里按选择器统一处理，新加的组件自动就有。
 */
function spawnRipple(event: MouseEvent): void {
  const target = (event.target as HTMLElement | null)?.closest<HTMLElement>(RIPPLE_TARGETS);
  if (!target || target.hasAttribute("disabled")) return;

  // 宿主样式（定位 + 裁剪）在这里打类，免得 CSS 再抄一份选择器
  target.classList.add("ripple-host");

  const rect = target.getBoundingClientRect();
  // 直径取长边两倍，保证从任何角落点都能铺满
  const size = Math.max(rect.width, rect.height) * 2;
  const dot = document.createElement("span");
  dot.className = "ripple";
  dot.style.width = dot.style.height = size + "px";
  dot.style.left = event.clientX - rect.left - size / 2 + "px";
  dot.style.top = event.clientY - rect.top - size / 2 + "px";
  target.appendChild(dot);
  window.setTimeout(() => dot.remove(), 620);
}

/** 输入框里的剪切 / 复制 / 粘贴 / 全选。 */
async function editAction(element: HTMLElement, action: string): Promise<void> {
  const field = element as HTMLInputElement | HTMLTextAreaElement;

  if (action === "copy" || action === "cut") {
    const selected = field.value?.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0) ?? "";
    try {
      await navigator.clipboard.writeText(selected);
      if (action === "cut") {
        const start = field.selectionStart ?? 0;
        const end = field.selectionEnd ?? 0;
        field.value = field.value.slice(0, start) + field.value.slice(end);
        // 改完要通知 React —— 不然受控输入还停在旧值
        field.dispatchEvent(new Event("input", { bubbles: true }));
      }
    } catch {
      // 剪贴板被挡就算了，不弹红字
    }
    return;
  }

  if (action === "paste") {
    try {
      const text = await navigator.clipboard.readText();
      const start = field.selectionStart ?? field.value.length;
      const end = field.selectionEnd ?? field.value.length;
      field.value = field.value.slice(0, start) + text + field.value.slice(end);
      field.selectionStart = field.selectionEnd = start + text.length;
      field.dispatchEvent(new Event("input", { bubbles: true }));
    } catch {
      // 读剪贴板需要权限，被拒就安静收场
    }
    return;
  }

  if (action === "selectAll") {
    field.focus();
    field.select();
  }
}

/**
 * 外壳挂载时那几件全局的事。
 *
 * 全部用 `latest` 读当下的状态：监听器与定时器只挂一次，但里面要调的动作每次渲染
 * 都是新的（React 没有 Vue 那种"响应式 ref 自动是最新的"这回事）。
 */
export function useShellEffects(): { dragging: boolean } {
  const app = useAppState();
  const [dragging, setDragging] = useState(false);
  const latest = useRef<AppState>(app);
  latest.current = app;

  useEffect(() => {
    /**
     * 右键：**一律拦掉浏览器原生的那个**。
     *
     * WebView 的原生菜单是「刷新 / 另存为 / 检查元素」那一套 —— 放在桌面应用里
     * 格格不入。所以整个窗口一律拦掉，换成我们自己的：
     * - 组件自己处理过的（卡片、地图行…）用组件那份菜单 —— 靠时间戳认出来
     * - **输入框里给复制粘贴** —— 不然用户连粘贴个路径都做不到
     * - 其余地方落到默认菜单
     */
    const onContextMenu = (event: MouseEvent): void => {
      event.preventDefault();
      if (contextMenuHandledRecently()) return;

      const editable = (event.target as HTMLElement | null)?.closest<HTMLElement>(
        "input, textarea, [contenteditable='true']",
      );

      if (editable) {
        showContextMenu(
          event,
          [
            { id: "cut", label: "剪切" },
            { id: "copy", label: "复制" },
            { id: "paste", label: "粘贴" },
            { id: "selectAll", label: "全选", separatorBefore: true },
          ],
          (id) => void editAction(editable, id),
        );
        return;
      }

      showContextMenu(
        event,
        [
          { id: "refresh", label: "刷新数据" },
          { id: "settings", label: "设置", separatorBefore: true },
          { id: "repo", label: "项目主页" },
        ],
        (id) => {
          if (id === "refresh") void latest.current.installation.bootstrap();
          if (id === "settings") latest.current.nav.setView("settings");
          if (id === "repo") void api.openUrl("https://github.com/SuperQuail/SC2Miyin-Launcher");
        },
      );
    };

    const onMouseDown = (event: MouseEvent): void => spawnRipple(event);

    window.addEventListener("mousedown", onMouseDown);
    window.addEventListener("contextmenu", onContextMenu);

    void latest.current.installation.bootstrap();

    // 启动时先问一句"是不是还有另一份 data 要并"；排在更新检查前面，它更靠近数据安全
    const mergeTimer = setTimeout(() => void latest.current.merge.check(), 600);
    // **启动就自动扫描更新**，不需要用户手点；稍微延后，别和启动时的战役库读取抢时间
    const updateTimer = setTimeout(() => void latest.current.update.autoCheckUpdate(), 1200);
    // 可选工具默认静默安装：排在更新检查后面 —— 那更要紧
    const toolsTimer = setTimeout(() => void latest.current.ensureTools(), 2600);

    let cancelled = false;
    let stopWatching: (() => void) | null = null;
    // 浏览器演示模式没有这个 API，静默跳过
    if (latest.current.isDesktop) {
      try {
        void getCurrentWebview()
          .onDragDropEvent((event) => {
            const payload = event.payload;

            if (payload.type === "over") {
              setDragging(true);
            } else if (payload.type === "leave") {
              setDragging(false);
            } else if (payload.type === "drop") {
              setDragging(false);
              const path = payload.paths[0];
              if (!path) return;
              // 拖到哪个页面都行：直接进安装页 —— 以前是切回战役页，人在别处拖完还得自己退回去看
              latest.current.nav.openInstall();
              latest.current.library.setDroppedPackage(path);
            }
          })
          .then((unlisten) => {
            if (cancelled) unlisten();
            else stopWatching = unlisten;
          })
          .catch(() => {
            stopWatching = null;
          });
      } catch {
        stopWatching = null;
      }
    }

    return () => {
      cancelled = true;
      window.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("contextmenu", onContextMenu);
      clearTimeout(mergeTimer);
      clearTimeout(updateTimer);
      clearTimeout(toolsTimer);
      stopWatching?.();
    };
  }, []);

  return { dragging };
}
