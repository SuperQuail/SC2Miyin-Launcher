import { afterEach, describe, expect, it, vi } from "vitest";

import {
  closeContextMenu,
  contextMenuHandledRecently,
  contextMenuSnapshot,
  pickContextMenuItem,
  showContextMenu,
  subscribeContextMenu,
} from "./contextMenu";

/** 只要那四个字段 —— store 不关心事件是 React 还是原生的。 */
function fakeEvent(x: number, y: number): MouseEvent {
  return {
    clientX: x,
    clientY: y,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  } as unknown as MouseEvent;
}

afterEach(() => {
  closeContextMenu();
});

describe("右键菜单 store", () => {
  it("show 之后可见，落点就是鼠标位置，并且拦掉原生菜单", () => {
    const event = fakeEvent(12, 34);
    showContextMenu(event, [{ id: "a", label: "A" }], () => {});

    const state = contextMenuSnapshot();
    expect(state.visible).toBe(true);
    expect([state.x, state.y]).toEqual([12, 34]);
    expect(state.items).toHaveLength(1);
    expect(event.preventDefault).toHaveBeenCalled();
    // 外壳靠这个时间戳认出"组件自己处理过这次右键"
    expect(contextMenuHandledRecently()).toBe(true);
  });

  it("选中一项：先关菜单，再把 id 回给当初的调用方", () => {
    const picked = vi.fn();
    showContextMenu(fakeEvent(1, 1), [{ id: "del", label: "删除", danger: true }], picked);
    pickContextMenuItem("del");

    expect(picked).toHaveBeenCalledWith("del");
    expect(contextMenuSnapshot().visible).toBe(false);
    expect(contextMenuSnapshot().items).toEqual([]);
  });

  it("订阅者在开关时都会收到通知，退订之后不再收到", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeContextMenu(listener);

    showContextMenu(fakeEvent(5, 5), [], () => {});
    closeContextMenu();
    expect(listener).toHaveBeenCalledTimes(2);

    unsubscribe();
    showContextMenu(fakeEvent(6, 6), [], () => {});
    expect(listener).toHaveBeenCalledTimes(2);
  });
});
