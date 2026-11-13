import { shortLabel, type WheelEntry } from "./wheelData";

export type CampaignDatabaseType = "wol" | "hots" | "lotv" | "nova" | "custom";

const TYPE_LABELS: Record<CampaignDatabaseType, string> = {
  wol: "自由之翼",
  hots: "虫群之心",
  lotv: "虚空之遗",
  nova: "诺娃隐秘行动",
  custom: "自制战役",
};

const TYPE_ORDER: Record<CampaignDatabaseType, number> = {
  wol: 0,
  hots: 1,
  lotv: 2,
  nova: 3,
  custom: 4,
};

/** 数据库里的一个战役；一个 JSON 对应一个。 */
export interface DatabaseCampaign {
  id: string;
  title: string;
  type: CampaignDatabaseType;
  typeLabel: string;
  authors: string[];
  description: string;
  download_sources: string[];
  author_links: string[];
}

/** 转盘上的数据库候选。 */
export interface DatabaseWheelItem extends WheelEntry {
  type: string;
  typeLabel: string;
  campaign: DatabaseCampaign;
}

function stringList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .filter(Boolean);
}

/** 从文件路径与 JSON 对象解析一条资料；缺标题或类型目录不认识时返回 null。 */
export function parseDatabaseCampaign(path: string, raw: unknown): DatabaseCampaign | null {
  const parts = path.split(/[\\/]/);
  const folder = parts[parts.length - 2];
  if (!folder || !(folder in TYPE_LABELS)) return null;
  if (raw === null || typeof raw !== "object") return null;

  const data = raw as Record<string, unknown>;
  const title = typeof data.title === "string" ? data.title.trim() : "";
  if (!title) return null;

  const type = folder as CampaignDatabaseType;
  return {
    id: path,
    title,
    type,
    typeLabel: TYPE_LABELS[type],
    authors: stringList(data.authors),
    description: typeof data.description === "string" ? data.description.trim() : "",
    download_sources: stringList(data.download_sources),
    author_links: stringList(data.author_links),
  };
}

const modules = import.meta.glob("../../../data/campaign-db/*/*.json", {
  eager: true,
  import: "default",
}) as Record<string, unknown>;

/** 构建时自动收集数据区里的全部 JSON；新增文件后重新构建即可。 */
export const DATABASE_CAMPAIGNS: DatabaseCampaign[] = Object.entries(modules)
  .map(([path, raw]) => parseDatabaseCampaign(path, raw))
  .filter((item): item is DatabaseCampaign => item !== null)
  .sort(
    (a, b) =>
      TYPE_ORDER[a.type] - TYPE_ORDER[b.type] ||
      a.title.localeCompare(b.title, "zh-Hans-CN"),
  );

export function databaseWheelItems(campaigns: readonly DatabaseCampaign[]): DatabaseWheelItem[] {
  return campaigns.map((campaign) => ({
    id: campaign.id,
    label: campaign.title,
    short: shortLabel(campaign.title, 5),
    type: campaign.type,
    typeLabel: campaign.typeLabel,
    campaign,
  }));
}
