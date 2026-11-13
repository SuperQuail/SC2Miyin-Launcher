import { describe, expect, it } from "vitest";

import { labelPlacement, sectorPath, WHEEL_CENTER, WHEEL_OUTER, WHEEL_VIEW } from "./wheelGeometry";

describe("sectorPath", () => {
  it("n=1..12 的每个扇区都输出合法路径", () => {
    for (let count = 1; count <= 12; count += 1) {
      for (let index = 0; index < count; index += 1) {
        const path = sectorPath(index, count);
        expect(path.startsWith("M ")).toBe(true);
        expect(path.endsWith("Z")).toBe(true);
        expect(path).not.toContain("NaN");
        expect(path).not.toContain("Infinity");
      }
    }
  });

  it("n=1 是整圆，n=8 相邻扇区路径不同", () => {
    expect(sectorPath(0, 1).match(/A /g)?.length).toBe(2);
    expect(sectorPath(0, 8)).not.toBe(sectorPath(1, 8));
  });
});

describe("labelPlacement", () => {
  it("标签落在轮盘内，且左半圈会翻转 180°", () => {
    for (let index = 0; index < 8; index += 1) {
      const label = labelPlacement(index, 8);
      expect(label.x).toBeGreaterThan(0);
      expect(label.x).toBeLessThan(WHEEL_VIEW);
      expect(label.y).toBeGreaterThan(0);
      expect(label.y).toBeLessThan(WHEEL_VIEW);
    }

    expect(labelPlacement(0, 8).flipped).toBe(false);
    expect(labelPlacement(2, 8).flipped).toBe(true);
    expect(labelPlacement(4, 8).flipped).toBe(true);
    expect(labelPlacement(6, 8).flipped).toBe(false);
  });

  it("轮盘中心与外径是常量，便于其他模块引用", () => {
    expect(WHEEL_CENTER).toBe(50);
    expect(WHEEL_OUTER).toBe(46);
  });
});
