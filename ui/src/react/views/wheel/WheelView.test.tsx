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
  it("轮盘静止；点开始转只转指针，4 圈后指向中奖扇区", async () => {
    const host = await renderView({ entries: TWO, random: () => 0.99 });
    const button = spinButton(host);

    expect(host.querySelector(".wheel-disc svg")).not.toBeNull();
    expect(host.querySelector(".wheel-disc")?.getAttribute("style") ?? "").not.toContain("transform");

    await act(async () => {
      button.click();
    });

    // 2 个扇区，winner=1 的中心在 270°；4 圈后是 1710°，再减图片朝向的 180°
    expect(pointer(host).style.transform).toBe("rotate(1530deg)");
    expect(button.disabled).toBe(true);

    await act(async () => {
      vi.advanceTimersByTime(4100);
    });

    expect(pointer(host).style.transform).toBe("rotate(90deg)");
    expect(button.disabled).toBe(false);
  });

  it("页面只有标题、来源子菜单、轮盘与抽取按钮", async () => {
    const host = await renderView({ entries: TWO });

    expect(host.querySelector("h2")?.textContent).toBe("战役转盘");
    expect(host.querySelector("img.wheel-pointer__img")?.getAttribute("src")).toBe(
      "/wheel/dajiaozhu-pointer.png",
    );
    expect(
      [...host.querySelectorAll(".wheel-subnav button")].map((button) => button.textContent),
    ).toEqual(["已有战役", "战役数据库"]);
    expect(host.querySelectorAll("button")).toHaveLength(3);
    expect(spinButton(host).textContent).toBe("抽取");
  });

  it("子菜单可在已有战役与战役数据库之间切换", async () => {
    const host = await renderView({ entries: TWO });
    const tabs = [...host.querySelectorAll<HTMLButtonElement>(".wheel-subnav button")];

    await act(async () => {
      tabs[1].click();
    });

    expect(tabs[1].getAttribute("aria-selected")).toBe("true");
    expect(tabs[1].classList.contains("wheel-subnav__item--active")).toBe(true);
    expect(host.querySelector(".wheel-disc svg")).not.toBeNull();
  });

  it("减少动态效果时不做长动画，很快恢复按钮", async () => {
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

    expect(button.disabled).toBe(false);
  });

  it("少于两个候选时按钮禁用", async () => {
    const host = await renderView({ entries: [{ id: "a", label: "甲" }] });

    expect(spinButton(host).disabled).toBe(true);
  });
});
