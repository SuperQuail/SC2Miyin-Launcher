// 星际枢纽（SCNexus）托管的战役：检测、转换与导出契约。
//
// 详细依据见 docs/design/scnexus-export-handoff.md 与 AGENTS §14。
import type { Variant } from "./campaign";

/** 在 SCNexus 托管目录里发现的一个战役。 */
export interface ScnexusCampaign {
  /** 稳定 id：元数据里的 snid 优先，缺了就用目录路径的稳定哈希。 */
  id: string;
  name: string;
  author: string | null;
  version: string | null;
  description: string | null;
  /** 资料片归属："wol" | "hots" | "lotv" | "nova"（SCNexus 只认这四种）。 */
  campaign: string;
  /** 内容从哪发现的：SCNexus 标准库，还是它托管的 CCM 目录。 */
  source: "library" | "ccm";
  /** 战役目录的绝对路径。 */
  path: string;
  /** 是不是 SCNexus 当前启用的那一个。 */
  active: boolean;
  maps: number;
  mods: number;
  size_bytes: number;
}

/** 「转为启动器托管」的结果。 */
export interface ConvertedScnexus {
  /** 转换后归到哪个槽位（wol / hots / lotv / nova）。 */
  slot: string;
  /** 新导入到我们库里的版本。 */
  variant: Variant;
  /** 转换过程中要告诉用户的事（例如源目录没删成）。 */
  warnings: string[];
}
