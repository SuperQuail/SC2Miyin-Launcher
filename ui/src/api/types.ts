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
  /** 包自报的标签。 */
  tags: string[];
  /** 包声明的注册 ID：补丁靠它引用战役，更新靠它认出同一个战役。 */
  registration_id: string | null;
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
  /** 包类型：战役本体还是覆盖层补丁。 */
  kind: PackageKind;
  /** 注册 ID（补丁靠它引用战役）。 */
  id: string | null;
  /** 补丁依赖的战役；为空表示只能手动指定。 */
  requires: string[];
  /** 补丁默认优先级。 */
  priority: number | null;
  /** 载荷清单（地图/模组及其落点）。 */
  payloads: Payload[];
  /** 归属判定的结论与依据；包内已声明时为 null。 */
  identification: Identification | null;
  /** 包自带的封面图（相对内容根）；null 表示没有，界面用官方美术。 */
  cover: string | null;
  /** 包自报的标签。 */
  tags: string[];
  /** 按包内声明推断出的目标战役；null 表示认不出来，需要用户指定。 */
  suggested_slot: string | null;
  content_root: string;
  suggested_dir_name: string | null;
  map_count: number;
  mod_count: number;
  entry_count: number;
  unpacked_bytes: number;
  issues: HealthIssue[];
}

/** 启动器后端能力。 */
/** 包类型。 */
export type PackageKind = "campaign" | "patch";

/** 判定归属所依据的证据，按可靠度从高到低。 */
export type CampaignEvidence =
  | "metadata"
  | "mirror_path"
  | "map_dependency"
  | "map_name_prefix";

/** 一次归属判定。 */
export interface Identification {
  campaign_type: CampaignType;
  evidence: CampaignEvidence;
  /** 依据的具体内容，例如 "Void Story"。 */
  detail: string;
}

/** 载荷的落点。 */
export type PayloadTarget =
  | { kind: "mirror"; path: string }
  | { kind: "mod"; name: string }
  | { kind: "map"; name: string };

/** 包内一个载荷。 */
export interface Payload {
  source: string;
  target: PayloadTarget;
  /** 是否是解开的目录树（false = 单文件 MPQ）。 */
  expanded: boolean;
  is_mod: boolean;
}

/** 这条导入信息是从哪来的。 */
export type ImportSource = "metadata" | "inferred" | "manual";

/** 新旧版本对比结论。 */
export type VersionRelation = "newer" | "same" | "older" | "unknown";

/** 与库里已有版本的冲突。 */
export interface Conflict {
  existing_id: string;
  existing_name: string;
  existing_version: string | null;
  incoming_version: string | null;
  /** 是否命中同一个注册 ID（比同名更强的信号）。 */
  same_id: boolean;
  relation: VersionRelation;
}

/** 导入预览。 */
export interface ImportPreview {
  path: string;
  inspection: PackageInspection;
  source: ImportSource;
  /** 自动判断出的目标战役；null 表示需要用户指定。 */
  slot: string | null;
  conflict: Conflict | null;
}

/** 导入方式。 */
export type ImportMode = "rename" | "overwrite";

/** 库里的一个补丁。 */
export interface Patch {
  id: string;
  name: string;
  author: string | null;
  version: string | null;
  description: string | null;
  registration_id: string | null;
  priority: number;
  /** 依赖的战役；为空表示只能手动指定。 */
  requires: string[];
  payloads: Payload[];
  imported_at: number;
  size_bytes: number;
  mod_count: number;
}

/** 挂在某个战役上的补丁。 */
export interface BoundPatch extends Patch {
  /** 生效优先级（可能被挂载时改过）。 */
  priority: number;
  enabled: boolean;
  /** 按 requires 是否匹配得上这个战役。 */
  matched: boolean;
}

/** 合成清单里的一项。 */
export interface ComposedFile {
  target: string;
  layer: { kind: "campaign" } | { kind: "patch"; id: string; name: string; priority: number };
  expanded: boolean;
}

/** 一份合成清单。 */
export interface Composition {
  files: ComposedFile[];
  overridden: ComposedFile[];
}

/** 导出结果。 */
export interface ExportReport {
  path: string;
  files: number;
  maps: number;
  mods: number;
  /** 目录树形态的载荷名 —— CCM 大概率读不了，界面要提示。 */
  expanded: string[];
  /** 一起带走的补丁名。 */
  patches: string[];
}

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

/** 版本里的一张地图。 */
export interface MapEntry {
  /** 相对版本目录的路径，用 / 分隔 —— 主地图存的就是这个形式。 */
  path: string;
  /** 显示名（文件名去掉扩展名）。 */
  name: string;
  /** 包内的第一层目录名。作者可能拿它分章节 / 幕，也可能压根不用目录。 */
  chapter: string | null;
  size: number;
  is_main: boolean;
}

/** 版本里的一个模组。 */
export interface ModEntry {
  /** 挂载键：相对版本目录的路径。 */
  path: string;
  name: string;
  mounted: boolean;
}

/** 主地图的解析结果。 */
export interface MainMapChoice {
  path: string | null;
  /** 是不是「只有一张地图，替你选了」。 */
  automatic: boolean;
  /** 声明了却找不到时的提示 —— 只警告，不阻断。 */
  warning: string | null;
}

/** 编辑器启动的结果。 */
export interface EditorLaunch {
  editor: string;
  map: string;
  /** 用户接下来要自己做什么，界面照着念。 */
  guidance: string;
}

/** 版本自带的说明文档。 */
export interface DocInfo {
  path: string;
  name: string;
  size: number;
}

/** 库里的一个模组，带着它属于哪个版本。 */
export interface LibraryMod {
  slot: string;
  slot_name: string;
  variant_id: string;
  variant_name: string;
  /** 挂载键：相对版本目录的路径。 */
  path: string;
  name: string;
  mounted: boolean;
}

/** 游戏目录 Mods/ 里的一个模组。 */
export interface GameModEntry {
  display: string;
  name: string;
  expanded: boolean;
  size_bytes: number;
}

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

  /** 选完文件后的第一步：预检、判断归属、查冲突。不写任何文件。 */
  prepareImport(path: string): Promise<ImportPreview>;
  /** 按指定战役导入；传了 slot 就按传的来，覆盖自动判定。 */
  importPackageWith(
    path: string,
    slot: string | null,
    mode: ImportMode,
  ): Promise<Variant>;
  /** 改一个已导入版本的元数据。 */
  updateVariant(
    slot: string,
    variantId: string,
    changes: {
      name?: string;
      author?: string;
      registrationId?: string;
      description?: string;
    },
  ): Promise<Variant>;

  listPatches(): Promise<Patch[]>;
  listBindings(slot: string): Promise<BoundPatch[]>;
  listAvailablePatches(slot: string): Promise<BoundPatch[]>;
  importPatch(path: string): Promise<Patch>;
  autoBindPatches(): Promise<string[]>;
  bindPatch(slot: string, patchId: string): Promise<void>;
  unbindPatch(slot: string, patchId: string): Promise<void>;
  configurePatch(
    slot: string,
    patchId: string,
    enabled: boolean | null,
    priority: number | null,
  ): Promise<unknown>;
  deletePatch(patchId: string): Promise<void>;
  /** 改一个已导入补丁的元数据。 */
  updatePatch(
    patchId: string,
    changes: {
      name?: string;
      author?: string;
      registrationId?: string;
      description?: string;
      priority?: number;
    },
  ): Promise<Patch>;
  /** 单独导出一个补丁包。 */
  exportPatch(patchId: string, destination: string): Promise<ExportReport>;
  previewComposition(slot: string, variantId: string): Promise<Composition>;

  exportVariant(
    slot: string,
    variantId: string,
    destination: string,
    mergePatches: boolean,
  ): Promise<ExportReport>;
  pickExportPath(defaultName: string): Promise<string | null>;

  /** 当前程序版本。 */
  appVersion(): Promise<string>;
  networkSettings(): Promise<NetworkSettings>;
  setNetworkSettings(settings: NetworkSettings): Promise<void>;
  /** 当前自动探测到的代理；直连时为 null。 */
  detectedProxy(): Promise<DetectedProxy | null>;
  checkUpdate(): Promise<UpdateCheck>;
  downloadUpdate(version: string): Promise<Staged>;
  /** 换上新版本并退出程序。 */
  applyUpdate(): Promise<void>;
  openUrl(url: string): Promise<void>;
  /** 订阅下载进度，返回取消订阅的函数。 */
  onUpdateProgress(handler: (progress: UpdateProgress) => void): Promise<() => void>;
  /** 列出版本里的地图。 */
  variantMaps(slot: string, variantId: string): Promise<MapEntry[]>;
  /** 列出版本里的模组与挂载状态。 */
  variantMods(slot: string, variantId: string): Promise<ModEntry[]>;
  /** 改挂载清单。 */
  setMountedMods(slot: string, variantId: string, mods: string[]): Promise<Variant>;
  /** 改主地图；传 null 清空。 */
  setMainMap(slot: string, variantId: string, map: string | null): Promise<Variant>;
  /** 这个版本该用哪张地图作为入口。 */
  mainMapChoice(slot: string, variantId: string): Promise<MainMapChoice>;
  /** 铺模组并用编辑器打开某张地图。 */
  openMapInEditor(slot: string, variantId: string, map: string): Promise<EditorLaunch>;
  /** 版本自带的说明文档；没有就是 null。 */
  variantDoc(slot: string, variantId: string): Promise<DocInfo | null>;
  /** 读出说明文档的字节，交给 PDF 渲染器。 */
  readDoc(slot: string, variantId: string): Promise<ArrayBuffer>;

  /** 全库模组汇总。 */
  listLibraryMods(): Promise<LibraryMod[]>;
  /** 游戏目录 Mods/ 里实际放着的模组。 */
  listGameMods(): Promise<GameModEntry[]>;

  /** 订阅更新过程的日志（界面渲染成内嵌终端），返回取消订阅的函数。 */
  onUpdateLog(handler: (line: string) => void): Promise<() => void>;
}
