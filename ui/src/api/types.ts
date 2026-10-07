// 与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）。

/** 安装信息的来源。 */
export type DiscoverySource = "registry" | "manual";

/** 一份可用的星际争霸 II 安装。 */
export interface Installation {
  root: string;
  source: DiscoverySource;
  version: string | null;
  build: number | null;
  branch: string | null;
  executable: string;
  switcher: string | null;
  editor: string | null;
  versions_root: string;
  maps_root: string;
  mods_root: string;
  interfaces_root: string;
  custom_campaigns_root: string;
  campaign_maps_root: string;
  documents_root: string | null;
  user_maps_root: string | null;
  banks_root: string | null;
  arcade_banks_root: string | null;
}

/** 战役包格式。 */
export type CampaignFormat = "ccm" | "miyin" | "plain" | "unknown";

/**
 * 归属资料片。
 *
 * 注意 `Other` 变体在 JSON 里是 `{ "other": "原值" }`。
 */
export type CampaignType =
  | "wol"
  | "hots"
  | "hotsevolution"
  | "lotv"
  | "lotvprologue"
  | "nova"
  | { other: string };

/** 健康度等级。 */
export type HealthLevel = "ok" | "warning" | "broken";

/** 一条核对结论。 */
export interface HealthIssue {
  level: HealthLevel;
  code: string;
  message: string;
  hint: string | null;
}

/** 库里的一个战役版本，即「某个玩家做的某一版」。 */
export interface Variant {
  id: string;
  name: string;
  author: string | null;
  version: string | null;
  description: string | null;
  format: CampaignFormat;
  /** 导入时间（Unix 秒）。 */
  imported_at: number;
  source: string | null;
  map_count: number;
  mod_count: number;
  size_bytes: number;
  /** 启用时地图落到 Maps/Campaign 下的哪个子目录。 */
  target_sub: string | null;
  /** 包内自带的封面图（相对版本目录）；null 表示没有，界面用官方美术。 */
  cover: string | null;
}

/** 一个官方资料片槽位。 */
export interface SlotView {
  slug: string;
  display_name: string;
  /** 启用时地图落到的官方子目录（相对 Maps/Campaign）。 */
  sub_directory: string | null;
  variants: Variant[];
  /** 当前启用的版本 id；null 表示原版战役。 */
  active: string | null;
  active_name: string | null;
  notice: string | null;
}

/** 安装前对包的预检结果。 */
export interface PackageInspection {
  path: string;
  installable: boolean;
  format: CampaignFormat;
  name: string | null;
  author: string | null;
  version: string | null;
  description: string | null;
  campaign_type: CampaignType;
  content_root: string;
  suggested_dir_name: string | null;
  map_count: number;
  mod_count: number;
  entry_count: number;
  unpacked_bytes: number;
  issues: HealthIssue[];
}

/** 启动器后端能力。 */
export interface LauncherApi {
  detectInstallation(): Promise<Installation | null>;
  setInstallation(path: string): Promise<Installation>;
  /** 战役库根目录（软件同级的 data 目录）。 */
  libraryRoot(): Promise<string>;
  /** 全部槽位，已按官方发布顺序排列。 */
  listSlots(): Promise<SlotView[]>;
  inspectPackage(path: string): Promise<PackageInspection>;
  importPackage(path: string, slot: string | null): Promise<Variant>;
  /** 启用某个版本；variantId 传 null 表示切回原版战役。 */
  activateVariant(slot: string, variantId: string | null): Promise<string[]>;
  deleteVariant(slot: string, variantId: string): Promise<void>;
  launchGame(): Promise<void>;
  revealPath(path: string): Promise<void>;
  pickPackage(): Promise<string | null>;
  pickGameDirectory(): Promise<string | null>;
  /** 读取某个版本自带的封面图（data URL）；没有则返回 null。 */
  variantCover(slot: string, variantId: string): Promise<string | null>;
}
