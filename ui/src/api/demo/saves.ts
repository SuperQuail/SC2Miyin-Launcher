/**
 * 演示后端：存档隔离与存档组。
 *
 * 开关、组列表、改归属、切换、存回都真的会动这份内存数据；
 * 「官方战役组继承当前存档 / 自制战役是空组」也照着后端语义模拟了一遍。
 */
import type {
  BankEntry,
  LauncherApi,
  SaveBackup,
  SaveEntry,
  SaveInventory,
  SaveIsolation,
  SaveSet,
  SaveTarget,
} from "../types";

import { demoInstalledIn, demoIsInstalled, demoslots } from "./fixtures";

/**
 * 演示模式的存档隔离模型（内存里那份）。
 *
 * 只为了把存档页跑起来：开关、组列表、改归属、切换、存回都真的会动这份内存数据；
 * "官方战役组继承当前存档 / 自制战役是空组"也照着后端语义模拟了一遍，
 * 好让验收里那三条在浏览器里看得见（**内容**是假的，目录和文件都是编的）。
 */
const demoSaveGroups: SaveBackup[] = [
  { name: "原版-1790000000", label: "原版", bytes: 1_842_000, files: 3, auto: false },
  // 第二个组是给「组详情 → 挪到…」演示用的：两个组各有一份存档，挪动看得出来
  {
    name: "自由之翼 · 重生 v1.4.2-1790000100",
    label: "自由之翼 · 重生 v1.4.2",
    bytes: 1_220_000,
    files: 2,
    auto: false,
  },
];
const demoSavesState: SaveIsolation = {
  // 初始要跟**真实默认**一致：关着（见 saves.rs 的 Isolation::default）。
  // 组照常列出来，用户在设置页打开开关之后切组 / 归属的演示都能走（demoEnsureSaves 认 enabled）。
  enabled: false,
  // 已经归过档（跟真机上「打开过、后来关掉」的状态一样），所以打开开关不会再收一份「原版」
  active: "原版-1790000000",
  // 键的规则和后端一致：原版 = 槽位；有版本 = 槽位/身份
  assignments: { "自由之翼 · 重生 v1.4.2-1790000100": "wol/自由之翼：重生 v1.4.2" },
};
/** 游戏里这份 Banks 现在长什么样（切组会换）。 */
let demoBanks: SaveSet = {
  files: [
    { name: "ENS.SC2Bank", bytes: 611_000 },
    { name: "Liberty.SC2Bank", bytes: 620_000 },
    { name: "Swarm.SC2Bank", bytes: 611_000 },
  ],
  bytes: 1_842_000,
  missing: false,
};

function demoIsolation(): SaveIsolation {
  return { ...demoSavesState, assignments: { ...demoSavesState.assignments } };
}

/**
 * 演示模式的**组内文件**：名字是真的、字节是编的（按组的平均大小）。
 * 挪动会真的改这些数组，这样组详情弹层在演示里也能看到"少了一份 / 多了一份"。
 */
const demoGroupFiles: Record<string, string[]> = {};

function demoFilesOf(name: string): string[] {
  if (!demoGroupFiles[name]) {
    const group = demoSaveGroups.find((item) => item.name === name);
    demoGroupFiles[name] = Array.from(
      { length: group?.files ?? 0 },
      // 带上组名：两个组不能有同名文件，否则「挪到…」永远撞车，演示看不出效果
      (_, index) => (group?.label ?? name) + (index + 1) + ".SC2Bank",
    );
  }
  return demoGroupFiles[name];
}

/** 按组的体积 / 文件数编一份"Banks 长这样"—— 演示模式只求自洽。 */
function demoBanksOf(group: SaveBackup): SaveSet {
  const each = group.files > 0 ? Math.round(group.bytes / group.files) : 0;
  return {
    files: Array.from({ length: group.files }, (_, index) => ({
      name: "Bank" + (index + 1) + ".SC2Bank",
      bytes: each,
    })),
    bytes: group.bytes,
    missing: false,
  };
}

/**
 * 演示模式的「真实存档」清单：两个账号、含 `Unsaved/Campaign`，十几份存档 + 几份 Banks。
 *
 * 目录形状照真机来（`Accounts/<账号>/<地域>/Saves/**`、`Banks/<地图 key>/`）——
 * 界面在演示模式里看到的层级、账号分组和真机一致。
 */
function demoSaveInventory(): SaveInventory {
  const base = "C:\\Users\\演示\\Documents\\StarCraft II";
  const main = { account: "123456789", region: "美服-123456789" };
  const second = { account: "987654321", region: "亚服-987654321" };

  const save = (
    owner: { account: string; region: string },
    folder: string,
    name: string,
    modified: number,
    bytes: number,
  ): SaveEntry => ({
    ...owner,
    folder,
    name,
    path: base + "\\Accounts\\" + owner.account + "\\" + owner.region + "\\" + folder.replace(/\//g, "\\") + "\\" + name + ".SC2Save",
    bytes,
    modified,
  });

  const saves: SaveEntry[] = [
    save(main, "Saves/Unsaved/Campaign", "虚空之遗 战役 08", 1_729_800_400, 52_300),
    save(main, "Saves/Unsaved/Campaign", "虚空之遗 战役 07", 1_729_700_100, 51_800),
    save(main, "Saves/Unsaved/Campaign", "虚空之遗 战役 06", 1_729_600_900, 50_100),
    save(main, "Saves/Campaign", "虫群之心 战役 04", 1_729_500_300, 47_700),
    save(main, "Saves/Campaign", "虫群之心 战役 03", 1_729_400_200, 46_900),
    save(main, "Saves", "自由之翼 战役 02", 1_729_300_500, 44_200),
    save(main, "Saves", "自由之翼 战役 01", 1_729_200_700, 43_800),
    save(main, "Saves/Unsaved/Campaign", "诺娃 战役 02", 1_729_100_400, 49_600),
    save(main, "Saves/Unsaved/Campaign", "诺娃 战役 01", 1_729_000_100, 48_900),
    save(second, "Saves/Campaign", "虚空之遗 战役 03", 1_728_900_800, 45_300),
    save(second, "Saves/Unsaved/Campaign", "虚空之遗 战役 02", 1_728_800_600, 44_700),
    save(second, "Saves", "虫群之心 战役 01", 1_728_700_200, 41_200),
  ];
  saves.sort((a, b) => b.modified - a.modified);

  const bank = (map: string, name: string, modified: number, bytes: number): BankEntry => ({
    ...main,
    map,
    name,
    path: base + "\\Accounts\\" + main.account + "\\" + main.region + "\\Banks\\" + map + "\\" + name + ".SC2Bank",
    bytes,
    modified,
  });

  const banks: BankEntry[] = [
    bank("虚空之遗战役", "VoidCampaign", 1_729_800_400, 12_300),
    bank("虫群之心战役", "SwarmCampaign", 1_729_500_300, 9_800),
    bank("自由之翼战役", "LibertyCampaign", 1_729_300_500, 8_400),
    bank("诺娃战役", "NovaCampaign", 1_729_100_400, 7_600),
    bank("虚空之遗战役", "VoidCampaign.backup", 1_728_500_000, 11_900),
  ];
  banks.sort((a, b) => b.modified - a.modified);

  return { accounts: [main.account, second.account], saves, banks };
}

/**
 * 演示模式的隔离键 —— 和后端同规则：**有版本就是 `槽位/身份`**（官方改版也一样），
 * 原版才是槽位本身；身份优先注册 ID、没有就用版本 id。
 */
export function demoIsolationKey(slot: string, variantId: string): string {
  const item = demoslots
    .find((entry) => entry.slug === slot)
    ?.variants.find((entry) => entry.id === variantId);
  return slot + "/" + (item?.registration_id ?? variantId);
}

/** 演示模式里模拟 `ensure_for_slot`：官方战役继承当前存档，自制战役每部一个空组。 */
export function demoEnsureSaves(key: string, label: string): void {
  if (!demoSavesState.enabled) return;
  const assigned = Object.entries(demoSavesState.assignments).find(([, value]) => value === key)?.[0];
  if (assigned) {
    demoSavesState.active = assigned;
    const group = demoSaveGroups.find((item) => item.name === assigned);
    if (group) demoBanks = demoBanksOf(group);
    return;
  }
  const custom = key.startsWith("custom/") || key === "custom";
  const campaign = !custom && !key.includes("/");
  // 原版键：先复用还没归属的老组（「原版」优先）—— 它的内容就是原版进度，
  // 别新建一个空组把它晾在一边（后端 ensure_for_slot 同规则）
  if (campaign) {
    const bound = (name: string) => name in demoSavesState.assignments;
    const legacy =
      demoSaveGroups.find((item) => !item.auto && item.label === "原版" && !bound(item.name)) ??
      demoSaveGroups.find((item) => !item.auto && !bound(item.name));
    if (legacy) {
      demoSavesState.assignments[legacy.name] = key;
      demoSavesState.active = legacy.name;
      demoBanks = demoBanksOf(legacy);
      return;
    }
  }
  // 只有「还没归过档」才收编当前这份；已经有组在用就从空组开始（别串档）
  const adopt = !custom && demoSavesState.active === null;
  const name = label + "-" + (1790000200 + demoSaveGroups.length);
  demoSaveGroups.push({
    name,
    label,
    bytes: adopt ? demoBanks.bytes : 0,
    files: adopt ? demoBanks.files.length : 0,
    auto: false,
  });
  demoSavesState.assignments[name] = key;
  demoSavesState.active = name;
  if (adopt) demoBanks = { ...demoBanks };
  else demoBanks = { files: [], bytes: 0, missing: false };
}

/** 演示模式的归属候选：和后端 `targets()` 同规则（官方：原版 + 每个改版；自制每部一条）。 */
function demoTargets(): SaveTarget[] {
  const out: SaveTarget[] = [];
  // 键相同的只留一条（同一部战役的多个版本共用一个组）；重名补版本号/身份，
  // 否则下拉里会出现两条一模一样的选项 —— 和 core 里 named_targets 的规则一致
  const push = (key: string, label: string, enabled: boolean, version?: string | null) => {
    if (out.some((item) => item.key === key)) return;
    const clash = out.some((item) => item.label === label);
    const suffix = clash
      ? version && version.trim()
        ? " " + version
        : "（" + key.split("/").pop() + "）"
      : "";
    out.push({ key, label: label + suffix, enabled });
  };
  // 官方：每个槽位先「（原版）」（键 = 槽位），再跟上它的每个改版（同一部只一条）
  for (const slot of demoslots.filter((item) => item.slug !== "custom")) {
    // 原版「装着」= 这个槽位一个版本都没装
    push(slot.slug, slot.display_name + "（原版）", demoInstalledIn(slot.slug).length === 0);
    for (const item of slot.variants) {
      push(
        demoIsolationKey(slot.slug, item.id),
        slot.display_name + " · " + item.name,
        demoIsInstalled(slot.slug, item.id),
        item.version,
      );
    }
  }
  // 自制：每部一条（可以同时好几部 true）
  const custom = demoslots.find((slot) => slot.slug === "custom");
  for (const item of custom?.variants ?? []) {
    push(
      demoIsolationKey("custom", item.id),
      "自制战役 · " + item.name,
      demoIsInstalled("custom", item.id),
      item.version,
    );
  }
  return out;
}

/** 演示模式：把一份存档从一个组挪到另一个组（同名不覆盖、目标不存在就建）。 */
function demoMoveFile(from: string, file: string, to: string): void {
  if (from === to) throw new Error("源组和目标组是同一个");
  const source = demoSaveGroups.find((item) => item.name === from);
  if (!source) throw new Error("没有这个存档组：" + from);
  const names = demoFilesOf(from);
  if (!names.includes(file)) throw new Error("找不到这份存档：" + file);
  const target = demoSaveGroups.find((item) => item.name === to);
  if (target && demoFilesOf(to).includes(file)) {
    throw new Error("「" + to + "」里已经有同名存档 " + file + "，没有覆盖");
  }
  names.splice(names.indexOf(file), 1);
  source.files = names.length;
  if (target) {
    demoFilesOf(to).push(file);
    target.files = demoFilesOf(to).length;
  } else {
    demoSaveGroups.push({ name: to, label: to, files: 1, bytes: 0, auto: false });
    demoGroupFiles[to] = [file];
  }
}

export const savesDemo = {
  saveIsolation: async () => demoIsolation(),
  setSaveIsolation: async (enabled) => {
    demoSavesState.enabled = enabled;
    return demoIsolation();
  },
  saveCurrentSaves: async () => demoIsolation(),
  switchSaveProfile: async (name) => {
    const group = demoSaveGroups.find((item) => item.name === name);
    demoSavesState.active = name;
    if (group) demoBanks = demoBanksOf(group);
    return demoIsolation();
  },
  assignSaveProfile: async (name, slot) => {
    if (slot) {
      // 一个战役只占一个组：先把别的组身上这个归属摘掉
      for (const key of Object.keys(demoSavesState.assignments)) {
        if (demoSavesState.assignments[key] === slot) delete demoSavesState.assignments[key];
      }
      demoSavesState.assignments[name] = slot;
    } else {
      delete demoSavesState.assignments[name];
    }
    return demoIsolation();
  },
  saveIntoProfile: async (name) => {
    const group = demoSaveGroups.find((item) => item.name === name);
    if (group) {
      group.bytes = demoBanks.bytes;
      group.files = demoBanks.files.length;
    }
    demoSavesState.active = name;
    return demoIsolation();
  },
  savesTargets: async () => demoTargets(),
  saveGroupFiles: async (name) => {
    const group = demoSaveGroups.find((item) => item.name === name);
    if (!group) throw new Error("没有这个存档组：" + name);
    const names = demoFilesOf(name);
    const each = names.length > 0 ? Math.round(group.bytes / names.length) : 0;
    return names.map((file) => ({ name: file, bytes: each }));
  },
  moveSaveFile: async (from, file, to) => demoMoveFile(from, file, to),
  moveSaveToKey: async (from, file, key) => {
    // 已经有组绑着这个键：挪进那个组
    const bound = Object.entries(demoSavesState.assignments).find(([, value]) => value === key)?.[0];
    if (bound) {
      demoMoveFile(from, file, bound);
      return bound;
    }

    // 没有就按 targets 的 label 建一个新组，自动绑上这个键
    const target = demoTargets().find((item) => item.key === key);
    if (!target) throw new Error("未知的归属：" + key);
    const name = target.label + "-" + (1790000500 + demoSaveGroups.length);
    demoSaveGroups.push({ name, label: target.label, files: 0, bytes: 0, auto: false });
    demoGroupFiles[name] = [];
    demoSavesState.assignments[name] = key;
    demoMoveFile(from, file, name);
    return name;
  },
  listSaves: async () => demoBanks,
  backupSaves: async (label) => {
    const name = label + "-" + (1790000300 + demoSaveGroups.length);
    demoSaveGroups.push({
      name,
      label,
      bytes: demoBanks.bytes,
      files: demoBanks.files.length,
      auto: false,
    });
    return name;
  },
  listSaveBackups: async () =>
    demoSaveGroups.map((item) => ({
      ...item,
      // 和后端一致：没有绑定的组标 legacy（名字不代表内容）
      legacy: !item.auto && !(item.name in demoSavesState.assignments),
    })),
  scanSaves: async () => demoSaveInventory(),
  restoreSaves: async (name) => {
    // 和后端一样：还原前先留一份自动备份（它不是"用户意义上的组"）
    const safety = "还原前-" + (1790000400 + demoSaveGroups.length);
    demoSaveGroups.push({
      name: safety,
      label: "还原前",
      bytes: demoBanks.bytes,
      files: demoBanks.files.length,
      auto: true,
    });
    const group = demoSaveGroups.find((item) => item.name === name);
    if (group) demoBanks = demoBanksOf(group);
    return safety;
  },
} satisfies Partial<LauncherApi>;
