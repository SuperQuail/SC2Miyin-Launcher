import { describe, expect, it } from "vitest";

import { SAMPLE_ENTRIES, pickIndex, shortLabel, toWheelData } from "./wheelData";

describe("pickIndex", () => {
  it("落在 [0, count-1]，注入越界值也不出去", () => {
    expect(pickIndex(0, 8)).toBe(0);
    expect(pickIndex(0.999, 8)).toBe(7);
    expect(pickIndex(1, 8)).toBe(7);
    expect(pickIndex(-0.5, 8)).toBe(0);
    expect(pickIndex(Number.NaN, 8)).toBe(0);
  });

  it("没有候选时返回 0，不抛异常", () => {
    expect(pickIndex(0.5, 0)).toBe(0);
  });
});

describe("shortLabel", () => {
  it("按字符截断，中文不会被切坏", () => {
    expect(shortLabel("自由之翼：重生")).toBe("自由之翼…");
    expect(shortLabel("Starcraft Mass Recall")).toBe("Star…");
    expect(shortLabel("短名")).toBe("短名");
  });
});

describe("toWheelData", () => {
  it("数量、交替底色、文字色都对，且不写 per-item 字体", () => {
    const palette = { background: ["#aaa", "#bbb"], text: "#111", border: "#222" };
    const data = toWheelData(SAMPLE_ENTRIES, palette);

    expect(data).toHaveLength(SAMPLE_ENTRIES.length);
    expect(data[0].option).toBe("自由之翼…");
    expect(data[0].style?.backgroundColor).toBe("#aaa");
    expect(data[1].style?.backgroundColor).toBe("#bbb");
    expect(data[2].style?.backgroundColor).toBe("#aaa");
    expect(data[0].style?.textColor).toBe("#111");
    expect("fontFamily" in (data[0].style ?? {})).toBe(false);
  });
});
