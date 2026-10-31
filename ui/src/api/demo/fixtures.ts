/**
 * 演示后端共用的夹具：安装信息、槽位与版本、计数器、以及「稍等一下」的延迟。
 *
 * 只在浏览器演示模式（`isDesktop === false`）里用到；真实桥见 `../bridge.ts`。
 */
import type { Installation, SlotView, Variant } from "../types";

/** 演示数据：数值取自开发机的真实安装。 */
export const demoInstallation: Installation = {
  root: "D:\\Game\\BLZ\\StarCraft II",
  source: "registry",
  version: "5.0.16.97579",
  build: 97579,
  branch: "cn",
  executable: "D:\\Game\\BLZ\\StarCraft II\\StarCraft II.exe",
  switcher: "D:\\Game\\BLZ\\StarCraft II\\Support64\\SC2Switcher_x64.exe",
  editor: "D:\\Game\\BLZ\\StarCraft II\\Support64\\SC2Editor_x64.exe",
  versions_root: "D:\\Game\\BLZ\\StarCraft II\\Versions",
  maps_root: "D:\\Game\\BLZ\\StarCraft II\\Maps",
  mods_root: "D:\\Game\\BLZ\\StarCraft II\\Mods",
  interfaces_root: "D:\\Game\\BLZ\\StarCraft II\\Interfaces",
  custom_campaigns_root: "D:\\Game\\BLZ\\StarCraft II\\Maps\\CustomCampaigns",
  campaign_maps_root: "D:\\Game\\BLZ\\StarCraft II\\Maps\\Campaign",
  documents_root: "C:\\Users\\ROG\\Documents\\StarCraft II",
  user_maps_root: "C:\\Users\\ROG\\Documents\\StarCraft II\\Maps",
  banks_root: "C:\\Users\\ROG\\Documents\\StarCraft II\\Banks",
  arcade_banks_root: "C:\\Users\\ROG\\Documents\\StarCraft II\\ArcadeBanks",
};

export const DEMO_LIBRARY_ROOT = "D:\\Code\\Rust\\HSCL\\target\\debug\\data";

/** 演示里导入 / 更新 / 补丁共用这个号（和拆分前一样全局递增）。 */
let counter = 0;

/** 取下一个编号。 */
export function nextDemoCounter(): number {
  counter += 1;
  return counter;
}

export function variant(
  id: string,
  name: string,
  author: string,
  version: string,
  maps: number,
  size: number,
): Variant {
  return {
    id,
    name,
    author,
    version,
    description: null,
    format: "ccm",
    imported_at: 1790000000,
    source: id + ".zip",
    map_count: maps,
    mod_count: 0,
    size_bytes: size,
    target_sub: null,
    cover: null,
    tags: [],
    registration_id: null,
    declared_mods: [],
    doc: null,
  };
}

/** 演示用的槽位（顺序即官方发布顺序）。 */
export const demoslots: SlotView[] = [
  {
    slug: "wol",
    display_name: "自由之翼",
    sub_directory: null,
    variants: [
      variant("自由之翼：重生 v1.4.2", "自由之翼：重生", "SomeCreator", "1.4.2", 32, 1_284_000_000),
      variant("自由之翼：重生 v1.5.0", "自由之翼：重生", "SomeCreator", "1.5.0", 34, 1_402_000_000),
    ],
    active: "自由之翼：重生 v1.4.2",
    active_name: "自由之翼：重生",
    notice: null,
  },
  {
    slug: "hots",
    display_name: "虫群之心",
    sub_directory: "swarm",
    variants: [
      Object.assign(
        variant("虫群之心：进化重制", "虫群之心：进化重制", "SwarmTeam", "0.9.1", 14, 760_000_000),
        { target_sub: "swarm/evolution" },
      ),
    ],
    active: null,
    active_name: null,
    notice: null,
  },
  {
    slug: "lotv",
    display_name: "虚空之遗",
    sub_directory: "void",
    variants: [
      variant("虚空之遗：残卷", "虚空之遗：残卷", "VoidWorks", "2.0", 9, 412_000_000),
      // 「改版和原版各自一份存档」的演示样本（用户原话举的就是它）
      variant("虚空之遗：肉鸽", "虚空之遗：肉鸽", "RogueTeam", "1.2", 16, 780_000_000),
    ],
    active: null,
    active_name: null,
    notice: null,
  },
  {
    slug: "nova",
    display_name: "诺娃隐秘行动",
    sub_directory: "nova",
    variants: [],
    active: null,
    active_name: null,
    notice: null,
  },
  {
    slug: "custom",
    display_name: "自制战役",
    sub_directory: null,
    variants: [
      Object.assign(
        variant(
          "Starcraft Mass Recall 8.0",
          "Starcraft Mass Recall",
          "SCMR Team",
          "8.0",
          138,
          1_414_700_000,
        ),
        { mod_count: 4, doc: null },
      ),
      // 第二部自制战役：用来演"每部自制战役各自一组存档"
      Object.assign(
        variant("边境之地 1.0", "边境之地", "FrontierWorks", "1.0", 12, 380_000_000),
        { mod_count: 0, doc: null },
      ),
    ],
    active: "Starcraft Mass Recall 8.0",
    active_name: "Starcraft Mass Recall",
    notice: null,
  },
];

/**
 * 演示模式里「游戏目录里实际装着哪些版本」—— 对应后端的 `active.json`。
 *
 * 和后端一样：**这是「装了没」的唯一依据**，`slot.active` 只是「最后选中的那个」。
 * 自制战役可以同时装好几部（所以 `custom` 那一格可能是多个）。
 */
const demoInstalled: Record<string, string[]> = {};
// 演示数据里 `active` 就是「装着的那一版」，先对齐，免得一进页面就自相矛盾
for (const slot of demoslots) {
  if (slot.active) demoInstalled[slot.slug] = [slot.active];
}

export function demoInstalledIn(slot: string): string[] {
  return demoInstalled[slot] ?? [];
}

export function demoIsInstalled(slot: string, variantId: string): boolean {
  return demoInstalledIn(slot).includes(variantId);
}

/** 启用 / 删除版本后改写「装了哪些」（演示里就是这份内存映射）。 */
export function setDemoInstalled(slot: string, variantIds: string[]): void {
  demoInstalled[slot] = variantIds;
}

export function delay<T>(value: T, ms = 220): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}
