import type { CampaignType } from "./types";

/**
 * 各资料片的官方美术（随应用打包，见 ui/public/campaigns/）。
 *
 * 这些是 Blizzard 版权素材，仅用于让用户一眼认出战役归属，不可再许可。
 */
const ART: Record<string, string> = {
  wol: "/campaigns/wol.jpg",
  hots: "/campaigns/hots.jpg",
  hotsevolution: "/campaigns/hots.jpg",
  lotv: "/campaigns/lotv.jpg",
  lotvprologue: "/campaigns/lotv-prologue.jpg",
  nova: "/campaigns/nco.jpg",
};

export const BACKDROP = "/backdrop.jpg";
export const LOGO = "/campaigns/sc2-logo.png";

/** 弥音立绘（本项目的看板娘）。 */
export const MIYIN = {
  /** 眨眼比心 —— 封面看板娘。 */
  wink: "/miyin/wink.png",
  /** Q 版握拳 —— 小尺寸吉祥物。 */
  chibi: "/miyin/chibi.png",
  /** 特写 —— 关于页立绘。 */
  portrait: "/miyin/portrait.png",
  /** 哭脸 —— 空状态与错误提示。 */
  cry: "/miyin/cry.png",
} as const;

/** 把 CampaignType 归一化成字符串键（Other 变体统一为 other）。 */
export function campaignTypeKey(type: CampaignType): string {
  return typeof type === "string" ? type : "other";
}

/** 该资料片对应的官方美术；没有对应素材时返回 null。 */
export function campaignArt(type: CampaignType): string | null {
  return ART[campaignTypeKey(type)] ?? null;
}

/** 按槽位标识取官方美术。 */
export function slotArt(slug: string): string | null {
  return ART[slug] ?? null;
}

/** 资料片中文名。 */
export function campaignTypeName(type: CampaignType): string {
  switch (campaignTypeKey(type)) {
    case "wol":
      return "自由之翼";
    case "hots":
      return "虫群之心";
    case "hotsevolution":
      return "虫群之心 · 进化";
    case "lotv":
      return "虚空之遗";
    case "lotvprologue":
      return "虚空之遗 · 序章";
    case "nova":
      return "诺娃隐秘行动";
    default:
      return "未标注资料片";
  }
}

/** 人类可读的时间（Unix 秒）。 */
export function formatDate(seconds: number | null): string {
  if (!seconds) return "—";
  const date = new Date(seconds * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return date.getFullYear() + "-" + pad(date.getMonth() + 1) + "-" + pad(date.getDate());
}

/** 人类可读的体积。 */
export function formatBytes(bytes: number | null): string {
  if (bytes === null || bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return (unit === 0 ? value.toFixed(0) : value.toFixed(1)) + " " + units[unit];
}
