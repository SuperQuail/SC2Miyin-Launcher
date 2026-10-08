/**
 * 星际争霸 II 作弊码。
 *
 * 在战役里按 `Enter` 打开聊天框，输入后回车即可。**单人战役 / 离线有效**，
 * 对战与天梯无效（也不该有效）。
 *
 * `scope` 标明适用范围：有些码只对特定战役生效，写清楚免得用户白试。
 */
export type CheatScope = "通用" | "自由之翼" | "虫群之心" | "虚空之遗" | "诺娃隐秘行动";

export interface Cheat {
  /** 作弊码本身，照抄进聊天框。 */
  code: string;
  /** 效果。 */
  effect: string;
  /** 适用范围。 */
  scope: CheatScope;
  /** 额外说明，可选。 */
  note?: string;
}

export const CHEATS: Cheat[] = [
  // ---- 战斗控制 ----
  { code: "whatisbestinlife", effect: "立刻获胜", scope: "通用" },
  { code: "letsjustbugoutandcalliteven", effect: "立刻失败", scope: "通用" },
  { code: "terribleterribledamage", effect: "开启无敌模式", scope: "通用" },
  { code: "tooktheredpill", effect: "取消战争迷雾", scope: "通用" },
  { code: "hanshotfirst", effect: "取消技能冷却时间", scope: "通用" },
  { code: "imadoctornotaroachjim", effect: "开启单位快速治愈", scope: "通用" },
  { code: "catfoodforprawnguns", effect: "开启快速建造和升级", scope: "通用" },
  { code: "tyuhasleftthegame", effect: "取消胜利条件，可以无限玩下去", scope: "通用" },
  { code: "nevergiveupneversurrender", effect: "允许战败后继续游戏", scope: "通用" },

  // ---- 资源与人口 ----
  { code: "spectraltiger", effect: "增加 5000 晶矿", scope: "通用" },
  { code: "realmendrilldeep", effect: "增加 5000 气矿", scope: "通用" },
  { code: "whorunbartertown", effect: "增加 5000 晶矿 + 气矿", scope: "通用" },
  { code: "bunker55aliveinside", effect: "无限人口", scope: "通用" },
  { code: "moredotsmoredots", effect: "取消所有单位和建筑费用", scope: "通用" },
  { code: "WhySoSerious", effect: "立刻获得 500 万经费", scope: "通用" },

  // ---- 科技与解锁 ----
  { code: "sosayweall", effect: "取消科技的要求", scope: "通用" },
  { code: "iamironman", effect: "立刻获得所有升级", scope: "通用" },
  {
    code: "HoradricCube",
    effect: "解锁所有研究科技",
    scope: "虫群之心",
    note: "虫群之心的进化关卡专用",
  },
  {
    code: "LeaveYourSleep",
    effect: "解锁所有关卡，可自由切换",
    scope: "通用",
    note: "在战役选关界面使用",
  },

  // ---- 收集品 ----
  { code: "EyeOfSauron", effect: "解锁所有影片", scope: "通用" },
  { code: "StayClassyMarSara", effect: "解锁所有 UNN 新闻", scope: "自由之翼" },
];

/** 按适用范围分组用的固定顺序。 */
export const CHEAT_SCOPES: CheatScope[] = [
  "通用",
  "自由之翼",
  "虫群之心",
  "虚空之遗",
  "诺娃隐秘行动",
];

/** 模糊匹配：码本身或效果描述里包含关键字即可。 */
export function matchCheats(keyword: string, scope: CheatScope | "全部"): Cheat[] {
  const needle = keyword.trim().toLowerCase();
  return CHEATS.filter((cheat) => {
    if (scope !== "全部" && cheat.scope !== scope) return false;
    if (!needle) return true;
    return (
      cheat.code.toLowerCase().includes(needle) || cheat.effect.toLowerCase().includes(needle)
    );
  });
}
