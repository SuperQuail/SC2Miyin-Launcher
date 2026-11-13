import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { WheelView } from "./WheelView";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mounted: Array<() => Promise<void>> = [];
const originalMatchMedia = Object.getOwnPropertyDescriptor(window, "matchMedia");

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
  vi.useRealTimers();
  if (originalMatchMedia) Object.defineProperty(window, "matchMedia", originalMatchMedia);
  else delete (window as unknown as { matchMedia?: unknown }).matchMedia;
});

async function renderView(props: Parameters<typeof WheelView>[0] = {}): Promise<HTMLDivElement> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<WheelView {...(props ?? {})} />);
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
  });
  return host;
}

function spinButton(host: HTMLElement): HTMLButtonElement {
  const button = host.querySelector<HTMLButtonElement>(".wheel-page__actions button");
  if (!button) throw new Error("没有抽奖按钮");
  return button;
}

function pointer(host: HTMLElement): HTMLElement {
  const element = host.querySelector<HTMLElement>(".wheel-pointer");
  if (!element) throw new Error("没有指针");
  return element;
}

const TWO = [
  { id: "a", label: "甲" },
  { id: "b", label: "乙" },
];

describe("WheelView", () => {
  it("轮盘是静止的 SVG；点开始转只转指针，4 圈后指向中奖扇区", async () => {
    const host = await renderView({ entries: TWO, random: () => 0.99 });
    const button = spinButton(host);

    expect(host.querySelector(".wheel-disc svg")).not.toBeNull();
    expect(host.querySelectorAll(".wheel-disc path")).toHaveLength(2);
    expect(host.querySelector(".wheel-disc")?.getAttribute("style") ?? "").not.toContain("transform");

    await act(async () => {
      button.click();
    });

    // 2 个扇区，winner=1 的中心在 270°；4 圈后是 1710°
    expect(pointer(host).style.transform).toBe("rotate(1530deg)");
    expect(button.disabled).toBe(true);

    await act(async () => {
      vi.advanceTimersByTime(4100);
    });

    expect(pointer(host).style.transform).toBe("rotate(90deg)");
  });

  it("转完后显示结果、高亮候选并恢复按钮", async () => {
    const host = await renderView({ entries: TWO, random: () => 0.99 });
    const button = spinButton(host);

    await act(async () => {
      button.click();
    });
    await act(async () => {
      vi.advanceTimersByTime(4100);
    });

    expect(host.querySelector('[role="status"]')?.textContent).toContain("乙");
    expect(host.querySelector(".wheel-candidates__item--on")?.textContent).toBe("乙");
    expect(button.disabled).toBe(false);
    expect(button.textContent).toContain("再转一次");
  });

  it("减少动态效果时不做长动画，很快出结果", async () => {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: () => ({ matches: true }),
    });

    const host = await renderView({ entries: TWO, random: () => 0 });
    const button = spinButton(host);

    await act(async () => {
      button.click();
    });
    await act(async () => {
      vi.advanceTimersByTime(70);
    });

    expect(host.querySelector('[role="status"]')?.textContent).toContain("甲");
  });

  it("可以换成自定义指针图片（传 pointerSrc 就渲染 img）", async () => {
    const host = await renderView({ entries: TWO, pointerSrc: "data:image/svg+xml,%3Csvg/%3E" });

    const img = host.querySelector("img.wheel-pointer__img");
    expect(img?.getAttribute("src")).toBe("data:image/svg+xml,%3Csvg/%3E");
    expect(host.querySelector(".wheel-pointer svg")).toBeNull();
  });

  it("默认用大主教指针图", async () => {
    const host = await renderView({ entries: TWO });

    const img = host.querySelector("img.wheel-pointer__img");
    expect(img?.getAttribute("src")).toBe("/wheel/dajiaozhu-pointer.png");
  });

  it("少于两个候选时按钮禁用", async () => {
    const host = await renderView({ entries: [{ id: "a", label: "甲" }] });

    expect(spinButton(host).disabled).toBe(true);
  });
});
