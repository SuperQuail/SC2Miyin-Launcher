import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { WheelDisc } from "./WheelDisc";
import { shortLabel, type WheelEntry, type WheelPalette } from "./wheelData";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const palette: WheelPalette = {
  background: ["#111", "#222"],
  text: "#fff",
  border: "#333",
  accent: "#0af",
  surface: "#000",
};

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function render(entries: WheelEntry[]): Promise<HTMLDivElement> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<WheelDisc entries={entries} palette={palette} />);
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
  });
  return host;
}

function entries(count: number): WheelEntry[] {
  return Array.from({ length: count }, (_, index) => ({
    id: String(index),
    label: "战役" + index,
  }));
}

/** 数字模式下 <text> 里还有 <title>；只取直接文本节点才是扇区标签。 */
function labelText(item: Element): string {
  return [...item.childNodes]
    .filter((node) => node.nodeType === 3)
    .map((node) => node.textContent ?? "")
    .join("");
}

describe("WheelDisc", () => {
  it("10 个以内继续显示短标签", async () => {
    const host = await render(entries(10));
    const labels = [...host.querySelectorAll(".wheel-disc text")].map(labelText);

    expect(labels).toEqual(entries(10).map((item) => shortLabel(item.label)));
  });

  it("超过 10 个改为编号，并保留完整名悬停标题", async () => {
    const host = await render(entries(11));
    const labels = [...host.querySelectorAll(".wheel-disc text")].map(labelText);
    const titles = [...host.querySelectorAll(".wheel-disc title")].map((item) => item.textContent);

    expect(labels).toEqual(Array.from({ length: 11 }, (_, index) => String(index + 1)));
    expect(titles).toEqual(entries(11).map((item) => item.label));
  });
});
