import { describe, expect, it } from "vitest";

import type { LibraryMod } from "../../../api/types";
import { filterMods, groupMods, originLabel } from "./data";

/**
 * 模组页分组的**唯一**依据。
 *
 * 分组判错的后果是用户看不到「关掉这个会影响谁」：独立模组的多个版本散成好几组、
 * 或者两个战役包的模组被并进同一组。这两种错都不会报错，只会显示得莫名其妙。
 */
const base: LibraryMod = {
  slot: "wol",
  slot_name: "自由之翼",
  variant_id: "scmr-8",
  variant_name: "SCMR 8.0",
  path: "Mods/scmr",
  name: "SCMR 数据",
  mounted: false,
  parts: 1,
  origin: "official_campaign",
  required: false,
  standalone_id: null,
  modid: null,
  version: null,
  folder: null,
  kind: null,
  mod_record_id: null,
  source_kind: "campaign",
};

function mod(origin: LibraryMod["origin"], overrides: Partial<LibraryMod> = {}): LibraryMod {
  return { ...base, origin, ...overrides };
}

describe("groupMods —— 按「来源 → 战役 → 版本」分组", () => {
  it("同一个战役包里的模组并成一组，组的身份取自第一个", () => {
    const groups = groupMods([
      mod("official_campaign", { path: "Mods/a", name: "A" }),
      mod("official_campaign", { path: "Mods/b", name: "B" }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].mods.map((item) => item.name)).toEqual(["A", "B"]);
    expect(groups[0].origin).toBe("official_campaign");
    expect(groups[0].slot).toBe("wol");
    expect(groups[0].variantId).toBe("scmr-8");
    expect(groups[0].slotName).toBe("自由之翼");
    expect(groups[0].variantName).toBe("SCMR 8.0");
  });

  it("同一战役的不同版本分成两组（用户才分得清是哪一版）", () => {
    const groups = groupMods([
      mod("official_campaign", { variant_id: "v8", variant_name: "8.0" }),
      mod("official_campaign", { variant_id: "v9", variant_name: "9.0" }),
    ]);

    expect(groups.map((group) => group.variantName)).toEqual(["8.0", "9.0"]);
  });

  it("来源不同就不许并组（哪怕槽位与版本号一样）", () => {
    const groups = groupMods([
      mod("official_campaign", { path: "Mods/a" }),
      mod("custom_campaign", { path: "Mods/b" }),
    ]);

    expect(groups.map((group) => group.origin)).toEqual(["official_campaign", "custom_campaign"]);
  });

  it("独立模组按 modid 归并：同一个模组的多个版本显示成一组", () => {
    const groups = groupMods([
      mod("standalone", { modid: "miyin.core", path: "Mods/core-1", name: "核心 1" }),
      mod("standalone", { modid: "miyin.core", path: "Mods/core-2", name: "核心 2" }),
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0].mods).toHaveLength(2);
    expect(groups[0].slotName).toBe("miyin.core");
  });

  it("独立模组的 modid 不同的各成一组", () => {
    const groups = groupMods([
      mod("standalone", { modid: "miyin.core", path: "Mods/core" }),
      mod("standalone", { modid: "miyin.ui", path: "Mods/ui" }),
    ]);

    expect(groups.map((group) => group.slotName)).toEqual(["miyin.core", "miyin.ui"]);
  });

  it("独立模组没有 modid 时按路径归并，标题退回模组名", () => {
    const merged = groupMods([
      mod("standalone", { modid: null, path: "Mods/only", name: "第一个" }),
      mod("standalone", { modid: null, path: "Mods/only", name: "第二个" }),
    ]);
    expect(merged).toHaveLength(1);
    expect(merged[0].slotName).toBe("第一个");

    const apart = groupMods([
      mod("standalone", { modid: null, path: "Mods/a", name: "甲" }),
      mod("standalone", { modid: null, path: "Mods/b", name: "乙" }),
    ]);
    expect(apart.map((group) => group.slotName)).toEqual(["甲", "乙"]);
  });

  it("顺序按首次出现，后面的同组模组不会把组挪到队尾", () => {
    const groups = groupMods([
      mod("standalone", { modid: "mod.b", path: "Mods/b1" }),
      mod("official_campaign"),
      mod("standalone", { modid: "mod.a", path: "Mods/a1" }),
      mod("standalone", { modid: "mod.b", path: "Mods/b2" }),
    ]);

    expect(groups.map((group) => group.slotName)).toEqual(["mod.b", "自由之翼", "mod.a"]);
    expect(groups[0].mods).toHaveLength(2);
  });

  it("空列表给空数组", () => {
    expect(groupMods([])).toEqual([]);
  });
});

describe("filterMods —— 按模组名或所属战役搜", () => {
  const list = [
    mod("official_campaign", { name: "SCMR 数据", variant_name: "重制版 8.0", slot_name: "自由之翼" }),
    mod("custom_campaign", { name: "Omega", variant_name: "1.0", slot_name: "自制战役" }),
  ];

  it("关键词为空就给全部", () => {
    expect(filterMods(list, "")).toEqual(list);
    expect(filterMods(list, "   ")).toEqual(list);
  });

  it("匹配模组名，且不看大小写", () => {
    expect(filterMods(list, "scmr").map((item) => item.name)).toEqual(["SCMR 数据"]);
    expect(filterMods(list, "OMEGA").map((item) => item.name)).toEqual(["Omega"]);
  });

  it("也匹配所属战役与版本名", () => {
    expect(filterMods(list, "自制").map((item) => item.name)).toEqual(["Omega"]);
    expect(filterMods(list, "重制版").map((item) => item.name)).toEqual(["SCMR 数据"]);
  });

  it("关键词首尾空格忽略", () => {
    expect(filterMods(list, "  omega  ").map((item) => item.name)).toEqual(["Omega"]);
  });

  it("一个都不匹配就给空数组（界面据此显示「没有匹配的模组」）", () => {
    expect(filterMods(list, "不存在的模组")).toEqual([]);
  });
});

describe("originLabel —— 来源怎么说", () => {
  it("三种来源各有说法", () => {
    expect(originLabel("standalone")).toBe("单独导入");
    expect(originLabel("custom_campaign")).toBe("自制战役包");
    expect(originLabel("official_campaign")).toBe("原版战役包");
  });
});

describe("filterMods —— 搜得到自己显示出来的东西", () => {
  it("独立模组按 modid 也能搜到（分组标题显示的就是它）", () => {
    const rows = [mod("standalone", { name: "显示名", modid: "SCMRmod", standalone_id: "s1" })];
    expect(filterMods(rows, "scmrmod")).toHaveLength(1);
  });

  it("照旧按名字搜得到", () => {
    const rows = [mod("standalone", { name: "幼儿园补丁", modid: "SCMRmod", standalone_id: "s1" })];
    expect(filterMods(rows, "幼儿园")).toHaveLength(1);
  });
});
