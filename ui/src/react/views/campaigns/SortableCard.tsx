import type { DragEvent as ReactDragEvent, ReactNode } from "react";

import type { DropMark } from "./useVariantOrder";
import "./sortable.css";

/**
 * 可拖动排序的卡片外壳（版本卡片用）。
 *
 * 包一层而不是改 `components/VariantCard.tsx`：拖动是**列表**才有的事，
 * 卡片本身在别处也要用；「原版战役」那张固定卡就故意不套它 —— 拖不动。
 *
 * 三层反馈：拖动中半透明、落点画一条竖线、鼠标落在哪半边决定插前还是插后。
 * 键盘（Alt+←/→）在 `useVariantOrder` 里统一接（那里才躲得开外壳的按键守卫）。
 */
export function SortableCard({
  id,
  dragging,
  dropMark,
  onDragState,
  onDragOver,
  onDrop,
  children,
}: {
  /** 版本 id：排序、落点、焦点都认它。 */
  id: string;
  /** 正在拖的那一格；用来认自己是不是被拖的那张。 */
  dragging: string | null;
  /** 当前落点（列表级的，只有一条）。 */
  dropMark: DropMark | null;
  /** 开始拖 / 拖完（`null` 结束）。 */
  onDragState: (id: string | null) => void;
  onDragOver: (event: ReactDragEvent, list: HTMLElement | null) => void;
  onDrop: (event: ReactDragEvent, list: HTMLElement | null) => void;
  children: ReactNode;
}) {
  const isDragging = dragging === id;
  const mark = dropMark && dropMark.id === id ? dropMark : null;

  /** 列表容器（落点要按整排卡片算）。 */
  function listOf(event: ReactDragEvent): HTMLElement | null {
    return (event.currentTarget as HTMLElement).parentElement;
  }

  return (
    <div
      className={
        "sortable" +
        (isDragging ? " sortable--dragging" : "") +
        (mark ? " sortable--drop-" + (mark.before ? "before" : "after") : "")
      }
      data-order-id={id}
      title="拖动调整顺序，也可以按 Alt+← / Alt+→"
      onDragStart={(event) => {
        event.dataTransfer.effectAllowed = "move";
        // 不清空的话，Chromium 起手不认这次拖动
        event.dataTransfer.setData("text/plain", id);
        onDragState(id);
      }}
      onDragEnd={() => onDragState(null)}
      onDragOver={(event) => onDragOver(event, listOf(event))}
      onDrop={(event) => onDrop(event, listOf(event))}
    >
      {children}
    </div>
  );
}
