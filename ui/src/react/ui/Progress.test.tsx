import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Progress } from "./Progress";

function render(props: Parameters<typeof Progress>[0]): HTMLElement {
  const host = document.createElement("div");
  host.innerHTML = renderToStaticMarkup(createElement(Progress, props));
  const bar = host.querySelector("[role=progressbar]");
  if (!bar) throw new Error("Progress 没有渲染出 progressbar");
  return bar as HTMLElement;
}

describe("Progress", () => {
  it("把百分比写进 aria 与填充宽度", () => {
    const bar = render({ value: 31, max: 100 });
    expect(bar.getAttribute("aria-valuenow")).toBe("31");
    expect(bar.getAttribute("aria-valuemax")).toBe("100");
    expect((bar.querySelector(".progress__bar") as HTMLElement).style.width).toBe("31%");
  });

  it("越界会夹到 0..max", () => {
    expect(render({ value: 999, max: 100 }).getAttribute("aria-valuenow")).toBe("100");
    expect(render({ value: -5, max: 100 }).getAttribute("aria-valuenow")).toBe("0");
  });

  it("value 为 null 时是不确定态：没有 aria-valuenow，条会来回跑", () => {
    const bar = render({ value: null });
    expect(bar.getAttribute("aria-valuenow")).toBeNull();
    expect(bar.className).toContain("progress--indeterminate");
  });

  it("label / detail 一起渲染", () => {
    const bar = render({ value: 12, max: 2331, label: "正在打包", detail: "12 / 2331" });
    expect(bar.querySelector(".progress__label")?.textContent).toBe("正在打包");
    expect(bar.querySelector(".progress__detail")?.textContent).toBe("12 / 2331");
  });
});