import { beforeEach, describe, expect, it } from "vitest";

import {
  VARIANT_ORDER_KEY,
  applyOrder,
  dropIndex,
  moveSlot,
  orderFor,
  readVariantOrder,
  stepTarget,
  withOrder,
  writeVariantOrder,
} from "./variantOrder";

/** 某个战役下的玩家版本（顺序就是界面显示的顺序）。 */
const variants = ["v1", "v2", "v3", "v4"].map((id) => ({ id }));

describe("applyOrder", () => {
  it("记过的按记下的顺序排前面，没记过的接在后面保持原顺序", () => {
    expect(applyOrder(variants, ["v4", "v1"]).map((item) => item.id)).toEqual([
      "v4",
      "v1",
      "v2",
      "v3",
    ]);
  });

  it("新导入的版本不掉队也不插队", () => {
    const grown = [...variants, { id: "v5" }];
    expect(applyOrder(grown, ["v3"]).map((item) => item.id)).toEqual([
      "v3",
      "v1",
      "v2",
      "v4",
      "v5",
    ]);
  });

  it("库里已经没有的 id 直接忽略（删版本之后不会乱）", () => {
    const shrunk = variants.slice(0, 3);
    expect(applyOrder(shrunk, ["v4", "v3", "v1"]).map((item) => item.id)).toEqual([
      "v3",
      "v1",
      "v2",
    ]);
  });

  it("没记过 = 原样返回", () => {
    expect(applyOrder(variants, []).map((item) => item.id)).toEqual(["v1", "v2", "v3", "v4"]);
  });

  it("不改原数组", () => {
    const input = [...variants];
    applyOrder(input, ["v4"]);
    expect(input.map((item) => item.id)).toEqual(["v1", "v2", "v3", "v4"]);
  });
});

describe("按槽位分开存", () => {
  beforeEach(() => localStorage.clear());

  it("一个槽位的顺序动不了别的槽位", () => {
    const map = withOrder(withOrder({}, "wol", ["a", "b"]), "hots", ["c"]);
    const next = withOrder(map, "wol", ["b", "a"]);
    expect(orderFor(next, "wol")).toEqual(["b", "a"]);
    expect(orderFor(next, "hots")).toEqual(["c"]);
  });

  it("没记过的槽位是空的（不是别人的顺序）", () => {
    expect(orderFor({ custom: ["x"] }, "lotv")).toEqual([]);
  });

  it("存下去再读回来还是同一张表", () => {
    writeVariantOrder({ wol: ["b", "a"], custom: ["z"] });
    expect(readVariantOrder()).toEqual({ wol: ["b", "a"], custom: ["z"] });
  });

  it("存坏了 / 结构不对就当没排过（不崩）", () => {
    localStorage.setItem(VARIANT_ORDER_KEY, "not json");
    expect(readVariantOrder()).toEqual({});
    localStorage.setItem(VARIANT_ORDER_KEY, JSON.stringify(["wol"]));
    expect(readVariantOrder()).toEqual({});
    localStorage.setItem(VARIANT_ORDER_KEY, JSON.stringify({ wol: "v1", hots: ["v2", 7] }));
    expect(readVariantOrder()).toEqual({ hots: ["v2"] });
  });
});

describe("moveSlot（to 是「去掉自己之后」的下标，和 dropIndex 同口径）", () => {
  const list = ["v1", "v2", "v3", "v4"];

  it("插到第 3 格 = 排到最后", () => {
    expect(moveSlot(list, 0, 3)).toEqual(["v2", "v3", "v4", "v1"]);
  });

  it("往后挪一格 = 和下一张换位", () => {
    expect(moveSlot(list, 1, 2)).toEqual(["v1", "v3", "v2", "v4"]);
  });

  it("往前挪一格 = 和上一张换位", () => {
    expect(moveSlot(list, 2, 1)).toEqual(["v1", "v3", "v2", "v4"]);
  });

  it("拖到最前面", () => {
    expect(moveSlot(list, 2, 0)).toEqual(["v3", "v1", "v2", "v4"]);
  });

  it("原地不动", () => {
    expect(moveSlot(list, 1, 1)).toEqual(list);
  });

  it("越界的 from / to 都不抛", () => {
    expect(moveSlot(list, 9, 1)).toEqual(list);
    expect(moveSlot(list, 0, 99)).toEqual(["v2", "v3", "v4", "v1"]);
  });
});

describe("拖动落点：落点线画在哪，卡片就落在哪", () => {
  const list = ["v1", "v2", "v3", "v4"];

  it("拖到最后一格右半边 = 排到最后（落点线说在 v4 右边就得真在右边）", () => {
    // 光标在最后一格中点的右边 -> 落点下标 3
    expect(moveSlot(list, 0, 3)).toEqual(["v2", "v3", "v4", "v1"]);
  });

  it("拖到中间某一格右半边 = 紧挨着它后面", () => {
    // 光标在 v2 中点右边、v3 中点左边 -> 落点下标 1（剩下的是 v2,v3,v4）
    expect(moveSlot(list, 0, 1)).toEqual(["v2", "v1", "v3", "v4"]);
  });
});

describe("stepTarget（Alt+←/→ 的落点）", () => {
  const list = ["v1", "v2", "v3", "v4"];

  it("往右一格 = 和下一张换位", () => {
    expect(stepTarget(list, 0, 1)).toBe(1);
    expect(moveSlot(list, 0, 1)).toEqual(["v2", "v1", "v3", "v4"]);
  });

  it("往左一格 = 和上一张换位", () => {
    expect(stepTarget(list, 2, -1)).toBe(1);
    expect(moveSlot(list, 2, 1)).toEqual(["v1", "v3", "v2", "v4"]);
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
  /** 四张 100px 宽的卡片，横着排；只认 data-order-id（原版那张固定卡没有它）。 */
  const cards = [0, 1, 2, 3].map((index) => {
    const box = { left: index * 100, width: 100 };
    return {
      dataset: { orderId: "v" + (index + 1) },
      getBoundingClientRect: () => box,
    } as unknown as HTMLElement;
  });

  it("落点 = 鼠标左边还剩几张卡（被拖那张不算）", () => {
    expect(dropIndex(cards, "v4", 10)).toBe(0); // 最前面
    expect(dropIndex(cards, "v4", 260)).toBe(3); // v1/v2/v3 都在左边 -> 正好是原地
    expect(dropIndex(cards, "v4", 360)).toBe(3); // 到末尾
  });

  it("被拖的那格自己不算（不然它永远挡在自己前面）", () => {
    expect(dropIndex(cards, "v1", 45)).toBe(0);
    expect(dropIndex(cards, "v1", 150)).toBe(1); // v2 之前（v1 自己那格不算）
  });
});
