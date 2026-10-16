import { slotArt } from "../../api/art";
import type { SlotView } from "../../api/types";
import "./slot-card.css";

/**
 * 战役卡片（React 版，从 `components/SlotCard.vue` 整块迁过来）。
 *
 * 列表里的一张卡：封面 + 状态。点它进入这个战役自己的菜单页。
 */
export function SlotCard({ slot, onOpen }: { slot: SlotView; onOpen: (slug: string) => void }) {
  const art = slotArt(slot.slug);
  const coverStyle = art ? { backgroundImage: "url(" + art + ")" } : undefined;
  const isVanilla = slot.active === null;
  const variantCount = slot.variants.length;
  const statusText = slot.active_name ? "已启用：" + slot.active_name : "原版战役";

  return (
    <button className="slot" type="button" onClick={() => onOpen(slot.slug)}>
      <span className="slot__cover" style={coverStyle}>
        <span className="slot__scrim" />
        <span className={"slot__badge" + (isVanilla ? "" : " slot__badge--mod")}>
          {isVanilla ? "原版" : "已切换"}
        </span>
        {variantCount > 0 && <span className="slot__count">{variantCount} 个版本</span>}
      </span>

      <span className="slot__body">
        <span className="slot__name">{slot.display_name}</span>
        <span className={"slot__status" + (isVanilla ? "" : " slot__status--active")}>{statusText}</span>
      </span>
    </button>
  );
}
