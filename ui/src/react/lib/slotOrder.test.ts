import { describe, expect, it } from "vitest";

import { applyOrder, dropIndex, moveSlot, stepTarget } from "./slotOrder";

/** 库里那几部官方战役（固定顺序，见 AGENTS §14.2）。 */
const official = ["wol", "hots", "lotv", "nova"].map((slug) => ({ slug }));

describe("applyOrder", () => {
  it("保存过的按保存顺序排前面，没保存过的接在后面保持原顺序", () => {
    expect(applyOrder(official, ["nova", "wol"]).map((slot) => slot.slug)).toEqual([
      "nova",
      "wol",
      "hots",
      "lotv",
    ]);
  });

  it("保存过一半、另一半是库里新来的：新来的不掉队也不插队", () => {
    // 库里从 4 部变成 5 部（多了一部自制战役那种），只记过 lotv 的位置
    const grown = [...official, { slug: "custom" }];
    expect(applyOrder(grown, ["lotv"]).map((slot) => slot.slug)).toEqual([
      "lotv",
      "wol",
      "hots",
      "nova",
      "custom",
    ]);
  });

  it("库里已经没有的 slug 直接忽略（不崩、也不影响别人）", () => {
    const shrunk = official.slice(0, 3); // nova 被删了
    expect(applyOrder(shrunk, ["nova", "lotv", "wol"]).map((slot) => slot.slug)).toEqual([
      "lotv",
      "wol",
      "hots",
    ]);
  });

  it("没保存过任何顺序 = 原样返回（不许按名字排）", () => {
    expect(applyOrder(official, []).map((slot) => slot.slug)).toEqual(["wol", "hots", "lotv", "nova"]);
  });

  it("不改原数组", () => {
    const input = [...official];
    applyOrder(input, ["nova"]);
    expect(input.map((slot) => slot.slug)).toEqual(["wol", "hots", "lotv", "nova"]);
  });
});

describe("moveSlot", () => {
  const list = ["wol", "hots", "lotv", "nova"];

  it("往后挪：放到目标位置，中间那些往前顶", () => {
    // 落点 3 = 插在 nova 前面 -> wol 落到 lotv 原来的位置
    expect(moveSlot(list, 0, 3)).toEqual(["hots", "lotv", "wol", "nova"]);
  });

  it("拖到最末尾", () => {
    expect(moveSlot(list, 0, 4)).toEqual(["hots", "lotv", "nova", "wol"]);
  });

  it("往前挪：插到目标前面", () => {
    expect(moveSlot(list, 3, 1)).toEqual(["wol", "nova", "hots", "lotv"]);
  });

  it("拖到最前面", () => {
    expect(moveSlot(list, 2, 0)).toEqual(["lotv", "wol", "hots", "nova"]);
  });

  it("原地不动", () => {
    expect(moveSlot(list, 1, 1)).toEqual(list);
    expect(moveSlot(list, 1, 2)).toEqual(list);
  });

  it("越界的 from 原样返回（不抛）", () => {
    expect(moveSlot(list, 9, 1)).toEqual(list);
  });
});

describe("stepTarget（Alt+←/→ 的落点）", () => {
  const list = ["wol", "hots", "lotv", "nova"];

  it("往右要跨过自己那一格，否则等于没动", () => {
    expect(stepTarget(list, 0, 1)).toBe(2);
    expect(moveSlot(list, 0, 2)).toEqual(["hots", "wol", "lotv", "nova"]);
  });

  it("往左插到上一格前面", () => {
    expect(stepTarget(list, 2, -1)).toBe(1);
    expect(moveSlot(list, 2, 1)).toEqual(["wol", "lotv", "hots", "nova"]);
  });

  it("到头了两边都不动", () => {
    expect(stepTarget(list, 0, -1)).toBeNull();
    expect(stepTarget(list, 3, 1)).toBeNull();
  });

  it("越界的 from 返回 null", () => {
    expect(stepTarget(list, 9, 1)).toBeNull();
  });
});

describe("dropIndex", () => {
  /** 四张 100px 宽的卡片，横着排。 */
  const cards = [0, 1, 2, 3].map((index) => {
    const box = { left: index * 100, width: 100 };
    return { dataset: { slug: "s" + index }, getBoundingClientRect: () => box } as unknown as HTMLElement;
  });

  it("落点 = 鼠标左边那几格的数量（往下拖多算一格，moveSlot 会减回来）", () => {
    expect(dropIndex(cards, "s3", 10)).toBe(0); // 最前面
    expect(dropIndex(cards, "s3", 260)).toBe(3); // s2 之后，正好是原地
    expect(dropIndex(cards, "s3", 360)).toBe(3); // 到末尾
  });

  it("被拖的那格自己不算（不然它永远挡在自己前面）", () => {
    expect(dropIndex(cards, "s0", 45)).toBe(0);
    expect(dropIndex(cards, "s0", 150)).toBe(1); // s1 之后（s0 自己那格不算）
  });
});
