import type { SlotView } from "../../../api/types";
import { shortLabel, type WheelEntry } from "./wheelData";

/** 转盘上的一个候选项：库里的某个版本，或某个官方槽位的原版。 */
export interface WheelCandidate extends WheelEntry {
  slot: string;
  /** 原版战役为 null；启用它表示切回原版。 */
  variantId: string | null;
  /** 资料片 slug（wol/hots/lotv/nova/custom）。 */
  type: string;
  /** 资料片显示名。 */
  typeLabel: string;
  /** 当前选中的版本是不是它。 */
  active: boolean;
}

const VANILLA_SUFFIX = ":__vanilla__";

/** 轮盘只放 4 个字：去掉资料片前缀和尾部的版本号，留下最有辨识度的部分。 */
function shortFor(name: string, typeLabel: string): string {
  const tail = name.startsWith(typeLabel)
    ? name.slice(typeLabel.length).replace(/^[\s:：·\-—]+/, "")
    : name;
  const withoutVersion = tail.replace(/\s+v?\d+(?:\.\d+)*\s*$/i, "").trim();
  return shortLabel(withoutVersion || tail || name, 4);
}

/** 原版短标签：资料片前两个字 + 原版，例如「诺娃原版」。 */
function vanillaShort(typeLabel: string): string {
  return Array.from(typeLabel).slice(0, 2).join("") + "原版";
}

/**
 * 把战役库展开成转盘候选。
 *
 * 官方槽位在原版真的生效时补一条「原版」；自制战役没有原版。
 * vanilla_enabled 是后端字段，演示数据可能没有，退回「没选任何版本」判断。
 */
export function buildCandidates(slots: readonly SlotView[]): WheelCandidate[] {
  const out: WheelCandidate[] = [];
  for (const slot of slots) {
    for (const variant of slot.variants) {
      out.push({
        id: slot.slug + ":" + variant.id,
        label: variant.name,
        short: shortFor(variant.name, slot.display_name),
        slot: slot.slug,
        variantId: variant.id,
        type: slot.slug,
        typeLabel: slot.display_name,
        active: slot.active === variant.id,
      });
    }
    if (slot.slug === "custom") continue;
    const vanillaEnabled =
      slot.vanilla_enabled ?? (slot.active === null && (slot.enabled_variants?.length ?? 0) === 0);
    if (!vanillaEnabled) continue;
    out.push({
      id: slot.slug + VANILLA_SUFFIX,
      label: slot.display_name + " · 原版",
      short: vanillaShort(slot.display_name),
      slot: slot.slug,
      variantId: null,
      type: slot.slug,
      typeLabel: slot.display_name,
      active: slot.active === null,
    });
  }
  return out;
}
