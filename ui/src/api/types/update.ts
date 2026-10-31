// 与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）。
// 自动更新与网络设置。

/** 代理模式。 */
export type ProxyMode = "auto" | "off" | "manual";

/** 网络设置。 */
export interface NetworkSettings {
  /** auto = 环境变量 -> Windows 系统代理 -> 直连。 */
  proxy_mode: ProxyMode;
  /** manual 模式下用的地址。 */
  proxy_url: string | null;
  /** 是否允许走 GitHub 镜像加速。 */
  use_mirrors: boolean;
  /** 是否包含预发行版本。 */
  include_prerelease: boolean;
}

/** 自动探测到的代理。 */
export interface DetectedProxy {
  url: string;
  /** 从哪来的（环境变量 / Windows 系统代理 / 手动设置）。 */
  source: string;
}

/** 发行包里的一个文件。 */
export interface ReleaseAsset {
  name: string;
  size: number;
  download_url: string;
  sha256: string | null;
}

/** 一次发行。 */
export interface ReleaseInfo {
  version: string;
  tag: string;
  published_at: string;
  html_url: string;
  notes: string;
  prerelease: boolean;
  assets: ReleaseAsset[];
}

/** 检查更新的结果。 */
export interface UpdateCheck {
  current: string;
  latest: ReleaseInfo | null;
  available: boolean;
  /** 走的是什么网络路径。 */
  via: string | null;
  /** 查不到时的原因。 */
  error: string | null;
}

/** 下载好、等着换上去的更新。 */
export interface Staged {
  version: string;
  archive: string;
  root: string;
  executable: string;
  extras: string[];
  /** 实际用的下载地址（镜像还是直连）。 */
  url: string;
  bytes: number;
}

/** 下载进度。 */
export interface UpdateProgress {
  done: number;
  total: number | null;
  percent: number | null;
}
