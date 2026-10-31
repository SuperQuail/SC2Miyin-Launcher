// 与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）。
// 安装信息与可选外部工具。

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

/** 一个可选外部工具的安装状态。 */
export interface ToolStatus {
  id: string;
  name: string;
  about: string;
  repo: string;
  installed: boolean;
  version: string | null;
  path: string | null;
  size_bytes: number;
  /** 自动安装失败过（启动时不再重试，可手动装）。 */
  auto_failed: boolean;
  /** 上次自动安装失败的原因。 */
  auto_error: string | null;
}

/** 一个工具的可安装版本。 */
export interface ToolRelease {
  version: string;
  tag: string;
  published_at: string;
  prerelease: boolean;
  download_url: string;
  size: number;
  has_asset: boolean;
}

/** 自动安装可选工具的结果。 */
export interface ToolEnsure {
  id: string;
  name: string;
  /** 现在能不能用。 */
  ready: boolean;
  /** 这次干了什么。 */
  action: "already_installed" | "skipped" | "installed" | "failed";
  error: string | null;
  /** 界面该怎么跟用户说；没问题时是 null。 */
  message: string | null;
}
