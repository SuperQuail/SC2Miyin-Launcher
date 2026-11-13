import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

const captured = vi.hoisted(() => ({ props: null as Record<string, unknown> | null }));

vi.mock("react-custom-roulette", () => ({
  Wheel: (props: Record<string, unknown>) => {
    captured.props = props;
    return null;
  },
}));

import { WheelView } from "./WheelView";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mounted: Array<() => Promise<void>> = [];
const originalMatchMedia = Object.getOwnPropertyDescriptor(window, "matchMedia");

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
  captured.props = null;
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

const TWO = [
  { id: "a", label: "甲" },
  { id: "b", label: "乙" },
];

/** 页面上还有「换指针图片」按钮，测试只认抽奖那颗。 */
function spinButton(host: HTMLElement): HTMLButtonElement {
  const button = host.querySelector<HTMLButtonElement>(".wheel-page__actions button");
  if (!button) throw new Error("没有抽奖按钮");
  return button;
}

describe("WheelView", () => {
  it("点开始抽会选中注入的候选，并把库要的参数交出去", async () => {
    const host = await renderView({ entries: TWO, random: () => 0.99 });
    const button = spinButton(host);

    expect(captured.props?.mustStartSpinning).toBe(false);

    await act(async () => {
      button.click();
    });

    expect(captured.props?.prizeNumber).toBe(1);
    expect(captured.props?.mustStartSpinning).toBe(true);
    expect(captured.props?.disableInitialAnimation).toBe(true);
    expect(captured.props?.fontFamily).toBe("sans-serif");
    expect((captured.props?.backgroundColors as string[]).length).toBe(2);
    // 边缘指针必须藏掉：圆心指针是转子外面自己画的
    expect((captured.props?.pointerProps as { style?: { display?: string } } | undefined)?.style?.display).toBe("none");
    expect(button.disabled).toBe(true);
  });

  it("库报告转完后显示结果、高亮候选并恢复按钮", async () => {
    const host = await renderView({ entries: TWO, random: () => 0.99 });
    const button = spinButton(host);

    await act(async () => {
      button.click();
    });
    await act(async () => {
      (captured.props?.onStopSpinning as () => void)();
    });

    expect(host.querySelector('[role="status"]')?.textContent).toContain("乙");
    expect(host.querySelector(".wheel-candidates__item--on")?.textContent).toBe("乙");
    expect(button.disabled).toBe(false);
    expect(button.textContent).toContain("再抽一次");
  });

  it("减少动态效果时用库允许的最小转速", async () => {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: () => ({ matches: true }),
    });

    await renderView({ entries: TWO });

    expect(captured.props?.spinDuration).toBe(0.01);
  });

  it("可以换成自定义指针图片（传 pointerSrc 就渲染 img）", async () => {
    const host = await renderView({ entries: TWO, pointerSrc: "data:image/svg+xml,%3Csvg/%3E" });

    const img = host.querySelector("img.wheel-needle__img");
    expect(img?.getAttribute("src")).toBe("data:image/svg+xml,%3Csvg/%3E");
    expect(host.querySelector(".wheel-needle svg")).toBeNull();
  });

  it("少于两个候选时不渲染轮盘、按钮禁用", async () => {
    const host = await renderView({ entries: [{ id: "a", label: "甲" }] });

    expect(captured.props).toBeNull();
    expect(spinButton(host).disabled).toBe(true);
  });
});
