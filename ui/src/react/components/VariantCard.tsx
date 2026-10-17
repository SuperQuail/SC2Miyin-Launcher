import { useEffect, useState, type MouseEvent as ReactMouseEvent } from "react";

import { formatBytes, slotArt } from "../../api/art";
import { api } from "../../api/bridge";
import type { SlotView, Variant } from "../../api/types";
import "./variant-card.css";

/**
 * 版本卡片（React 版，从 `components/VariantCard.vue` 整块迁过来）。
 *
 * 包内自带封面（data URL）；没有就用该战役的官方美术。
 */
export function VariantCard({
  slot,
  variant,
  active,
  selected,
  onPick,
  onActivate,
  onDrop,
  onMenu,
}: {
  slot: SlotView;
  /** null 表示「原版战役」这张卡片。 */
  variant: Variant | null;
  active: boolean;
  selected: boolean;
  onPick: () => void;
  /** 双击：直接启用这一份（已经启用的就不重复动作）。 */
  onActivate: () => void;
  onDrop: () => void;
  onMenu: (event: ReactMouseEvent) => void;
}) {
  const [customCover, setCustomCover] = useState<string | null>(null);
  const variantId = variant?.id ?? "";

  useEffect(() => {
    let cancelled = false;
    setCustomCover(null);
    if (!variantId) return;
    void (async () => {
      try {
        const cover = await api.variantCover(slot.slug, variantId);
        if (!cancelled) setCustomCover(cover);
      } catch {
        if (!cancelled) setCustomCover(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [slot.slug, variantId]);

  const isVanilla = variant === null;
  const cover = customCover ?? slotArt(slot.slug);
  const coverStyle = cover ? { backgroundImage: "url(" + cover + ")" } : undefined;
  const title = isVanilla ? "原版战役" : (variant?.name ?? "");

  const meta = (() => {
    if (!variant) return "暴雪官方战役，不做任何修改";
    const parts: string[] = [];
    if (variant.author) parts.push(variant.author);
    parts.push(variant.map_count + " 张地图");
    parts.push(formatBytes(variant.size_bytes));
    return parts.join(" · ");
  })();

  return (
    <article
      className={"variant" + (selected ? " variant--selected" : "") + (active ? " variant--active" : "")}
      role="button"
      tabIndex={0}
      onClick={onPick}
      onDoubleClick={() => {
        if (!active) onActivate();
      }}
      onKeyDown={(event) => event.key === "Enter" && onPick()}
      onContextMenu={onMenu}
    >
      <div className="variant__cover" style={coverStyle}>
        <div className="variant__scrim" />

        {/* 勾表示**已启用**，不表示"选中" —— 选中的样子靠卡片边框（.variant--selected） */}
        {active && <span className="variant__check">✓</span>}
        {isVanilla ? (
          <span className="variant__badge variant__badge--left">原版</span>
        ) : (
          variant?.version && <span className="variant__badge variant__badge--left">v{variant.version}</span>
        )}
        {active && <span className="variant__badge variant__badge--right">当前启用</span>}
      </div>

      <div className="variant__body">
        <h4 className="variant__name" title={title}>
          {title}
        </h4>
        <p className="variant__meta">{meta}</p>
        {variant && variant.tags.length > 0 && (
          <p className="variant__tags">
            {variant.tags.map((tag) => (
              <span key={tag} className="variant__tag">
                {tag}
              </span>
            ))}
          </p>
        )}
        {customCover !== null && <p className="variant__own">使用包内封面</p>}
      </div>

      {!isVanilla && (
        <button
          className="variant__drop"
          type="button"
          title="从库中删除这个版本"
          onClick={(event) => {
            event.stopPropagation();
            onDrop();
          }}
        >
          删除
        </button>
      )}
    </article>
  );
}
