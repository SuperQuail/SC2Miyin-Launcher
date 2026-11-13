import { describe, expect, it } from "vitest";

import { readPalette } from "./wheelPalette";

function fakeStyle(values: Record<string, string>): CSSStyleDeclaration {
  return { getPropertyValue: (name: string) => values[name] ?? "" } as unknown as CSSStyleDeclaration;
}

describe("readPalette", () => {
  it("从设计令牌读四个颜色", () => {
    const palette = readPalette(
      fakeStyle({
        "--accent-soft": " #abc ",
        "--surface-3": "#def",
        "--on-surface": "#123",
        "--outline": "#456",
      }),
    );

    expect(palette).toEqual({
      background: ["#abc", "#def"],
      text: "#123",
      border: "#456",
    });
  });

  it("令牌缺失时用兜底，不返回空串", () => {
    const palette = readPalette(fakeStyle({}));

    expect(palette.background).toHaveLength(2);
    expect(palette.background.every(Boolean)).toBe(true);
    expect(palette.text).toBeTruthy();
    expect(palette.border).toBeTruthy();
  });
});
