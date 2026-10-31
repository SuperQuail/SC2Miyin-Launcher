// 与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）。
// 战役库：版本 / 槽位、导入与预检、补丁、合成、导出、启动与合并。

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
  /** 最后选中的那个版本 id；null 表示原版战役。**别拿它当「装了没」**。 */
  active: string | null;
  active_name: string | null;
  /** **装在游戏目录里**的版本 id（按激活清单算）。自制战役可以同时有好几个。 */
  enabled_variants?: string[];
  /** 原版是不是装着的那一份（= 这个槽位没有任何版本装着）。 */
  vanilla_enabled?: boolean;
  notice: string | null;
}

/** 安装前对包的预检结果。 */
export interface PackageInspection {
  path: string;
  installable: boolean;
  /**
   * 是不是**缺密码 / 密码不对**（加密包）。
   *
   * 为 true 时要弹密码框：填完带着密码再预检一次。别的失败（包坏了、越界条目）
   * 都是 false —— 那些再问密码也没用。
   */
  needs_password: boolean;
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

/** 导出格式：我们的完整格式 / 星际枢纽能直接导入的标准格式。 */
export type ExportFormat = "miyin" | "scnexus";

/** 导出时会丢失的一项内容（不静默丢，逐条给用户看）。 */
export interface ExportLoss {
  /** 机器可读的分类，例如 custom_campaign / cover / patch。 */
  kind: string;
  /** 一句话说明丢的是什么。 */
  label: string;
  /** 具体丢掉了哪些内容 / 为什么。 */
  detail: string;
}

/** 导出的预检（不写盘）：会带走什么、会丢什么。 */
export interface ExportPreview {
  maps: number;
  mods: number;
  files: number;
  losses: ExportLoss[];
  /** 这个版本能不能用这种格式导出；false 时只能换格式。 */
  supported: boolean;
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
  /** 导出格式；补丁导出没有这一项。 */
  format?: ExportFormat;
  /** 星际枢纽格式下没带过去的内容；弥音格式为空。 */
  losses?: ExportLoss[];
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

/** 主地图的解析结果。 */
export interface MainMapChoice {
  path: string | null;
  /** 是不是「只有一张地图，替你选了」。 */
  automatic: boolean;
  /** 声明了却找不到时的提示 —— 只警告，不阻断。 */
  warning: string | null;
}

/** 编辑器启动的结果。 */
/** 直接启动游戏的结果。 */
export interface GameLaunch {
  /** 要打开哪张地图（库内相对路径）。 */
  map: string;
  /** 界面照着念的一句话。 */
  guidance: string;
}

/** 换版本的结果：换成谁了，以及新包里多出来的模组（已按默认挂上）。 */
export interface UpdateResult {
  variant: Variant;
  addedMods: string[];
}

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

/** 合并时一个版本冲突的三条出路。 */
export type MergeChoice = "target" | "local" | "both";

/** 本地那份与游戏目录那份都有的同一个版本。 */
export interface MergeConflict {
  slot: string;
  variant: string;
  variant_name: string;
  local_bytes: number;
  target_bytes: number;
}

/** 不冲突、直接并进游戏目录那份的版本。 */
export interface MergeAddition {
  slot: string;
  variant: string;
  variant_name: string;
  bytes: number;
}

/** 启动时发现的另一份数据。没有可并的东西时后端返回 null。 */
export interface MergeStatus {
  /** 本地那份的路径（给人看）。 */
  local: string;
  /** 游戏目录那份的路径。 */
  target: string;
  additions: MergeAddition[];
  conflicts: MergeConflict[];
  /** 机器相关文件，一律以 target 为准（active.json / installed.json / dev / backup）。 */
  keep_target: string[];
}
