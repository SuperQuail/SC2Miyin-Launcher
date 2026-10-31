// 与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）。
// 模组：战役里的、独立库里的、导入 / 对比 / 同步。

/** 版本里的一个模组。 */
export interface ModEntry {
  /** 挂载键：`Mods/` 之后的第一段（文件夹名，或 .SC2Mod 文件名）。 */
  path: string;
  name: string;
  /** 用户配置里挂着它（勾选框状态），不代表已经铺进游戏目录。 */
  mounted: boolean;
  /** 真的铺在 `<游戏>/Mods/` 里（按安装清单算）。 */
  in_game: boolean;
  /** 这个模组由几个文件组成。 */
  parts: number;
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
  /** 用户配置里挂着它（勾选框状态）。 */
  mounted: boolean;
  /** 真的铺在 `<游戏>/Mods/` 里（按安装清单算）。 */
  in_game: boolean;
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
