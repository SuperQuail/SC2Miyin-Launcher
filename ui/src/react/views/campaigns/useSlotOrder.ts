import { useCallback, useEffect, useMemo, useState } from "react";

import { applyOrder, dropIndex, moveSlot, readSlotOrder, stepTarget, writeSlotOrder } from "../../lib/slotOrder";

/** 落点：插在这张卡的哪一边。 */
export interface DropMark {
  slug: string;
  /** 插在这格前面（false = 后面）。 */
  before: boolean;
}

export interface SlotOrderApi<T> {
  /** 排好序的列表（原数组不动）。 */
  slots: T[];
  /** 正在拖的那一格；null = 没在拖。也用来挡住"外面拖进来的文件"。 */
  dragging: string | null;
  /** 当前落点（拖到自己头上时是 null）。 */
  dropMark: DropMark | null;
  /** 开始拖 / 拖完（`null` 表示结束）。 */
  setDragging: (slug: string | null) => void;
  /** 拖动经过列表：算出落点、画插入线。`list` 是列表容器。 */
  dragOver: (event: React.DragEvent, list: HTMLElement | null) => void;
  /** 落下：把被拖的那张插到落点上并存下来。 */
  dropAt: (event: React.DragEvent, list: HTMLElement | null) => void;
}

/**
 * 拖动排序的接线（顺序本身是纯函数，见 `lib/slotOrder.ts`）。
 *
 * 顺序**只认列表上此刻的 slug**：`items` 要把当前显示的战役全给出来 ——
 * 拖动会把整个顺序重写一遍，少了谁就等于把它挤到后面去。
 */
export function useSlotOrder<T extends { slug: string }>(items: T[]): SlotOrderApi<T> {
  const slugs = useMemo(() => items.map((item) => item.slug), [items]);
  const [saved, setSaved] = useState<string[]>(() => readSlotOrder());
  const [dragging, setDragging] = useState<string | null>(null);
  const [drop, setDrop] = useState<DropMark | null>(null);
  // 刚动过的那张：DOM 重排之后把焦点还给它，键盘挪完不至于"焦点丢了"。
  const [settled, setSettled] = useState<string | null>(null);

  const slots = useMemo(() => applyOrder(items, saved), [items, saved]);

  const commit = useCallback((order: string[], slug: string) => {
    setSaved(order);
    writeSlotOrder(order);
    setSettled(slug);
  }, []);

  // 动过的那张继续拿着焦点：键盘连按 Alt+→ 才有意义（拖完也顺手回到它身上）
  useEffect(() => {
    if (!settled) return;
    document.querySelector<HTMLElement>('[data-slug="' + settled + '"]')?.focus();
    setSettled(null);
  }, [settled, slots]);

  /**
   * Alt+←/→ 在列表里挪一格。
   *
   * **必须挂在 window 的捕获阶段**：外壳的 `installWebviewGuards()`（见
   * `api/webview.ts`）把 alt+arrowleft/right 当浏览器"前进/后退"在捕获阶段拦掉了，
   * 事件根本到不了 React 的根监听 —— 挂在卡片上是个死快捷键（踩过）。
   * 守卫已经在同一个事件上放过 `preventDefault`，导航不会发生；这里只管挪格子。
   */
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.altKey) return;
      const step = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
      if (!step) return;
      const focused = document.activeElement?.closest?.("[data-slug]") as HTMLElement | null;
      const slug = focused?.dataset.slug;
      if (!slug) return;
      const from = slugs.indexOf(slug);
      const to = stepTarget(slugs, from, step);
      if (to === null) return; // 到边了就不动
      event.preventDefault();
      commit(moveSlot(slugs, from, to), slug);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [commit, slugs]);

  const dragOver = useCallback(
    (event: React.DragEvent, list: HTMLElement | null) => {
      if (!dragging) return; // 外面拖进来的文件：那是外壳的整体拖放，不接
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
      const all = Array.from(list?.querySelectorAll<HTMLElement>("[data-slug]") ?? []);
      const to = dropIndex(all, dragging, event.clientX);
      const drawn = all.filter((card) => card.dataset.slug !== dragging);
      // 第 to 格之前；到末尾就贴着最后一格的右边
      const mark: DropMark =
        to >= drawn.length
          ? { slug: drawn[drawn.length - 1]?.dataset.slug ?? dragging, before: false }
          : { slug: drawn[to].dataset.slug ?? dragging, before: true };
      setDrop((current) =>
        current && current.slug === mark.slug && current.before === mark.before ? current : mark,
      );
    },
    [dragging],
  );

  const dropAt = useCallback(
    (event: React.DragEvent, list: HTMLElement | null) => {
      if (!dragging) return;
      event.preventDefault();
      const all = Array.from(list?.querySelectorAll<HTMLElement>("[data-slug]") ?? []);
      const from = slugs.indexOf(dragging);
      const to = dropIndex(all, dragging, event.clientX);
      setDragging(null);
      setDrop(null);
      if (from >= 0) commit(moveSlot(slugs, from, to), dragging);
    },
    [commit, dragging, slugs],
  );

  return {
    slots,
    dragging,
    // 拖到自己头上没有落点可画（拖了等于白拖）
    dropMark: drop && drop.slug !== dragging ? drop : null,
    setDragging,
    dragOver,
    dropAt,
  };
}
