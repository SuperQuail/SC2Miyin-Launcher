import { formatBytes, slotArt } from "../../api/art";
import type { ScnexusCampaign } from "../../api/types";
import "./variant-card.css";
import "./scnexus-card.css";

/**
 * 星际枢纽（SCNexus）托管的一个战役卡片。
 *
 * 复用 `VariantCard` 的视觉（同一个 .variant 骨架）：左上角固定标出「来自于：星际枢纽」，
 * 让用户分得清「启动器库里的改版」与「还在星际枢纽里、可以转过来」的条目。
 */
export function ScnexusCard({
  campaign,
  onConvert,
}: {
  campaign: ScnexusCampaign;
  /** 打开「转为启动器托管」确认框。 */
  onConvert: () => void;
}) {
  const coverStyle = { backgroundImage: "url(" + slotArt(campaign.campaign) + ")" };
  const meta = [
    campaign.author,
    campaign.version ? "v" + campaign.version : null,
    campaign.maps + " 张地图",
    formatBytes(campaign.size_bytes),
  ]
    .filter((part): part is string => Boolean(part))
    .join(" · ");

  return (
    <article className="variant scnexus-card">
      <div className="variant__cover" style={coverStyle}>
        <div className="variant__scrim" />
        <span className="variant__badge variant__badge--left scnexus-card__badge">
          来自于：星际枢纽
        </span>
        {campaign.active && (
          <span className="variant__badge variant__badge--right">星际枢纽中已启用</span>
        )}
      </div>

      <div className="variant__body">
        <h4 className="variant__name" title={campaign.name}>
          {campaign.name}
        </h4>
        <p className="variant__meta">{meta}</p>
        {campaign.description && (
          <p className="scnexus-card__desc" title={campaign.description}>
            {campaign.description}
          </p>
        )}
        <div className="scnexus-card__actions">
          <button className="btn btn-tonal btn--tiny" type="button" onClick={onConvert}>
            转为启动器托管
          </button>
          <span className="scnexus-card__path" title={campaign.path}>
            {campaign.source === "ccm" ? "CCM 目录" : "星际枢纽标准库"}
          </span>
        </div>
      </div>
    </article>
  );
}
