import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  applyOrder,
  dropIndex,
  moveSlot,
  orderFor,
  readVariantOrder,
  stepTarget,
  withOrder,
  writeVariantOrder,
} from "../../lib/variantOrder";

/** 落点：插在这张卡的哪一边。 */
export interface DropMark {
  id: string;
  /** 插在这格前面（false = 后面）。 */
  before: boolean;
}

export interface VariantOrderApi<T> {
  /** 排好序的版本卡片（原数组不动）。 */
  items: T[];
  /** 正在拖的那一格；null = 没在拖。也用来挡住"外面拖进来的文件"。 */
  dragging: string | null;
  /** 当前落点（拖到自己头上时是 null）。 */
  dropMark: DropMark | null;
  /** 开始拖 / 拖完（`null` 表示结束）。 */
  setDragging: (id: string | null) => void;
  /** 拖动经过列表：算出落点、画插入线。`list` 是列表容器。 */
  dragOver: (event: React.DragEvent, list: HTMLElement | null) => void;
  /** 落下：把被拖的那张插到落点上并存下来。 */
  dropAt: (event: React.DragEvent, list: HTMLElement | null) => void;
}

/**
 * 版本卡片的拖动排序（顺序本身是纯函数，见 `lib/variantOrder.ts`）。
 *
 * 顺序**按槽位分开存**：一个战役一个顺序，互不影响。`items` 要把这个槽位此刻显示的
 * 版本**全给出来** —— 拖动会把整个顺序重写一遍，少了谁就等于把它挤到后面去。
 *
 * 「原版战役」那张固定卡不归这里管：它没有 `data-order-id`，所以既拖不动、
 * 也不会把落点算偏（`dropIndex` 只认带这个属性的卡片）。
 */
export function useVariantOrder<T extends { id: string }>(slot: string, items: T[]): VariantOrderApi<T> {
  const [map, setMap] = useState<Record<string, string[]>>(() => readVariantOrder());
  const [dragging, setDragging] = useState<string | null>(null);
  const [drop, setDrop] = useState<DropMark | null>(null);
  // 刚动过的那张：DOM 重排之后把焦点还给它，键盘连按 Alt+→ 才接得上。
  const [settled, setSettled] = useState<string | null>(null);

  const saved = useMemo(() => orderFor(map, slot), [map, slot]);
  const ordered = useMemo(() => applyOrder(items, saved), [items, saved]);
  /**
   * **屏幕上此刻的顺序** —— 排序运算只认它。
   *
   * 不能拿 `items`（那是库里的原始顺序）算：一旦用户排过一次，两者就不一样了，
   * 按原始顺序去算 `from`/`to` 会"看着挪了、其实没动"（踩过）。
   */
  const shown = useMemo(() => ordered.map((item) => item.id), [ordered]);

  // 写盘要拿"当下的那张表"：commit 里读 state 会读到渲染时那一份旧的
  const mapRef = useRef(map);
  mapRef.current = map;

  const commit = useCallback(
    (order: string[], id: string) => {
      const next = withOrder(mapRef.current, slot, order);
      mapRef.current = next;
      setMap(next);
      writeVariantOrder(next);
      setSettled(id);
    },
    [slot],
  );

  useEffect(() => {
    if (!settled) return;
    document.querySelector<HTMLElement>('[data-order-id="' + settled + '"]')?.focus();
    setSettled(null);
  }, [settled, ordered]);

  /**
   * Alt+←/→ 在列表里挪一格。
   *
   * **必须挂在 window 的捕获阶段**：外壳的 `installWebviewGuards()`（见
   * `api/webview.ts`）把 alt+arrowleft/right 当浏览器"前进/后退"在捕获阶段拦掉了，
   * 事件根本到不了 React 的根监听 —— 挂在卡片上是个死快捷键（踩过）。
   * 守卫已经 preventDefault，导航不会发生；这里只管挪格子。
   */
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.altKey) return;
      const step = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
      if (!step) return;
      const focused = document.activeElement?.closest?.("[data-order-id]") as HTMLElement | null;
      const id = focused?.dataset.orderId;
      if (!id) return;
      const from = shown.indexOf(id);
      const to = stepTarget(shown, from, step);
      if (to === null) return; // 到边了就不动
      event.preventDefault();
      commit(moveSlot(shown, from, to), id);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [commit, shown]);

  const cardsOf = (list: HTMLElement | null) =>
    Array.from(list?.querySelectorAll<HTMLElement>("[data-order-id]") ?? []);

  const dragOver = useCallback(
    (event: React.DragEvent, list: HTMLElement | null) => {
      if (!dragging) return; // 外面拖进来的文件：那是外壳的整体拖放，不接
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
      const all = cardsOf(list);
      const to = dropIndex(all, dragging, event.clientX);
      const drawn = all.filter((card) => card.dataset.orderId !== dragging);
      // 第 to 格之前；到末尾就贴着最后一格的右边
      const mark: DropMark =
        to >= drawn.length
          ? { id: drawn[drawn.length - 1]?.dataset.orderId ?? dragging, before: false }
          : { id: drawn[to].dataset.orderId ?? dragging, before: true };
      setDrop((current) =>
        current && current.id === mark.id && current.before === mark.before ? current : mark,
      );
    },
    [dragging],
  );

  const dropAt = useCallback(
    (event: React.DragEvent, list: HTMLElement | null) => {
      if (!dragging) return;
      event.preventDefault();
      const from = shown.indexOf(dragging);
      const to = dropIndex(cardsOf(list), dragging, event.clientX);
      setDragging(null);
      setDrop(null);
      if (from >= 0) commit(moveSlot(shown, from, to), dragging);
    },
    [commit, dragging, shown],
  );

  return {
    items: ordered,
    dragging,
    // 拖到自己头上没有落点可画（拖了等于白拖）
    dropMark: drop && drop.id !== dragging ? drop : null,
    setDragging,
    dragOver,
    dropAt,
  };
}
