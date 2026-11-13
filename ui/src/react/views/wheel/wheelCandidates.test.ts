import { describe, expect, it } from "vitest";

import type { SlotView, Variant } from "../../../api/types";
import { buildCandidates } from "./wheelCandidates";

function variant(id: string, name: string): Variant {
  return {
    id,
    name,
    author: null,
    version: null,
    description: null,
    format: "miyin",
    imported_at: 0,
    source: null,
    map_count: 0,
    mod_count: 0,
    size_bytes: 0,
    target_sub: null,
    cover: null,
    tags: [],
    registration_id: null,
    declared_mods: [],
    doc: null,
  };
}

function slot(over: Partial<SlotView>): SlotView {
  return {
    slug: "wol",
    display_name: "自由之翼",
    sub_directory: null,
    variants: [],
    active: null,
    active_name: null,
    vanilla_enabled: false,
    notice: null,
    ...over,
  };
}

describe("buildCandidates", () => {
  it("把版本与原版展开成候选，短标签去掉资料片前缀和版本号", () => {
    const slots = [
      slot({
        variants: [
          variant("a", "自由之翼：重生 v1.4.2"),
          variant("b", "自由之翼：重生 v1.5.0"),
        ],
        active: "a",
        vanilla_enabled: true,
      }),
    ];

    const out = buildCandidates(slots);

    expect(out.map((item) => item.id)).toEqual(["wol:a", "wol:b", "wol:__vanilla__"]);
    expect(out.map((item) => item.short)).toEqual(["重生", "重生", "自由原版"]);
    expect(out.map((item) => item.active)).toEqual([true, false, false]);
    expect(out[2].variantId).toBeNull();
  });

  it("自制战役没有原版；短标签保留可辨识的名字", () => {
    const slots = [
      slot({
        slug: "custom",
        display_name: "自制战役",
        variants: [variant("c", "边境之地 1.0")],
        vanilla_enabled: true,
      }),
    ];

    const out = buildCandidates(slots);

    expect(out).toHaveLength(1);
    expect(out[0].short).toBe("边境之地");
    expect(out[0].type).toBe("custom");
    expect(out[0].typeLabel).toBe("自制战役");
  });
});
