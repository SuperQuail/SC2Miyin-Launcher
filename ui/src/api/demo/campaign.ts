// 演示后端：安装、战役库、启用 / 导入 / 导出与启动。
import type {
  ActivationConflict,
  ConvertedScnexus,
  ExportFormat,
  ExportLoss,
  ExportPreview,
  GameModEntry,
  Installation,
  LauncherApi,
  LibraryMod,
  MapEntry,
  MergeAddition,
  MergeChoice,
  MergeConflict,
  MergeStatus,
  ModEntry,
  PackageInspection,
  Preview,
  ScnexusCampaign,
} from "../types";

import {
  delay,
  demoInstallation,
  DEMO_LIBRARY_ROOT,
  demoInstalledIn,
  demoslots,
  nextDemoCounter,
  setDemoInstalled,
  variant,
} from "./fixtures";
import { demoEnsureSaves, demoIsolationKey } from "./saves";

/** 演示用的预检结果：一个没有元数据、靠依赖声明认出来的包。 */
const demoInspection: PackageInspection = {
  path: "D:\\Downloads\\demo-campaign.zip",
  installable: true,
  needs_password: false,
  format: "plain",
  name: null,
  author: "未知作者",
  version: null,
  description: null,
  campaign_type: "hots",
  kind: "campaign",
  id: null,
  requires: [],
  priority: null,
  payloads: [],
  identification: {
    campaign_type: "hots",
    evidence: "map_dependency",
    detail: "Swarm Story",
  },
  cover: null,
  tags: [],
  suggested_slot: "hots",
  content_root: "",
  suggested_dir_name: "demo-campaign",
  map_count: 6,
  mod_count: 0,
  entry_count: 6,
  unpacked_bytes: 512_000_000,
  issues: [
    {
      code: "NO_METADATA",
      level: "warning",
      message: "包内没有 metadata.txt / metadata.json，将按无元数据包安装",
      hint: null,
    },
  ],
};

/**
 * 演示用的**加密包**：没密码时的那份预检结果。
 *
 * 演示里任何非空密码都能解开（真后端会真的去解密校验）—— 这样点到密码框、
 * 看清交互就够，不必让人去猜一个演示密码。
 */
const lockedInspection: PackageInspection = {
  ...demoInspection,
  path: "D:\\Downloads\\demo-campaign-locked.zip",
  installable: false,
  needs_password: true,
  issues: [
    {
      code: "PASSWORD_REQUIRED",
      level: "broken",
      message: "这个压缩包有密码，请输入密码后再导入",
      hint: "请输入包作者提供的密码；忘了密码只能请作者重新打包",
    },
  ],
};

/** 让演示模式也有「正在处理」的观感。 */
/** 演示模式的库内模组。 */
function demoLibraryMods(): LibraryMod[] {
  const rows: LibraryMod[] = [];
  const push = (
    slot: string,
    slotName: string,
    variantId: string,
    variantName: string,
    origin: LibraryMod["origin"],
    names: string[],
  ) => {
    for (const name of names) {
      rows.push({
        slot,
        slot_name: slotName,
        variant_id: variantId,
        variant_name: variantName,
        path: name,
        name,
        mounted: true,
        in_game: true,
        parts: name === "Alenger" ? 18 : 1,
        origin,
        required: name === "Alenger",
        standalone_id: null,
        modid: name,
        version: "1.0",
        folder: null,
        kind: null,
        mod_record_id: null,
        source_kind: "campaign",
      });
    }
  };
  push("wol", "自由之翼", "wol-reborn", "自由之翼：重生 v1.4", "official_campaign", [
    "Alenger",
    "RebornData",
  ]);
  push("custom", "自制战役", "scmr-8", "Starcraft Mass Recall", "custom_campaign", [
    "SCMRmod",
    "SCMRlocal",
    "SCMRassets",
    "SCMRcinematics",
  ]);
  // 单独导入的
  rows.push({
    slot: "",
    slot_name: "独立模组",
    variant_id: "",
    variant_name: "",
    path: "手搓单位包",
    name: "手搓单位包",
    mounted: false,
    in_game: false,
    parts: 3,
    origin: "standalone",
    required: false,
    standalone_id: "手搓单位包",
    modid: "手搓单位包",
    version: "1.0",
    folder: "手搓单位包",
    kind: "folder",
    mod_record_id: "手搓单位包",
    source_kind: "library",
  });
  return rows;
}

/** 演示模式的游戏目录模组。 */
function demoGameMods(): GameModEntry[] {
  return [
    { display: "RebornData", name: "RebornData.SC2Mod", expanded: false, size_bytes: 12_345_678 },
    { display: "SCMRmod", name: "SCMRmod.SC2Mod", expanded: false, size_bytes: 45_678_901 },
  ];
}

/** 演示模式的地图列表 —— 照 SCMR 的样子来。 */
function demoMaps(): MapEntry[] {
  const chapters: [string, string[]][] = [
    ["1. Rebel Yell", ["Terran00t", "Terran01", "Terran02", "Terran10"]],
    ["2. Overmind", ["Zerg01", "Zerg02", "Zerg10"]],
    ["3. The Fall", ["Protoss01", "Protoss02"]],
  ];
  const maps: MapEntry[] = [];
  for (const [chapter, names] of chapters) {
    for (const name of names) {
      maps.push({
        path: chapter + "/" + name + ".SC2Map",
        name,
        chapter,
        size: 961_100,
        is_main: name === "Terran01",
      });
    }
  }
  return maps;
}

/** 演示模式的模组列表。 */
function demoMods(): ModEntry[] {
  return ["SCMRmod", "SCMRlocal", "SCMRassets", "SCMRcinematics"].map((name) => ({
    path: name + ".SC2Mod",
    name,
    mounted: true,
    in_game: true,
    parts: 1,
  }));
}

/** 演示模式：假装程序旁边那份 data 与游戏目录同级那份有重合，好让合并对话框能演一遍。 */
const demoMergeLocal = "D:\\Code\\Rust\\HSCL\\target\\debug\\data";
const demoMergeTarget = "D:\\Game\\BLZ\\miyin_launcher_data";
/** 演示里已经选过的：选完就当它并完了，下次查状态不再出现。 */
const demoMerged: Record<string, MergeChoice> = {};

/** 演示模式的合并状态：没得并可并、或都已经并完了就返回 null。 */
function demoMergeStatus(): MergeStatus | null {
  const conflicts: MergeConflict[] = [
    {
      slot: "custom",
      variant: "scmr-8",
      variant_name: "Starcraft Mass Recall",
      local_bytes: 1_414_700_000,
      target_bytes: 1_406_900_000,
    },
    {
      slot: "hots",
      variant: "hots-evolution-091",
      variant_name: "虫群之心：进化重制",
      local_bytes: 760_000_000,
      target_bytes: 812_000_000,
    },
  ].filter((conflict) => !(conflict.slot + "/" + conflict.variant in demoMerged));

  const additions: MergeAddition[] = [
    { slot: "wol", variant: "wol-reborn-150", variant_name: "自由之翼：重生 v1.5.0", bytes: 1_402_000_000 },
    { slot: "lotv", variant: "lotv-scroll", variant_name: "虚空之遗：残卷", bytes: 412_000_000 },
  ];

  if (conflicts.length === 0 && additions.length === 0) return null;
  return {
    local: demoMergeLocal,
    target: demoMergeTarget,
    additions,
    conflicts,
    keep_target: ["active.json", "installed.json", "dev", "backup"],
  };
}

/**
 * 演示模式：假装启用「虫群之心：进化重制」会和自由之翼当前启用的那一版抢位置 ——
 * 好让"启用冲突"对话框在浏览器里也能真跑（桌面壳里真冲突才出现）。
 */
export function demoActivationConflicts(slot: string, variantId: string): ActivationConflict[] {
  const wol = demoslots.find((item) => item.slug === "wol");
  const hots = demoslots.find((item) => item.slug === "hots");
  const owner = wol?.variants.find((item) => item.id === wol.active) ?? wol?.variants[0] ?? null;
  if (slot !== "hots" || !owner) return [];
  if (!hots?.variants.some((item) => item.id === variantId)) return [];

  return [
    "Mods/Alenger.SC2Mod",
    "Mods/RebornData.SC2Mod",
    "Maps/Campaign/01.SC2Map",
    "Maps/Campaign/02.SC2Map",
    "Maps/Campaign/03.SC2Map",
    "Maps/Campaign/04.SC2Map",
    "Maps/Campaign/05.SC2Map",
  ].map((target) => ({
    target,
    slot: "wol",
    slot_name: "自由之翼",
    variant: owner.id,
    variant_name: owner.name,
  }));
}

/** 演示用的星际枢纽托管战役；转换成功后从这里移走。 */
const demoScnexus: ScnexusCampaign[] = [
  {
    id: "scnexus-wol-remake",
    name: "自由之翼：星际复刻",
    author: "星际枢纽社区",
    version: "1.3.0",
    description: "在星际枢纽里安装的标准战役包。",
    campaign: "wol",
    source: "library",
    path: "D:\\Game\\BLZ\\StarCraft II\\SCNexusCampaign\\自由之翼：星际复刻",
    active: true,
    maps: 28,
    mods: 2,
    size_bytes: 1_120_000_000,
  },
  {
    id: "scnexus-hots-omega",
    name: "虫群之心：欧米伽",
    author: "Omega Works",
    version: "0.8",
    description: null,
    campaign: "hots",
    source: "library",
    path: "D:\\Game\\BLZ\\StarCraft II\\SCNexusCampaign\\虫群之心：欧米伽",
    active: false,
    maps: 12,
    mods: 1,
    size_bytes: 420_000_000,
  },
  {
    id: "scnexus-ccm-frontier",
    name: "边境之地（CCM）",
    author: null,
    version: "1.0",
    description: "星际枢纽托管的 CCM 战役。",
    campaign: "hots",
    source: "ccm",
    path: "D:\\Game\\BLZ\\StarCraft II\\Maps\\CustomCampaigns\\边境之地",
    active: false,
    maps: 12,
    mods: 0,
    size_bytes: 300_000_000,
  },
];

/** 星际枢纽格式会丢哪些东西 —— 和 core 的 loss 分类对齐。 */
function demoExportLosses(slot: string, mergePatches: boolean): ExportLoss[] {
  const losses: ExportLoss[] = [
    {
      kind: "mounted_mods",
      label: "模组的勾选配置",
      detail: "星际枢纽按它自己的规则挂载模组，不保存这份配置。",
    },
    { kind: "cover", label: "包内封面图", detail: "星际枢纽的元数据没有封面字段。" },
  ];
  if (slot === "custom") {
    losses.unshift({
      kind: "custom_campaign",
      label: "自制战役",
      detail: "星际枢纽标准只支持原版战役的改版，不支持独立的自制战役。",
    });
  }
  if (!mergePatches) {
    losses.push({
      kind: "patch",
      label: "已启用的补丁",
      detail: "没有勾选「包含补丁」，补丁不会合并进导出包。",
    });
  }
  return losses;
}

function demoExportPreview(
  slot: string,
  variantId: string,
  mergePatches: boolean,
  format: ExportFormat,
): ExportPreview {
  const found = demoslots.find((item) => item.slug === slot)?.variants.find((item) => item.id === variantId);
  const maps = found?.map_count ?? 8;
  const mods = found?.mod_count ?? 1;
  if (format === "miyin") {
    return { maps, mods, files: maps + mods + 3, losses: [], supported: true };
  }
  return {
    maps,
    mods,
    files: maps + mods,
    losses: demoExportLosses(slot, mergePatches),
    supported: slot !== "custom",
  };
}

export const campaignDemo = {
  detectInstallation: () => delay(demoInstallation as Installation | null),
  setInstallation: () => delay(demoInstallation),
  libraryRoot: () => delay(DEMO_LIBRARY_ROOT),
  listSlots: () =>
    delay(
      demoslots.map((slot) => {
        const enabled = demoInstalledIn(slot.slug);
        return {
          ...slot,
          variants: slot.variants.map((item) => ({ ...item })),
          enabled_variants: [...enabled],
          vanilla_enabled: enabled.length === 0,
        };
      }),
    ),
  // 演示模式：假的合并案只在 ?merge=1 时给 —— 否则一进演示就被弹窗挡住，什么都看不见
  mergeStatus: () =>
    new URLSearchParams(location.search).has("merge") ? delay(demoMergeStatus()) : delay(null),
  mergeApply: (choices) =>
    delay(
      (() => {
        // 记下结果：下次查状态时这些冲突就算并完了
        for (const [key, choice] of Object.entries(choices)) demoMerged[key] = choice;
      })(),
    ),

  inspectPackage: (path) =>
    delay({
      path,
      installable: true,
      needs_password: false,
      format: "ccm",
      name: "演示战役包",
      author: "Demo",
      version: "1.0",
      description: null,
      campaign_type: "wol",
      kind: "campaign",
      id: null,
      requires: [],
      priority: null,
      payloads: [],
      identification: null,
      cover: null,
      tags: ["演示"],
      suggested_slot: "wol",
      content_root: "Demo",
      suggested_dir_name: "演示战役包",
      map_count: 12,
      mod_count: 2,
      entry_count: 148,
      unpacked_bytes: 512_000_000,
      issues: [],
    } satisfies PackageInspection),
  // 演示模式也给一份预演：真后端只要版本里有载荷就会算出来。返回 null 会让
  // "启用 → 预演确认"这一步在演示里永远被跳过，看不出真实手感。
  previewActivation: () =>
    delay({
      add: [
        {
          target: "Maps/Campaign/demo.SC2Map",
          existing_bytes: 0,
          incoming_bytes: 961_100,
          owner: null,
        },
      ],
      overwrite: [],
      takeover: [],
      delete: [],
      bytes: 961_100,
    } satisfies Preview),
  activateVariant: (slot, variantId) =>
    delay(
      (() => {
        const target = demoslots.find((item) => item.slug === slot);
        if (target) {
          target.active = variantId;
          // 「装了没」记在清单里（演示模式就是这份内存映射）
          if (variantId === null) {
            setDemoInstalled(slot, []);
          } else if (slot === "custom") {
            // 自制战役允许同时装好几部
            const current = demoInstalledIn(slot);
            setDemoInstalled(slot, current.includes(variantId) ? current : [...current, variantId]);
          } else {
            setDemoInstalled(slot, [variantId]);
          }
          const found = target.variants.find((item) => item.id === variantId);
          target.active_name = found ? found.name : null;
          // 启用战役时存档跟着走（模拟后端的 ensure_for_slot）：
          // 官方战役继承当前这份，自制战役开一个空组
          if (variantId !== null) {
            const key = demoIsolationKey(slot, variantId);
            const variantName = found?.name ?? variantId;
            // 组名和后端一致：自制用版本名；官方改版「战役名 · 版本名」
            const label = slot === "custom" ? variantName : target.display_name + " · " + variantName;
            demoEnsureSaves(key, label);
          } else {
            // 切回原版：键就是槽位本身（后端 ensure_for_slot 会复用「原版」老组并补绑定）
            demoEnsureSaves(slot, target.display_name);
          }
        }
        return [] as string[];
      })(),
    ),

  // 演示：把所有战役的「启用」状态清空
  deactivateAll: () =>
    delay(
      (() => {
        for (const item of demoslots) {
          item.active = null;
          item.active_name = null;
        }
        return [] as string[];
      })(),
    ),

  deleteVariant: (slot, variantId) =>
    delay(
      (() => {
        const target = demoslots.find((item) => item.slug === slot);
        if (target) {
          target.variants = target.variants.filter((item) => item.id !== variantId);
          if (target.active === variantId) {
            target.active = null;
            target.active_name = null;
          }
        }
        return undefined;
      })(),
    ),

  launchGame: () => delay(undefined),
  revealPath: () => delay(undefined),
  // 演示「密码」那条路：选到的就是一个加密包
  pickPackage: () => delay("D:\\Downloads\\demo-campaign-locked.zip"),
  pickPackageFolder: () => delay("D:\\Downloads\\MyCampaign"),
  pickGameDirectory: () => delay("D:\\Game\\BLZ\\StarCraft II"),
  // 演示数据没有真实图片文件，统一返回 null，界面会退回官方美术
  variantCover: () => delay(null),
  listInstalledCampaigns: async () => [],
  listScnexusCampaigns: () => delay(demoScnexus.map((item) => ({ ...item }))),
  convertScnexusCampaign: (id, removeSource) =>
    delay(
      (() => {
        const index = demoScnexus.findIndex((item) => item.id === id);
        if (index < 0) throw new Error("这个星际枢纽战役已经不在列表里了");
        const [found] = demoScnexus.splice(index, 1);
        // 和后端一致：落点按 campaign 字段定，CCM 条目也归到对应官方资料片
        const slot = found.campaign;
        const created = variant(
          found.name + "（已转换）",
          found.name,
          found.author ?? "未知作者",
          found.version ?? "1.0",
          found.maps,
          found.size_bytes,
        );
        const target = demoslots.find((item) => item.slug === slot) ?? demoslots[0];
        target.variants.unshift(created);
        void removeSource;
        return { slot, variant: created, warnings: [] } as ConvertedScnexus;
      })(),
    ),
  prepareImport: (path, entry, password) =>
    delay({
      path,
      // 文件名带 locked 的演示加密包：给了密码就当解开了
      inspection: path.includes("locked") && !password ? lockedInspection : demoInspection,
      // 自制战役入口：归属由入口定死，不演示识别
      source: (entry === "custom" ? "manual" : "inferred") as "manual" | "inferred",
      slot: entry === "custom" ? "custom" : "hots",
      conflict: null,
    }),

  importPackageWith: (path, slot, mode) =>
    delay(
      (() => {
        const id = nextDemoCounter();
        const created = variant(
          "导入的战役包 v" + id,
          "导入的战役包",
          "未知作者",
          "1." + id,
          id,
          512_000_000,
        );
        const target = demoslots.find((item) => item.slug === slot) ?? demoslots[0];
        if (mode === "overwrite" && target.variants.length > 0) {
          target.variants[0] = created;
        } else {
          target.variants.unshift(created);
        }
        void path;
        return created;
      })(),
    ),

  updateVariant: (_slot, variantId, changes) =>
    delay(
      (() => {
        for (const item of demoslots) {
          const found = item.variants.find((entry) => entry.id === variantId);
          if (found) {
            if (changes.name !== undefined) found.name = changes.name;
            if (changes.author !== undefined) found.author = changes.author;
            if (changes.description !== undefined) found.description = changes.description;
            return found;
          }
        }
        throw new Error("找不到这个版本");
      })(),
    ),
  variantMaps: (_slot, _variantId) => delay(demoMaps()),
  variantMods: (_slot, _variantId) => delay(demoMods()),
  setMountedMods: (_slot, _variantId, _mods) => delay({} as never),
  setMainMap: (_slot, _variantId, _map) => delay({} as never),
  mainMapChoice: () => delay({ path: "1. Rebel Yell/Terran01.SC2Map", automatic: false, warning: null }),
  resolveSiteUrl: async (https, _http) => https,
  updateVariantFromPackage: (slot, variantId, path) =>
    delay(
      (() => {
        const id = nextDemoCounter();
        const created = variant(
          "更新后的战役包 v" + id,
          "更新后的战役包",
          "未知作者",
          "2." + id,
          id,
          512_000_000,
        );
        const target = demoslots.find((item) => item.slug === slot) ?? demoslots[0];
        // 演示里"更新"= 换掉指定的那一版（没指定就换当前那一版）
        const at = variantId ? target.variants.findIndex((item) => item.id === variantId) : 0;
        if (at >= 0) target.variants[at] = created;
        else target.variants.unshift(created);
        void path;
        // 演示里假装新包带了两个新模组，好让"要不要一起启用"那块也看得见
        return { variant: created, addedMods: ["SCMR_Alarak.SC2Mod", "SCMR_Tychus.SC2Mod"] };
      })(),
    ),
  launchGameWithMap: async (_slot, _variantId, map) => ({
    map,
    guidance: "（演示模式）已经交给星际争霸，正在加载这张地图。",
  }),
  openMapInEditor: (_slot, _variantId, map) =>
    delay({
      editor: "D:\\Game\\BLZ\\StarCraft II\\Support64\\SC2Editor_x64.exe",
      map,
      guidance: "编辑器已打开这张地图，按 Ctrl+F9（菜单「测试文档」）就能进入游戏。",
    }),
  variantDoc: () => delay(null),
  listLibraryMods: () => delay(demoLibraryMods()),
  listGameMods: () => delay(demoGameMods()),
  readDoc: () => delay(new ArrayBuffer(0)),
  previewExportVariant: (slot, variantId, mergePatches, format) =>
    delay(demoExportPreview(slot, variantId, mergePatches, format)),
  exportVariant: (slot, variantId, destination, mergePatches, format) =>
    delay(
      (() => {
        const preview = demoExportPreview(slot, variantId, mergePatches, format ?? "miyin");
        return {
          path: destination,
          files: preview.files,
          maps: preview.maps,
          mods: preview.mods,
          expanded: [],
          patches: mergePatches ? ["幼儿园补丁 V3"] : [],
          format: format ?? "miyin",
          losses: preview.losses,
        };
      })(),
    ),
  pickExportPath: (defaultName) => delay("D:\\\\Downloads\\\\" + defaultName + ".zip"),
} satisfies Partial<LauncherApi>;
