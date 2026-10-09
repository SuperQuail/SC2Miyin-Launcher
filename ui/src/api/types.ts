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
  /** 包内**声明为依赖**的模组键。 */
  declared_mods: string[];
  /** 版本自带的说明文档（PDF）；没有就是 null。 */
  doc: string | null;
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
/** 预演里的一条：一个会被动到的目标。对应 `miyin_core::library::PreviewEntry`。 */
export interface PreviewEntry {
  target: string;
  existing_bytes: number;
  incoming_bytes: number;
  /** 现在归谁；别人占着才会有值。 */
  owner: string | null;
}

/** 铺盘前的预演。一个字都不写盘。 */
export interface Preview {
  add: PreviewEntry[];
  overwrite: PreviewEntry[];
  takeover: PreviewEntry[];
  delete: string[];
  bytes: number;
}

/** 现在的存档里有什么。对应 `miyin_core::saves::SaveSet`。 */
export interface SaveSet {
  files: { name: string; bytes: number }[];
  bytes: number;
  /** 目录不存在（还没产生过存档） */
  missing: boolean;
}

/** 存档隔离状态。默认关着 —— 不开的时候启动器不碰 Banks。 */
export interface SaveIsolation {
  enabled: boolean;
  /** 现在游戏里这份存档属于哪个组 */
  active: string | null;
  /** 组名 -> 战役槽位 */
  assignments: Record<string, string>;
}

/** 一份已备份的存档。 */
export interface SaveBackup {
  name: string;
  label: string;
  bytes: number;
  files: number;
}

/** 游戏目录里已经装着的一个自制战役（收编用）。 */
export interface InstalledCampaign {
  dir: string;
  name: string;
  bytes: number;
  files: number;
}

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
  /** 挂载键：`Mods/` 之后的第一段（文件夹名，或 .SC2Mod 文件名）。 */
  path: string;
  name: string;
  mounted: boolean;
  /** 这个模组由几个文件组成。 */
  parts: number;
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
  /** 挂载键：`Mods/` 之后的第一段。 */
  path: string;
  name: string;
  mounted: boolean;
  /** 这个模组由几个文件组成（按文件夹分，一个模组可能有很多部分）。 */
  parts: number;
  /** 从哪来的。 */
  origin: "official_campaign" | "custom_campaign" | "standalone";
  /** 是不是包内声明为依赖的模组。 */
  required: boolean;
  /** 单独导入的模组才有：库里的 id。 */
  standalone_id: string | null;
  /** 同一个模组的多个版本靠它归到一起。 */
  modid: string | null;
  version: string | null;
  /** 铺进游戏目录时用的名字（原样保留的那个）。 */
  folder: string | null;
  /** 铺成文件还是目录。 */
  kind: "file" | "folder" | null;
  /** 模组记录的 id —— 有它就能改信息。 */
  mod_record_id: string | null;
  /** 内容在哪：`library`（独立库）或 `campaign`（某个战役版本）。 */
  source_kind: string;
}

/** 跟着战役包来的模组的定位信息（第一次编辑时用来建记录）。 */
export interface ModIdentity {
  slot: string;
  variant: string;
  path: string;
  folder: string;
  name: string;
  version: string | null;
  kind: string;
  parts: number;
}

/** 独立模组库里的一个模组。 */
export interface StandaloneMod {
  id: string;
  name: string;
  /** modid：判定「同一个模组的新版本」靠它。 */
  modid: string | null;
  /** 内容指纹（SHA-256），用来判定「完全一样」。 */
  fingerprint: string;
  /** 铺进游戏目录时用的名字，原样保留。 */
  folder: string;
  /** 铺成文件还是目录。 */
  kind: "file" | "folder";
  author: string | null;
  version: string | null;
  description: string | null;
  enabled: boolean;
  imported_at: number;
  size_bytes: number;
  parts: number;
}

/** 修改模组信息；null / 不传表示这一项不动。 */
export interface ModChanges {
  name?: string | null;
  author?: string | null;
  version?: string | null;
  description?: string | null;
}

/** 游戏目录 Mods/ 里的一个模组。 */
export interface GameModEntry {
  display: string;
  name: string;
  expanded: boolean;
  size_bytes: number;
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

/** 导入模组的结果。 */
export interface ModImport {
  record: StandaloneMod;
  /** 这次涉及的所有模组 —— 一个包的 Mods/ 下可能有好几个。 */
  records: StandaloneMod[];
  /** 这次怎么处理的。 */
  action: "added" | "new_version" | "duplicate" | "renamed";
  existing: StandaloneMod | null;
  /** 界面该怎么说。 */
  message: string;
}

/** 导入模组前的预检。 */
export interface ModPreview {
  name: string;
  /** 铺进游戏目录时用的名字（原样保留）。 */
  folder: string;
  /** 这一包里有几个模组。 */
  mod_count: number;
  /** 认出来的 modid。 */
  modid: string;
  fingerprint: string;
  /** 库里同 modid 的已有版本。 */
  existing: StandaloneMod[];
  /** 库里已有一份内容完全一样的。 */
  duplicate: boolean;
  suggested_version: string;
}

/** 导入时怎么处理与已有模组的关系。 */
export type ModImportMode = "auto" | "version" | "separate";

/** 两个模组版本的一个文件差异。 */
export interface ModFileDiff {
  path: string;
  status: "same" | "changed" | "added" | "removed";
  size_before: number;
  size_after: number;
}

/** 一份文件的语义 diff。 */
export interface ModSemanticDiff {
  path: string;
  text: string;
}

/** 两个模组版本的对比结果。 */
export interface ModComparison {
  before: StandaloneMod;
  after: StandaloneMod;
  identical: boolean;
  files: ModFileDiff[];
  semantic: ModSemanticDiff[];
  semantic_note: string | null;
}

export interface LauncherApi {
  detectInstallation(): Promise<Installation | null>;
  setInstallation(path: string): Promise<Installation>;
  /** 战役库根目录（软件同级的 data 目录）。 */
  libraryRoot(): Promise<string>;
  /** 全部槽位，已按官方发布顺序排列。 */
  listSlots(): Promise<SlotView[]>;
  inspectPackage(path: string): Promise<PackageInspection>;
  /** 启用某个版本；variantId 传 null 表示切回原版战役。 */
  /** 存档隔离状态。 */
  saveIsolation(): Promise<SaveIsolation>;

  /** 打开 / 关掉隔离。**打开时会把现在这份 Banks 收成「原版」。** */
  setSaveIsolation(enabled: boolean): Promise<SaveIsolation>;

  /** 把现在这份存档存回它所属的组。 */
  saveCurrentSaves(): Promise<SaveIsolation>;

  /** 切到某个存档组（先存回现在这份）。 */
  switchSaveProfile(name: string): Promise<SaveIsolation>;

  /** 手动改归属：指给某个战役，或 null 表示算原版。 */
  assignSaveProfile(name: string, slot: string | null): Promise<SaveIsolation>;

  /** 现在的存档里有什么（我的文档 / StarCraft II / Banks）。 */
  listSaves(): Promise<SaveSet>;

  /** 把现在的存档备份一份。label 是备注（一般填战役名）。 */
  backupSaves(label: string): Promise<string>;

  /** 已经备份了哪些。 */
  listSaveBackups(): Promise<SaveBackup[]>;

  /** 还原一份备份；还原前会先把现在的存档另存一份，返回那份的名字。 */
  restoreSaves(name: string): Promise<string>;

  /** 游戏是不是正跑着；跑着就返回进程名。切换前先问它。 */
  sc2Running(): Promise<string | null>;

  /** 启用前先看：会往游戏目录里放什么、覆盖什么、删什么（不写盘）。 */
  previewActivation(slot: string, variantId: string): Promise<Preview | null>;

  activateVariant(slot: string, variantId: string | null): Promise<string[]>;
  deleteVariant(slot: string, variantId: string): Promise<void>;
  launchGame(): Promise<void>;
  revealPath(path: string): Promise<void>;
  pickPackage(): Promise<string | null>;
  pickGameDirectory(): Promise<string | null>;
  /** 读取某个版本自带的封面图（data URL）；没有则返回 null。 */
  variantCover(slot: string, variantId: string): Promise<string | null>;

  /** 选完文件后的第一步：预检、判断归属、查冲突。不写任何文件。 */
  /**
   * 预检一个包。
   *
   * `entry` 是**用户从哪个入口点的导入**：`custom` 表示站在「自制战役」页 ——
   * 那就不再判断它属于哪部原版战役，直接按自制战役来。
   */
  /** 游戏目录里已经装着的自制战役（收编进库用）。 */
  listInstalledCampaigns(): Promise<InstalledCampaign[]>;

  prepareImport(path: string, entry?: "campaign" | "custom"): Promise<ImportPreview>;
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
      /** 包内声明为依赖的模组键；不传表示这一项不动。 */
      declaredMods?: string[];
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

  /** 可选工具的清单与安装状态。 */
  listTools(): Promise<ToolStatus[]>;
  /** 最小化窗口。 */
  windowMinimize(): Promise<void>;
  /** 最大化 / 还原；返回操作后是不是最大化。 */
  windowToggleMaximize(): Promise<boolean>;
  /** 关窗口。 */
  windowClose(): Promise<void>;
  /** 现在是不是最大化。 */
  windowIsMaximized(): Promise<boolean>;

  /** 启动时调：没装就静默装上；之前失败过就不再试。 */
  ensureTool(id: string): Promise<ToolEnsure>;
  /** 某个工具有哪些版本可装。 */
  toolReleases(id: string): Promise<ToolRelease[]>;
  /** 装一个工具；version 传 null 表示装最新的。 */
  installTool(id: string, version: string | null): Promise<ToolStatus>;
  /** 卸载一个工具。 */
  uninstallTool(id: string): Promise<void>;
  /** 在浏览器里打开工具的仓库。 */
  openToolRepo(id: string): Promise<void>;

  /** 全库模组汇总。 */
  listLibraryMods(): Promise<LibraryMod[]>;
  /** 游戏目录 Mods/ 里实际放着的模组。 */
  listGameMods(): Promise<GameModEntry[]>;
  /** 独立模组库。 */
  listStandaloneMods(): Promise<StandaloneMod[]>;
  /** 选一个模组包：file（压缩包 / .SC2Mod）或 folder。 */
  pickModSource(kind: "file" | "folder"): Promise<string | null>;
  /** 导入前的预检：这个包是谁、库里有没有同族的。 */
  previewMod(path: string): Promise<ModPreview>;
  /** 导入模组包。 */
  importMod(path: string, mode?: ModImportMode): Promise<ModImport>;
  /** 改模组信息。 */
  updateMod(id: string, changes: ModChanges): Promise<StandaloneMod>;
  /**
   * 改一个模组的元数据。
   *
   * 独立模组和战役包带来的模组走同一条路 —— 改的都是记录，不动包里的原始文件。
   * 战役模组第一次编辑时会先建一条记录。
   */
  editMod(
    recordId: string | null,
    identity: ModIdentity | null,
    changes: ModChanges,
  ): Promise<StandaloneMod>;

  /** 删掉模组。 */
  removeMod(id: string): Promise<void>;
  /** 比两个模组版本差在哪。 */
  compareMods(before: string, after: string): Promise<ModComparison>;
  /** 把选中的若干模组版本打包成一个 zip，返回结果说明。 */
  exportMods(ids: string[]): Promise<string>;
  /** 导出模组包，返回写到哪了。 */
  exportMod(id: string): Promise<string>;
  /** 启用 / 停用独立模组（启用会立刻铺进游戏目录）。 */
  setModEnabled(id: string, enabled: boolean): Promise<StandaloneMod>;

  /** 订阅更新过程的日志（界面渲染成内嵌终端），返回取消订阅的函数。 */
  onUpdateLog(handler: (line: string) => void): Promise<() => void>;
}
