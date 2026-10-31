// 与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）。
// 存档：扫描、备份、还原与存档隔离。

/** 存档里的一个文件。对应 `miyin_core::saves::SaveFile`。 */
export interface SaveFile {
  name: string;
  bytes: number;
}

/** 一个存档文件（`*.SC2Save`）。对应 `miyin_core::saves::SaveEntry`。 */
export interface SaveEntry {
  /** 账号目录名（`Accounts/<账号>` 那一层，通常是数字 id）。 */
  account: string;
  /** 账号下那一层（`<地域>-<账号>`）。 */
  region: string;
  /** 层级：带 `Saves` 前缀的相对路径（`Saves` / `Saves/Campaign` / `Saves/Unsaved/Campaign` …）。 */
  folder: string;
  /** 文件名，**不含扩展名**。 */
  name: string;
  path: string;
  bytes: number;
  /** 修改时间（Unix 秒）。 */
  modified: number;
}

/** 一个 Banks 文件（`*.SC2Bank`）。对应 `miyin_core::saves::BankEntry`。 */
export interface BankEntry {
  account: string;
  region: string;
  /** 地图 key（父目录名；直接在 `Banks` 根下就是空串）。 */
  map: string;
  name: string;
  path: string;
  bytes: number;
  modified: number;
}

/** 扫一遍真实存档目录的结果（`Accounts/<账号>/<地域>/Saves` 与 `Banks`）。 */
export interface SaveInventory {
  /** 账号目录名，按最近活跃排序。 */
  accounts: string[];
  /** 所有 `*.SC2Save`，按修改时间降序。 */
  saves: SaveEntry[];
  /** 所有 `*.SC2Bank`，按修改时间降序。 */
  banks: BankEntry[];
}

/** 现在的存档里有什么。对应 `miyin_core::saves::SaveSet`。 */
export interface SaveSet {
  files: SaveFile[];
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

/** 存档页「归属」下拉的一个候选：原版 / 每个导入的版本各一条，自制同理。 */
export interface SaveTarget {
  /** 写进 state.json 的隔离键：原版 = 槽位；有版本 = `<槽位>/<身份>`。 */
  key: string;
  /** 界面上的说法：`<战役名>（原版）` / `<战役名> · <版本名>`。 */
  label: string;
  /**
   * 这个战役 / 版本现在**装在游戏目录里**（按激活清单算，不看槽位选中项）。
   *
   * 官方改版最多一个为 true；**自制战役可以有好几个 true**。
   * 后端一定会送；可选是为了老的测试夹具不用补。
   */
  enabled?: boolean;
}

/** 一份已备份的存档（= 一个存档组）。 */
export interface SaveBackup {
  name: string;
  label: string;
  bytes: number;
  files: number;
  /**
   * **自动生成**的备份（还原前留的安全网，标签「还原前」）。
   *
   * 它不是用户意义上的存档组 —— 组列表要过滤掉（`!item.auto`），
   * 但还原列表里还应该能看到。
   *
   * 后端一定会送这个字段；声明成可选只是为了老的演示数据 / 测试夹具不用跟着补
   * —— 缺省当 false（= 这是个正常的组）。
   */
  auto?: boolean;
  /**
   * **没有绑定任何战役/版本**的老组（升级前收下的那份，或用户手动备份的）。
   *
   * 这种组的名字说明不了内容（当年是开隔离那一刻把整份 Banks 收下来的，
   * 用户可能正在玩某个自制战役）—— 界面别再叫它「原版」，
   * 显示成「未分类（升级前的存档）」，让用户自己去归位。
   */
  legacy?: boolean;
}
