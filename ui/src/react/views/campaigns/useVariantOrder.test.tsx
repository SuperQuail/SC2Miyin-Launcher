import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { VARIANT_ORDER_KEY, readVariantOrder } from "../../lib/variantOrder";
import { useVariantOrder, type VariantOrderApi } from "./useVariantOrder";

/**
 * 排序运算必须认**屏幕上此刻的顺序**，不是库里的原始顺序。
 *
 * 这个 bug 的形态很阴：先排过一次之后，两者就不一样了；这时再拖再按 Alt+→，
 * 用原始顺序算出来的 from/to 会"看着挪了、其实没动"，而且类型和纯函数测试都看不出来 ——
 * 纯函数测试里"显示的"和"原始的"永远一样。只有把 hook 挂起来才撞得到。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const items = [{ id: "a" }, { id: "b" }, { id: "c" }];
const mounted: Array<() => Promise<void>> = [];
let latest: VariantOrderApi<{ id: string }> | null = null;

beforeEach(() => {
  localStorage.clear();
  // 库里是 a,b,c；用户排过：宽屏上显示 b,a,c
  localStorage.setItem(VARIANT_ORDER_KEY, JSON.stringify({ wol: ["b", "a"], hots: ["z"] }));
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
  latest = null;
});

/** 挂一个假的列表：卡片带 data-order-id，和真界面一样。 */
async function mount(): Promise<void> {
  function Probe() {
    latest = useVariantOrder("wol", items);
    return createElement(
      "div",
      { className: "grid" },
      latest.items.map((item) =>
        createElement(
          "div",
          { key: item.id, "data-order-id": item.id },
          createElement("button", { type: "button" }, item.id),
        ),
      ),
    );
  }
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(createElement(Probe));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
}

/** 聚焦某张卡，按一次 Alt+方向键。 */
async function press(id: string, key: "ArrowLeft" | "ArrowRight"): Promise<void> {
  const button = document.querySelector<HTMLButtonElement>('[data-order-id="' + id + '"] button');
  if (!button) throw new Error("没有这张卡：" + id);
  await act(async () => {
    button.focus();
    window.dispatchEvent(new KeyboardEvent("keydown", { key, altKey: true, bubbles: true, cancelable: true }));
  });
}

describe("useVariantOrder", () => {
  it("先把记下的顺序显示出来（不是库里的原始顺序）", async () => {
    await mount();
    expect(latest?.items.map((item) => item.id)).toEqual(["b", "a", "c"]);
  });

  it("排过一次之后再按 Alt+→，挪的是屏幕上那一格", async () => {
    await mount();
    // 屏幕上 b,a,c：把 b 往右挪一格 -> a,b,c
    await press("b", "ArrowRight");
    expect(latest?.items.map((item) => item.id)).toEqual(["a", "b", "c"]);
    expect(readVariantOrder().wol).toEqual(["a", "b", "c"]);
  });

  it("往左挪同样是屏幕上的上一格", async () => {
    await mount();
    await press("a", "ArrowLeft"); // a 在 b 后面
    expect(latest?.items.map((item) => item.id)).toEqual(["a", "b", "c"]);
  });

  it("动一个槽位不碰别的槽位（各记各的）", async () => {
    await mount();
    await press("b", "ArrowRight");
    expect(readVariantOrder()).toEqual({ wol: ["a", "b", "c"], hots: ["z"] });
  });

  it("到边了不动（第一张再往左、最后一张再往右）", async () => {
    await mount();
    await press("b", "ArrowLeft"); // b 已经在最前
    expect(readVariantOrder().wol).toEqual(["b", "a"]);
  });

  it("横向落下：落点用屏幕顺序算，被拖那张不算", async () => {
    await mount();
    // 屏幕上 b,a,c：把 c 拖到最前（落点 0）
    await act(async () => {
      latest?.setDragging("c");
    });
    const list = document.querySelector<HTMLElement>(".grid");
    if (!list) throw new Error("没有列表容器");
    // 直接走落点计算：鼠标在最左边 -> 落点 0。
    // 每次 setState 之后都要重新取 latest —— 回调是那一刻的闭包，旧的还带着老的 dragging。
    const draggingApi = latest as VariantOrderApi<{ id: string }>;
    const event = { clientX: -100, clientY: 0, preventDefault: () => undefined, dataTransfer: { dropEffect: "" } };
    await act(async () => {
      draggingApi.dropAt(event as unknown as React.DragEvent, list);
    });
    expect(latest?.items.map((item) => item.id)).toEqual(["c", "b", "a"]);
    expect(readVariantOrder().wol).toEqual(["c", "b", "a"]);
  });
});
