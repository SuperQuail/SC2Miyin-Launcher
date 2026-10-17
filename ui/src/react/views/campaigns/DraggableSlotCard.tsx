import type { DragEvent as ReactDragEvent } from "react";

import type { SlotView } from "../../../api/types";
import { SlotCard } from "../../components/SlotCard";
import type { DropMark } from "./useSlotOrder";

/**
 * 可拖动排序的战役卡片。
 *
 * 包一层而不是改 `components/SlotCard.tsx`：那张卡开发者房间里也要用（AGENTS §18.1），
 * 拖动是**列表**才有的事，不该长在卡片自己身上。
 *
 * 三层反馈：拖动中半透明、落点画一条竖线、鼠标落点在哪半边决定插前还是插后。
 * 键盘（Alt+←/→）在 `useSlotOrder` 里统一接（那里才躲得开外壳的按键守卫）。
 */
export function DraggableSlotCard({
  slot,
  dragging,
  dropMark,
  onDragState,
  onDragOver,
  onDrop,
  onOpen,
}: {
  slot: SlotView;
  /** 正在拖的那一格；用来认自己是不是被拖的那张。 */
  dragging: string | null;
  /** 当前落点（列表级的，只有一条）。 */
  dropMark: DropMark | null;
  /** 开始拖 / 拖完（`null` 结束）。 */
  onDragState: (slug: string | null) => void;
  onDragOver: (event: ReactDragEvent, list: HTMLElement | null) => void;
  onDrop: (event: ReactDragEvent, list: HTMLElement | null) => void;
  onOpen: (slug: string) => void;
}) {
  const isDragging = dragging === slot.slug;
  const mark = dropMark && dropMark.slug === slot.slug ? dropMark : null;

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
      data-slug={slot.slug}
      title="拖我换位置，也可以用 Alt+← / Alt+→"
      onDragStart={(event) => {
        event.dataTransfer.effectAllowed = "move";
        // 不清空的话，Chromium 起手不认这次拖动
        event.dataTransfer.setData("text/plain", slot.slug);
        onDragState(slot.slug);
      }}
      onDragEnd={() => onDragState(null)}
      onDragOver={(event) => onDragOver(event, listOf(event))}
      onDrop={(event) => onDrop(event, listOf(event))}
    >
      <SlotCard slot={slot} onOpen={onOpen} />
    </div>
  );
}
