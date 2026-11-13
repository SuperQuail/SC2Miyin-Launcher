// 轮盘的纯逻辑：候选、抽样、映射成 react-custom-roulette 的数据。
// 不碰 DOM、不碰 canvas —— 这一层是唯一能用单测钉住的部分。

import type { WheelDataType } from "react-custom-roulette";

/** 轮盘上的一个候选。现在只有 id/label；下一期由 SlotView 映射过来。 */
export interface WheelEntry {
  id: string;
  label: string;
}

/** 从设计令牌读出来的配色。 */
export interface WheelPalette {
  /** 扇区交替底色，至少两项。 */
  background: string[];
  /** 扇区文字色。 */
  text: string;
  /** 边框、分隔线与圆心。 */
  border: string;
}

/** 轮上标签最多放几个字：canvas 只画一行，四个中文刚好放得下。 */
export const WHEEL_LABEL_MAX = 4;

/**
 * 抽一个下标。
 *
 * random 约定返回 [0,1)，但测试会注入 1 / 负数 / NaN 这类边界值 ——
 * 库要求 prizeNumber 落在 [0, count-1]，越界会画到轮子外面，所以这里 clamp。
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
 * 映射成库要的 WheelData。
 *
 * **不要在这里写 style.fontFamily**：库会把每个 fontFamily 拿去判是不是 web-safe，
 * 判成自定义就去 Google Fonts 拉字体 —— CSP 是 default-src 'self'，会被拦掉；
 * 离线环境也没有网。字体统一由 Wheel 组件的 fontFamily prop 传白名单里的 sans-serif。
 */
export function toWheelData(entries: readonly WheelEntry[], palette: WheelPalette): WheelDataType[] {
  return entries.map((entry, index) => ({
    option: shortLabel(entry.label),
    style: {
      backgroundColor: palette.background[index % palette.background.length],
      textColor: palette.text,
    },
  }));
}

/** 示例数据：只验证技术栈与观感，不是最终抽奖池。 */
export const SAMPLE_ENTRIES: WheelEntry[] = [
  { id: "wol-reborn", label: "自由之翼：重生" },
  { id: "hots-evolution", label: "虫群之心：进化重制" },
  { id: "lotv-scrolls", label: "虚空之遗：残卷" },
  { id: "lotv-rogue", label: "虚空之遗：肉鸽" },
  { id: "nova", label: "诺娃隐秘行动" },
  { id: "scmr", label: "Starcraft Mass Recall" },
  { id: "frontier", label: "边境之地" },
  { id: "custom-demo", label: "自制战役 · 示例" },
];
