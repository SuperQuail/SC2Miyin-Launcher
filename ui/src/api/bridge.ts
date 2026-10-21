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
  GameLaunch,
  UpdateResult,
  GameModEntry,
  LibraryMod,
  MergeAddition,
  MergeChoice,
  MergeConflict,
  MergeStatus,
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
  InstalledCampaign,
  LauncherApi,
  Preview,
  SaveBackup,
  SaveFile,
  SaveIsolation,
  SaveTarget,
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
  mergeStatus: () => invoke<MergeStatus | null>("merge_status"),
  mergeApply: (choices) => invoke<void>("merge_apply", { choices }),
  inspectPackage: (path) => invoke<PackageInspection>("inspect_package", { path }),
  saveIsolation: () => invoke<SaveIsolation>("save_isolation"),
  setSaveIsolation: (enabled) => invoke<SaveIsolation>("set_save_isolation", { enabled }),
  saveCurrentSaves: () => invoke<SaveIsolation>("save_current_saves"),
  switchSaveProfile: (name) => invoke<SaveIsolation>("switch_save_profile", { name }),
  assignSaveProfile: (name, slot) => invoke<SaveIsolation>("assign_save_profile", { name, slot }),
  listSaves: () => invoke<SaveSet>("list_saves"),
  backupSaves: (label) => invoke<string>("backup_saves", { label }),
  listSaveBackups: () => invoke<SaveBackup[]>("list_save_backups"),
  restoreSaves: (name) => invoke<string>("restore_saves", { name }),
  saveIntoProfile: (name) => invoke<SaveIsolation>("save_into_profile", { name }),
  savesTargets: () => invoke<SaveTarget[]>("saves_targets"),
  saveGroupFiles: (name) => invoke<SaveFile[]>("save_group_files", { name }),
  moveSaveFile: (from, file, to) => invoke<void>("move_save_file", { from, file, to }),
  moveSaveToKey: (from, file, key) => invoke<string>("move_save_to_key", { from, file, key }),

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

  listInstalledCampaigns: () => invoke<InstalledCampaign[]>("list_installed_campaigns"),
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
  launchGameWithMap: (slot, variantId, map) =>
    invoke<GameLaunch>("launch_game_with_map", { slot, variantId, map }),
  resolveSiteUrl: (https, http) => invoke<string>("resolve_site_url", { https, http }),
  updateVariantFromPackage: (slot, variantId, path) =>
    invoke<UpdateResult>("update_variant_from_package", { slot, variantId, path }),
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

function delay<T>(value: T, ms = 220): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

let counter = 0;

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
  enabled: true,
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
 * 演示模式的隔离键 —— 和后端同规则：**有版本就是 `槽位/身份`**（官方改版也一样），
 * 原版才是槽位本身；身份优先注册 ID、没有就用版本 id。
 */
function demoIsolationKey(slot: string, variantId: string): string {
  const item = demoslots
    .find((entry) => entry.slug === slot)
    ?.variants.find((entry) => entry.id === variantId);
  return slot + "/" + (item?.registration_id ?? variantId);
}

/** 演示模式里模拟 `ensure_for_slot`：官方战役继承当前存档，自制战役每部一个空组。 */
function demoEnsureSaves(key: string, label: string): void {
  if (!demoSavesState.enabled) return;
  const assigned = Object.entries(demoSavesState.assignments).find(([, value]) => value === key)?.[0];
  if (assigned) {
    demoSavesState.active = assigned;
    const group = demoSaveGroups.find((item) => item.name === assigned);
    if (group) demoBanks = demoBanksOf(group);
    return;
  }
  // 只有「还没归过档」才收编当前这份；已经有组在用就从空组开始（别串档）
  const custom = key.startsWith("custom/") || key === "custom";
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

/**
 * 演示模式：假装启用「虫群之心：进化重制」会和自由之翼当前启用的那一版抢位置 ——
 * 好让"启用冲突"对话框在浏览器里也能真跑（桌面壳里真冲突才出现）。
 */
function demoActivationConflicts(slot: string, variantId: string): ActivationConflict[] {
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

/** 演示模式的归属候选：和后端 `targets()` 同规则（官方：原版 + 每个改版；自制每部一条）。 */
function demoTargets(): SaveTarget[] {
  const out: SaveTarget[] = [];
  // 键相同的只留一条（同一部战役的多个版本共用一个组）；重名补版本号/身份，
  // 否则下拉里会出现两条一模一样的选项 —— 和 core 里 named_targets 的规则一致
  const push = (key: string, label: string, version?: string | null) => {
    if (out.some((item) => item.key === key)) return;
    const clash = out.some((item) => item.label === label);
    const suffix = clash
      ? version && version.trim()
        ? " " + version
        : "（" + key.split("/").pop() + "）"
      : "";
    out.push({ key, label: label + suffix });
  };
  // 官方：每个槽位先「（原版）」（键 = 槽位），再跟上它的每个改版（同一部只一条）
  for (const slot of demoslots.filter((item) => item.slug !== "custom")) {
    push(slot.slug, slot.display_name + "（原版）");
    for (const item of slot.variants) {
      push(demoIsolationKey(slot.slug, item.id), slot.display_name + " · " + item.name, item.version);
    }
  }
  // 自制：每部一条
  const custom = demoslots.find((slot) => slot.slug === "custom");
  for (const item of custom?.variants ?? []) {
    push(demoIsolationKey("custom", item.id), "自制战役 · " + item.name, item.version);
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

const demo: LauncherApi = {
  detectInstallation: () => delay(demoInstallation as Installation | null),
  setInstallation: () => delay(demoInstallation),
  libraryRoot: () => delay(DEMO_LIBRARY_ROOT),
  listSlots: () => delay(demoslots.map((slot) => ({ ...slot }))),
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
  listSaveBackups: async () => demoSaveGroups.map((item) => ({ ...item })),
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
          }
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

  listInstalledCampaigns: async () => [],
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
  resolveSiteUrl: async (https, _http) => https,
  updateVariantFromPackage: (slot, variantId, path) =>
    delay(
      (() => {
        counter += 1;
        const created = variant(
          "更新后的战役包 v" + counter,
          "更新后的战役包",
          "未知作者",
          "2." + counter,
          counter,
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

/** 启用前查到的冲突：这个战役要写、但已经被**另一个已启用**的战役占着。 */
export interface ActivationConflict {
  /** 相对游戏根目录，`/` 分隔。 */
  target: string;
  /** 占着它的那个战役槽位。 */
  slot: string;
  /** 槽位显示名（后端已经映射好了）。 */
  slot_name: string;
  /** 那个战役当前启用的版本 id。 */
  variant: string;
  variant_name: string;
}

/**
 * 启用前查一次：这次会不会覆盖**别的已启用战役**的文件。
 *
 * **只读** —— 后端那条命令一个字节都不写，界面必须在写盘之前调它。
 * 单独导出、没塞进 `LauncherApi`：共享类型文件（api/types.ts）不在本轮范围里，
 * 为一个只读检查多开一张接口表不值得。
 */
export async function activationConflicts(
  slot: string,
  variantId: string,
): Promise<ActivationConflict[]> {
  if (isDesktop) {
    return invoke<ActivationConflict[]>("activation_conflicts", { slot, variantId });
  }
  return delay(demoActivationConflicts(slot, variantId));
}
