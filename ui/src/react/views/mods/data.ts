import type { LibraryMod } from "../../../api/types";

/** 一组模组：同一个来源 + 同一个战役版本（独立模组则按 modid 归并）。 */
export interface ModGroup {
  key: string;
  slot: string;
  slotName: string;
  variantId: string;
  variantName: string;
  origin: LibraryMod["origin"];
  mods: LibraryMod[];
}

/**
 * 按「来源 → 战役 → 版本」分组。
 *
 * 模组不是孤立的：它总是跟着某个包来的，或者单独导入的。
 * 分开显示用户才知道「关掉它会影响谁」。
 *
 * **独立模组按 modid 归并** —— 同一个模组的多个版本显示成一组，
 * 用户一眼看得出「这几个是同一个东西的不同时期」。
 */
export function groupMods(mods: LibraryMod[]): ModGroup[] {
  const buckets = new Map<string, ModGroup>();

  for (const mod of mods) {
    const key =
      mod.origin === "standalone"
        ? "standalone|" + (mod.modid ?? mod.path)
        : mod.origin + "|" + mod.slot + "|" + mod.variant_id;
    const bucket = buckets.get(key);
    if (bucket) {
      bucket.mods.push(mod);
      continue;
    }
    buckets.set(key, {
      key,
      slot: mod.slot,
      slotName: mod.origin === "standalone" ? (mod.modid ?? mod.name) : mod.slot_name,
      variantId: mod.variant_id,
      variantName: mod.variant_name,
      origin: mod.origin,
      mods: [mod],
    });
  }

  return [...buckets.values()];
}

/** 模组名或所属战役。 */
export function filterMods(mods: LibraryMod[], keyword: string): LibraryMod[] {
  const needle = keyword.trim().toLowerCase();
  if (!needle) return mods;
  return mods.filter(
    (mod) =>
      mod.name.toLowerCase().includes(needle) ||
      mod.variant_name.toLowerCase().includes(needle) ||
      mod.slot_name.toLowerCase().includes(needle) ||
      // 独立模组的分组标题显示的是 modid（见 groupMods），所以它也得搜得到
      (mod.modid ?? "").toLowerCase().includes(needle),
  );
}

/** 来源怎么说。 */
export function originLabel(origin: LibraryMod["origin"]): string {
  if (origin === "standalone") return "单独导入";
  if (origin === "custom_campaign") return "自制战役包";
  return "原版战役包";
}
