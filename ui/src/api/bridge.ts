/**
 * 与 Rust 后端通信的桥接层。
 *
 * 如果没有 Tauri 运行时（例如直接用浏览器打开 `vite dev`），会自动退回**演示数据**，
 * 这样界面可以脱离桌面壳独立开发与截图验证。演示数据是可交互的：
 * 导入、切换、删除都会即时反映在界面上。
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  BoundPatch,
  DetectedProxy,
  NetworkSettings,
  DocInfo,
  EditorLaunch,
  GameModEntry,
  LibraryMod,
  ModComparison,
  ModImport,
  ModPreview,
  StandaloneMod,
  ToolEnsure,
  ToolRelease,
  ToolStatus,
  MainMapChoice,
  MapEntry,
  ModEntry,
  Staged,
  UpdateCheck,
  UpdateProgress,
  Composition,
  ExportReport,
  ImportPreview,
  Installation,
  LauncherApi,
  Preview,
  SaveBackup,
  SaveSet,
  PackageInspection,
  Patch,
  SlotView,
  Variant,
} from "./types";

/** 是否运行在 Tauri 桌面壳里。 */
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** 真实后端。 */
const desktop: LauncherApi = {
  detectInstallation: () => invoke<Installation | null>("detect_installation"),
  setInstallation: (path) => invoke<Installation>("set_installation", { path }),
  libraryRoot: () => invoke<string>("library_root"),
  listSlots: () => invoke<SlotView[]>("list_slots"),
  inspectPackage: (path) => invoke<PackageInspection>("inspect_package", { path }),
  sc2Running: () => invoke<string | null>("sc2_running"),
  listSaves: () => invoke<SaveSet>("list_saves"),
  backupSaves: (label) => invoke<string>("backup_saves", { label }),
  listSaveBackups: () => invoke<SaveBackup[]>("list_save_backups"),
  restoreSaves: (name) => invoke<string>("restore_saves", { name }),

  /** 启用前先看：会往游戏目录里放什么、覆盖什么、删什么（不写盘）。 */
  previewActivation: (slot, variantId) =>
    invoke<Preview>("preview_activation", { slot, variantId }),
  activateVariant: (slot, variantId) =>
    invoke<string[]>("activate_variant", { slot, variantId }),
  deleteVariant: (slot, variantId) => invoke<void>("delete_variant", { slot, variantId }),
  launchGame: () => invoke<void>("launch_game"),
  revealPath: (path) => invoke<void>("reveal_path", { path }),
  pickPackage: () => invoke<string | null>("pick_package"),
  pickGameDirectory: () => invoke<string | null>("pick_game_directory"),
  variantCover: (slot, variantId) => invoke<string | null>("variant_cover", { slot, variantId }),

  prepareImport: (path, entry) =>
    invoke<ImportPreview>("prepare_import", { path, entry: entry ?? null }),
  importPackageWith: (path, slot, mode) =>
    invoke<Variant>("import_package", { path, slot, mode }),
  updateVariant: (slot, variantId, changes) =>
    invoke<Variant>("update_variant", { slot, variantId, changes }),

  listPatches: () => invoke<Patch[]>("list_patches"),
  listBindings: (slot) => invoke<BoundPatch[]>("list_bindings", { slot }),
  listAvailablePatches: (slot) => invoke<BoundPatch[]>("list_available_patches", { slot }),
  importPatch: (path) => invoke<Patch>("import_patch", { path }),
  autoBindPatches: () => invoke<string[]>("auto_bind_patches"),
  bindPatch: (slot, patchId) => invoke<void>("bind_patch", { slot, patchId }),
  unbindPatch: (slot, patchId) => invoke<void>("unbind_patch", { slot, patchId }),
  configurePatch: (slot, patchId, enabled, priority) =>
    invoke<unknown>("configure_patch", { slot, patchId, enabled, priority }),
  deletePatch: (patchId) => invoke<void>("delete_patch", { patchId }),
  updatePatch: (patchId, changes) => invoke<Patch>("update_patch", { patchId, changes }),
  exportPatch: (patchId, destination) =>
    invoke<ExportReport>("export_patch", { patchId, destination }),
  previewComposition: (slot, variantId) =>
    invoke<Composition>("preview_composition", { slot, variantId }),

  exportVariant: (slot, variantId, destination, mergePatches) =>
    invoke<ExportReport>("export_variant", { slot, variantId, destination, mergePatches }),
  pickExportPath: (defaultName) => invoke<string | null>("pick_export_path", { defaultName }),

  variantMaps: (slot, variantId) => invoke<MapEntry[]>("variant_maps", { slot, variantId }),
  variantMods: (slot, variantId) => invoke<ModEntry[]>("variant_mods", { slot, variantId }),
  setMountedMods: (slot, variantId, mods) =>
    invoke<Variant>("set_mounted_mods", { slot, variantId, mods }),
  setMainMap: (slot, variantId, map) =>
    invoke<Variant>("set_main_map", { slot, variantId, map }),
  mainMapChoice: (slot, variantId) =>
    invoke<MainMapChoice>("main_map_choice", { slot, variantId }),
  openMapInEditor: (slot, variantId, map) =>
    invoke<EditorLaunch>("open_map_in_editor", { slot, variantId, map }),
  variantDoc: (slot, variantId) => invoke<DocInfo | null>("variant_doc", { slot, variantId }),
  listLibraryMods: () => invoke<LibraryMod[]>("list_library_mods"),
  listGameMods: () => invoke<GameModEntry[]>("list_game_mods"),
  listStandaloneMods: () => invoke<StandaloneMod[]>("list_standalone_mods"),
  windowMinimize: () => invoke<void>("window_minimize"),
  windowToggleMaximize: () => invoke<boolean>("window_toggle_maximize"),
  windowClose: () => invoke<void>("window_close"),
  windowIsMaximized: () => invoke<boolean>("window_is_maximized"),

  listTools: () => invoke<ToolStatus[]>("list_tools"),
  ensureTool: (id) => invoke<ToolEnsure>("ensure_tool", { id }),
  toolReleases: (id) => invoke<ToolRelease[]>("tool_releases", { id }),
  installTool: (id, version) => invoke<ToolStatus>("install_tool", { id, version }),
  uninstallTool: (id) => invoke<void>("uninstall_tool", { id }),
  openToolRepo: (id) => invoke<void>("open_tool_repo", { id }),
  pickModSource: (kind) => invoke<string | null>("pick_mod_source", { kind }),
  previewMod: (path) => invoke<ModPreview>("preview_mod", { path }),
  importMod: (path, mode) => invoke<ModImport>("import_mod", { path, mode: mode ?? null }),
  updateMod: (id, changes) => invoke<StandaloneMod>("update_mod", { id, changes }),
  editMod: (recordId, identity, changes) =>
    invoke<StandaloneMod>("edit_mod", { recordId, identity, changes }),
  removeMod: (id) => invoke<void>("remove_mod", { id }),
  exportMod: (id) => invoke<string>("export_mod", { id }),
  exportMods: (ids) => invoke<string>("export_mods", { ids }),
  compareMods: (before, after) => invoke<ModComparison>("compare_mods", { before, after }),
  setModEnabled: (id, enabled) => invoke<StandaloneMod>("set_mod_enabled", { id, enabled }),
  readDoc: (slot, variantId) =>
    invoke<ArrayBuffer>("read_doc", { slot, variantId }),

  appVersion: () => invoke<string>("app_version"),
  networkSettings: () => invoke<NetworkSettings>("network_settings"),
  setNetworkSettings: (settings) => invoke<void>("set_network_settings", { settings }),
  detectedProxy: () => invoke<DetectedProxy | null>("detected_proxy"),
  checkUpdate: () => invoke<UpdateCheck>("check_update"),
  downloadUpdate: (version) => invoke<Staged>("download_update", { version }),
  applyUpdate: () => invoke<void>("apply_update"),
  openUrl: (url) => invoke<void>("open_url", { url }),
  onUpdateProgress: async (handler) =>
    listen<UpdateProgress>("update://progress", (event) => handler(event.payload)),
  onUpdateLog: async (handler) =>
    listen<string>("update://log", (event) => handler(event.payload)),
};

/** 演示数据：数值取自开发机的真实安装。 */
const demoInstallation: Installation = {
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

const DEMO_LIBRARY_ROOT = "D:\\Code\\Rust\\HSCL\\target\\debug\\data";

/** 演示用的补丁：多一个 bound_to 记录它挂在谁身上。 */
type DemoPatch = BoundPatch & { bound_to: string | null };

/** 演示补丁库。 */
const demopatches: DemoPatch[] = [
  {
    id: "幼儿园补丁",
    name: "肉鸽之心 幼儿园补丁 V3",
    author: "小白的心之所念",
    version: "3.0",
    description: "给大部分单位加了幼儿园标签以进行强化",
    registration_id: "xiaoBai.kindergarten-v3",
    priority: 100,
    requires: ["KerriganRogue"],
    payloads: [],
    imported_at: 1790000000,
    size_bytes: 146_800,
    mod_count: 1,
    enabled: true,
    matched: false,
    bound_to: "hots",
  },
  {
    id: "优化覆盖补丁",
    name: "肉鸽之翼之遗 优化覆盖补丁",
    author: "未知作者",
    version: "1.0",
    description: "天赋优化覆盖",
    registration_id: null,
    priority: 200,
    requires: [],
    payloads: [],
    imported_at: 1790000100,
    size_bytes: 534_000,
    mod_count: 1,
    enabled: false,
    matched: false,
    bound_to: null,
  },
];

/** 演示用的预检结果：一个没有元数据、靠依赖声明认出来的包。 */
const demoInspection: PackageInspection = {
  path: "D:\\Downloads\\demo-campaign.zip",
  installable: true,
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

function variant(
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
const demoslots: SlotView[] = [
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
    ],
    active: "Starcraft Mass Recall 8.0",
    active_name: "Starcraft Mass Recall",
    notice: null,
  },
];

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
    parts: 1,
  }));
}

function delay<T>(value: T, ms = 220): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

let counter = 0;

const demo: LauncherApi = {
  detectInstallation: () => delay(demoInstallation as Installation | null),
  setInstallation: () => delay(demoInstallation),
  libraryRoot: () => delay(DEMO_LIBRARY_ROOT),
  listSlots: () => delay(demoslots.map((slot) => ({ ...slot }))),

  inspectPackage: (path) =>
    delay({
      path,
      installable: true,
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

  sc2Running: async () => null,
  listSaves: async () => ({ files: [], bytes: 0, missing: true }),
  backupSaves: async () => {
    throw new Error("演示模式没有存档可备份");
  },
  listSaveBackups: async () => [],
  restoreSaves: async () => {
    throw new Error("演示模式没有备份可还原");
  },
  previewActivation: async () => null,
  activateVariant: (slot, variantId) =>
    delay(
      (() => {
        const target = demoslots.find((item) => item.slug === slot);
        if (target) {
          target.active = variantId;
          const found = target.variants.find((item) => item.id === variantId);
          target.active_name = found ? found.name : null;
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
  pickPackage: () => delay("D:\\Downloads\\demo-campaign.zip"),
  pickGameDirectory: () => delay("D:\\Game\\BLZ\\StarCraft II"),
  // 演示数据没有真实图片文件，统一返回 null，界面会退回官方美术
  variantCover: () => delay(null),

  prepareImport: (path, entry) =>
    delay({
      path,
      inspection: demoInspection,
      // 自制战役入口：归属由入口定死，不演示识别
      source: (entry === "custom" ? "manual" : "inferred") as "manual" | "inferred",
      slot: entry === "custom" ? "custom" : "hots",
      conflict: null,
    }),

  importPackageWith: (path, slot, mode) =>
    delay(
      (() => {
        counter += 1;
        const created = variant(
          "导入的战役包 v" + counter,
          "导入的战役包",
          "未知作者",
          "1." + counter,
          counter,
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

  listPatches: () => delay(demopatches.slice()),
  listBindings: (slot) => delay(demopatches.filter((item) => item.bound_to === slot)),
  listAvailablePatches: (slot) => delay(demopatches.filter((item) => item.bound_to !== slot)),
  importPatch: () =>
    delay(
      (() => {
        counter += 1;
        const created: DemoPatch = {
          id: "补丁-" + counter,
          name: "导入的补丁 v" + counter,
          author: "未知作者",
          version: "1.0",
          description: null,
          registration_id: null,
          priority: 100,
          requires: [],
          payloads: [],
          imported_at: 1790000200,
          size_bytes: 200_000,
          mod_count: 1,
          enabled: false,
          matched: false,
          bound_to: null,
        };
        demopatches.push(created);
        return created;
      })(),
    ),
  autoBindPatches: () => delay([] as string[]),
  bindPatch: (slot, patchId) =>
    delay(
      ((found) => {
        if (found) {
          found.bound_to = slot;
          found.enabled = true;
        }
      })(demopatches.find((item) => item.id === patchId)),
    ),
  unbindPatch: (_slot, patchId) =>
    delay(
      ((found) => {
        if (found) found.bound_to = null;
      })(demopatches.find((item) => item.id === patchId)),
    ),
  configurePatch: (_slot, patchId, enabled, priority) =>
    delay(
      ((found) => {
        if (found) {
          if (enabled !== null) found.enabled = enabled;
          if (priority !== null) found.priority = priority;
        }
        return found;
      })(demopatches.find((item) => item.id === patchId)),
    ),
  updatePatch: (patchId, changes) =>
    delay(
      ((found) => {
        if (!found) throw new Error("找不到这个补丁");
        if (changes.name !== undefined) found.name = changes.name;
        if (changes.author !== undefined) found.author = changes.author;
        if (changes.description !== undefined) found.description = changes.description;
        if (changes.priority !== undefined) found.priority = changes.priority;
        return found;
      })(demopatches.find((item) => item.id === patchId)),
    ),
  exportPatch: (patchId, destination) =>
    delay({
      path: destination,
      files: 3,
      maps: 0,
      mods: 1,
      expanded: [],
      patches: [demopatches.find((item) => item.id === patchId)?.name ?? "补丁"],
    }),
  deletePatch: (patchId) =>
    delay(
      (() => {
        const index = demopatches.findIndex((item) => item.id === patchId);
        if (index >= 0) demopatches.splice(index, 1);
      })(),
    ),
  previewComposition: () =>
    delay({
      files: [
        {
          target: "Maps/Campaign/paiur01.SC2Map",
          layer: { kind: "campaign" as const },
          expanded: false,
        },
        {
          target: "Mods/Extra.SC2Mod",
          layer: {
            kind: "patch" as const,
            id: "幼儿园补丁",
            name: "幼儿园补丁 V3",
            priority: 100,
          },
          expanded: true,
        },
      ],
      overridden: [],
    }),

  variantMaps: (_slot, _variantId) => delay(demoMaps()),
  variantMods: (_slot, _variantId) => delay(demoMods()),
  setMountedMods: (_slot, _variantId, _mods) => delay({} as never),
  setMainMap: (_slot, _variantId, _map) => delay({} as never),
  mainMapChoice: () => delay({ path: "1. Rebel Yell/Terran01.SC2Map", automatic: false, warning: null }),
  openMapInEditor: (_slot, _variantId, map) =>
    delay({
      editor: "D:\\Game\\BLZ\\StarCraft II\\Support64\\SC2Editor_x64.exe",
      map,
      guidance: "编辑器已打开这张地图，按 Ctrl+F9（菜单「测试文档」）就能进入游戏。",
    }),
  variantDoc: () => delay(null),
  listLibraryMods: () => delay(demoLibraryMods()),
  listGameMods: () => delay(demoGameMods()),
  listStandaloneMods: () => delay([]),
  windowMinimize: () => delay(undefined),
  windowToggleMaximize: () => delay(false),
  windowClose: () => delay(undefined),
  windowIsMaximized: () => delay(false),

  ensureTool: () => delay({ id: "sc2diff", name: "SC2Diff", ready: false, action: "skipped", error: null, message: null } as ToolEnsure),
  listTools: () =>
    delay([
      {
        id: "sc2diff",
        name: "SC2Diff",
        about: "对地图和模组做语义 diff 与版本控制，模组「只存差异」靠它",
        repo: "SuperQuail/SC2Diff",
        installed: false,
        version: null,
        path: null,
        size_bytes: 0,
        auto_failed: false,
        auto_error: null,
      },
    ]),
  toolReleases: () =>
    delay([
      {
        version: "0.1.0a2",
        tag: "v0.1.0a2",
        published_at: "2026-10-08T05:21:12Z",
        prerelease: true,
        download_url: "https://example.invalid/sc2diff.exe",
        size: 1_410_000,
        has_asset: true,
      },
    ]),
  installTool: () =>
    delay({
      id: "sc2diff",
      name: "SC2Diff",
      about: "演示",
      repo: "SuperQuail/SC2Diff",
      installed: true,
      version: "0.1.0a2",
      path: "D:\\demo\\sc2diff.exe",
      size_bytes: 1_410_000,
      auto_failed: false,
      auto_error: null,
    } as ToolStatus),
  uninstallTool: () => delay(undefined),
  openToolRepo: () => delay(undefined),
  pickModSource: () => delay(null),
  previewMod: () =>
    delay({ name: "演示模组", folder: "演示模组", mod_count: 1, modid: "demo", fingerprint: "", existing: [], duplicate: false, suggested_version: "1.0" } as ModPreview),
  importMod: () =>
    delay({
      record: {
        id: "demo",
        name: "演示模组",
        modid: "demo",
        fingerprint: "",
        folder: "演示模组",
        kind: "folder",
        author: null,
        version: "1.0",
        description: null,
        enabled: false,
        imported_at: 0,
        size_bytes: 0,
        parts: 1,
      } as StandaloneMod,
      records: [],
      action: "added",
      existing: null,
      message: "已导入「演示模组」",
    } as ModImport),
  editMod: () => delay(
    { id: "demo", name: "演示模组", modid: null, fingerprint: "", folder: "演示模组", kind: "folder", source: { kind: "library" }, author: null, version: null, description: null, enabled: false, imported_at: 0, size_bytes: 0, parts: 1 } as StandaloneMod,
  ),
  updateMod: () => delay({ id: "demo", name: "演示模组", modid: null, fingerprint: "", folder: "演示模组", kind: "folder", author: null, version: null, description: null, enabled: false, imported_at: 0, size_bytes: 0, parts: 1 } as StandaloneMod),
  removeMod: () => delay(undefined),
  exportMod: () => delay("D:\\demo.zip"),
  exportMods: () => delay("3 个文件 -> D:\\demo.zip"),
  compareMods: () =>
    delay({
      before: {} as never,
      after: {} as never,
      identical: false,
      files: [],
      semantic: [],
      semantic_note: "演示模式不做对比",
    } as ModComparison),
  setModEnabled: () => delay({ id: "demo", name: "演示模组", modid: null, fingerprint: "", folder: "演示模组", kind: "folder", author: null, version: null, description: null, enabled: true, imported_at: 0, size_bytes: 0, parts: 1 } as StandaloneMod),
  readDoc: () => delay(new ArrayBuffer(0)),

  appVersion: () => delay("0.1.0a3"),
  networkSettings: () =>
    delay({
      proxy_mode: "auto" as const,
      proxy_url: null,
      use_mirrors: true,
      include_prerelease: true,
    }),
  setNetworkSettings: () => delay(undefined),
  detectedProxy: () => delay({ url: "http://127.0.0.1:7897", source: "Windows 系统代理" }),
  // 演示模式假装有一个新版本，这样整块更新面板（版本卡 / 进度条 / 按钮）都能看到
  checkUpdate: () =>
    delay({
      current: "0.1.0a3",
      latest: {
        version: "0.1.0a4",
        tag: "v0.1.0a4",
        published_at: new Date().toISOString(),
        html_url: "https://github.com/SuperQuail/SC2Miyin-Launcher/releases",
        notes: [
          "## 这一版的新东西",
          "",
          "### 自动更新",
          "",
          "设置页新增「更新」面板，**不用再去 Releases 页面找包**：",
          "",
          "- 检查 → 下载（带进度条）→ 安装并重启",
          "- 下载后校验 SHA-256，对不上就删掉重来",
          "- 内嵌终端会逐行写清刚才做了什么",
          "",
          "### 国内可用性",
          "",
          "| 场景 | 处理 |",
          "| --- | --- |",
          "| 直连慢 | 展开「直连 + 6 个镜像」并发竞速，谁先下完用谁 |",
          "| 代理被限流 | 自动改直连重试（额度按出口 IP 算） |",
          "| 不确定 | 一律不动，宁可让你手动装 |",
          "",
          "### 一行命令验证",
          "",
          "```bash",
          "cargo run -p miyin-core --example check-update",
          "```",
          "",
          "> 更新只替换程序本体，data/ 里的战役与补丁一个字节都不碰。",
          "",
          "详见 [包格式规范](https://github.com/SuperQuail/SC2Miyin-Launcher/blob/main/docs/package-format.md)。",
          "（演示数据，真实运行时这里显示的是 Release 正文。）",
        ].join("\n"),
        prerelease: true,
        assets: [],
      },
      available: true,
      via: "直连（代理被限流）",
      error: null,
    }),
  downloadUpdate: (version) =>
    delay({
      version,
      archive: "C:\\demo\\data\\updates\\" + version + "\\package.zip",
      root: "C:\\demo\\data\\updates\\" + version,
      executable: "C:\\demo\\data\\updates\\" + version + "\\miyin-launcher.exe",
      extras: [],
      url: "https://gh.ddlc.top/https://github.com/...",
      bytes: 6_061_384,
    }),
  applyUpdate: () => delay(undefined),
  openUrl: () => delay(undefined),
  onUpdateProgress: async (handler) => {
    // 演示模式假装在下一次东西，让进度条也动起来
    let done = 0;
    const total = 6_061_384;
    const timer = setInterval(() => {
      done = Math.min(done + 900_000, total);
      handler({
        done,
        total,
        percent: (done / total) * 100,
      });
      if (done >= total) clearInterval(timer);
    }, 260);
    return () => clearInterval(timer);
  },
  onUpdateLog: async (handler) => {
    // 演示模式把一次真实的检查过程回放一遍，好让前端看到终端的实际样子
    const lines = [
      "开始检查更新（当前版本 0.1.0a2）",
      "网络：http://127.0.0.1:7897（Windows 系统代理）",
      "查询 GitHub Releases…",
      "代理被限流：HTTP 403 API rate limit exceeded for 58.152.46.170（剩余额度 0）",
      "改用直连重试…",
      "直连成功",
      "读到 3 个发行版本（含预发行）",
      "发现新版本 0.1.0a3（预发行，2026-10-08）",
      "可下载：SC2Miyin-Launcher-0.1.0a3-win64.zip（5.78 MB）",
    ];
    const timers = lines.map((line, index) =>
      setTimeout(() => handler(line), 260 * (index + 1)),
    );
    return () => timers.forEach(clearTimeout);
  },

  exportVariant: (_slot, _variantId, destination, mergePatches) =>
    delay({
      path: destination,
      files: 12,
      maps: 8,
      mods: 1,
      expanded: [],
      patches: mergePatches ? ["幼儿园补丁 V3"] : [],
    }),
  pickExportPath: (defaultName) => delay("D:\\\\Downloads\\\\" + defaultName + ".zip"),
};

/** 当前可用的后端实现。 */
export const api: LauncherApi = isDesktop ? desktop : demo;
