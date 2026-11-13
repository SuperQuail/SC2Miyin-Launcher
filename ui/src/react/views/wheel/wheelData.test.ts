import { describe, expect, it } from "vitest";

import { pickIndex, sampleEntries, sectorCenter, shortLabel, targetRotation } from "./wheelData";

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

describe("sampleEntries", () => {
  it("不超过上限时原样返回", () => {
    const items = [1, 2, 3];
    expect(sampleEntries(items, 8, () => 0.5)).toEqual([1, 2, 3]);
  });

  it("超过上限时只取 limit 个且不重复", () => {
    const items = Array.from({ length: 20 }, (_, index) => index);
    const batch = sampleEntries(items, 8, () => 0.5);
    expect(batch).toHaveLength(8);
    expect(new Set(batch).size).toBe(8);
  });
});

describe("shortLabel", () => {
  it("按字符截断，中文不会被切坏", () => {
    expect(shortLabel("自由之翼：重生")).toBe("自由之翼…");
    expect(shortLabel("Starcraft Mass Recall")).toBe("Star…");
    expect(shortLabel("短名")).toBe("短名");
  });
});

describe("sectorCenter", () => {
  it("8 个扇区时从 22.5° 开始，每格 45°", () => {
    expect(sectorCenter(0, 8)).toBe(22.5);
    expect(sectorCenter(1, 8)).toBe(67.5);
    expect(sectorCenter(7, 8)).toBe(337.5);
    expect(sectorCenter(0, 1)).toBe(180);
  });
});

describe("targetRotation", () => {
  it("至少转 4 圈，最后落在中奖扇区中心", () => {
    expect(targetRotation(0, 0, 8)).toBe(1462.5);
    expect(targetRotation(0, 7, 8)).toBe(1777.5);
  });

  it("从任意当前角度出发，归一化后都等于扇区中心", () => {
    for (const current of [0, 17, 270, 1000]) {
      for (let winner = 0; winner < 8; winner += 1) {
        const target = targetRotation(current, winner, 8);
        expect(target).toBeGreaterThanOrEqual(current + 4 * 360);
        expect(((target % 360) + 360) % 360).toBeCloseTo(sectorCenter(winner, 8), 5);
      }
    }
  });
});
