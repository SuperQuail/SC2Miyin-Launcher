// 轮盘的纯逻辑：候选、抽样、指针落点。
// 不碰 DOM、不碰 canvas —— 这一层是唯一能用单测钉住的部分。

/** 轮盘上的一个候选。现在只有 id/label；下一期由 SlotView 映射过来。 */
export interface WheelEntry {
  id: string;
  label: string;
  /** 轮盘上用的短标签；不传就按 label 截断。 */
  short?: string;
}

/** 从设计令牌读出来的配色。 */
export interface WheelPalette {
  /** 扇区交替底色，至少两项。 */
  background: string[];
  /** 扇区文字色。 */
  text: string;
  /** 边框 / 圆心色。 */
  border: string;
  /** 主色，按钮与指针用。 */
  accent: string;
  /** 表面色，按钮/圆心用。 */
  surface: string;
}

/** 轮上标签最多放几个字：扇区里只放得下一行短标签。 */
export const WHEEL_LABEL_MAX = 4;

/**
 * 抽一个下标。
 *
 * random 约定返回 [0,1)，但测试会注入 1 / 负数 / NaN 这类边界值；
 * 结果必须落在 [0, count-1]。
 */
export function pickIndex(random: number, count: number): number {
  if (count <= 0) return 0;
  const raw = Number.isFinite(random) ? Math.floor(random * count) : 0;
  return Math.min(count - 1, Math.max(0, raw));
}

/** 轮上只放短标签；完整名字留给候选列表与结果卡。 */
export function shortLabel(label: string, max = WHEEL_LABEL_MAX): string {
  const chars = Array.from(label.trim());
  return chars.length <= max ? chars.join("") : chars.slice(0, max).join("") + "…";
}

/**
 * 第 index 个扇区的中心角（度，顺时针，0 = 12 点）。
 *
 * 扇区 i 从 i * 360/count 铺到 (i+1) * 360/count，中心在 (i+0.5) * 360/count。
 */
export function sectorCenter(index: number, count: number): number {
  if (count <= 0) return 0;
  return ((index % count) + 0.5) * (360 / count);
}

/**
 * 指针这次要停的绝对角度。
 *
 * - 至少转 turns 圈，只往顺时针走；
 * - 最后落在 winner 扇区的中心（加一点 jitter 也仍在扇区里）。
 */
export function targetRotation(current: number, winner: number, count: number, turns = 4, jitter = 0): number {
  const center = sectorCenter(winner, count);
  const normalized = ((current % 360) + 360) % 360;
  const delta = ((center - normalized) % 360 + 360) % 360;
  return current + turns * 360 + delta + jitter;
}

/** 示例数据：只验证技术栈与观感，不是最终抽奖池。 */
export const SAMPLE_ENTRIES: WheelEntry[] = [
  { id: "wol-reborn", label: "自由之翼：重生", short: "自由之翼" },
  { id: "hots-evolution", label: "虫群之心：进化重制", short: "虫群进化" },
  { id: "lotv-scrolls", label: "虚空之遗：残卷", short: "虚空残卷" },
  { id: "lotv-rogue", label: "虚空之遗：肉鸽", short: "虚空肉鸽" },
  { id: "nova", label: "诺娃隐秘行动", short: "诺娃隐秘" },
  { id: "scmr", label: "Starcraft Mass Recall", short: "SCMR" },
  { id: "frontier", label: "边境之地", short: "边境之地" },
  { id: "custom-demo", label: "自制战役 · 示例", short: "自制示例" },
];
