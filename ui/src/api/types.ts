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

/** 一个已安装的战役。 */
export interface Campaign {
  id: string;
  name: string;
  author: string | null;
  version: string | null;
  description: string | null;
  cover: string | null;
  path: string;
  format: CampaignFormat;
  campaign_type: CampaignType;
  enabled: boolean;
  health: HealthLevel;
  issues: HealthIssue[];
  map_count: number | null;
  size_bytes: number | null;
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
  listCampaigns(): Promise<Campaign[]>;
  inspectPackage(path: string): Promise<PackageInspection>;
  installPackage(path: string): Promise<Campaign>;
  uninstallCampaign(id: string): Promise<void>;
  launchGame(): Promise<void>;
  revealPath(path: string): Promise<void>;
  pickPackage(): Promise<string | null>;
  pickGameDirectory(): Promise<string | null>;
  campaignCover(dir: string): Promise<string | null>;
}
