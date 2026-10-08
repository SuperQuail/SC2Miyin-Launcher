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
  Staged,
  UpdateCheck,
  UpdateProgress,
  Composition,
  ExportReport,
  ImportPreview,
  Installation,
  LauncherApi,
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
  importPackage: (path, slot) => invoke<Variant>("import_package", { path, slot }),
  activateVariant: (slot, variantId) =>
    invoke<string[]>("activate_variant", { slot, variantId }),
  deleteVariant: (slot, variantId) => invoke<void>("delete_variant", { slot, variantId }),
  launchGame: () => invoke<void>("launch_game"),
  revealPath: (path) => invoke<void>("reveal_path", { path }),
  pickPackage: () => invoke<string | null>("pick_package"),
  pickGameDirectory: () => invoke<string | null>("pick_game_directory"),
  variantCover: (slot, variantId) => invoke<string | null>("variant_cover", { slot, variantId }),

  prepareImport: (path) => invoke<ImportPreview>("prepare_import", { path }),
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
];

/** 让演示模式也有「正在处理」的观感。 */
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

  importPackage: (path, slot) =>
    delay(
      (() => {
        counter += 1;
        const created = variant(
          "演示战役包 " + counter,
          "演示战役包",
          "Demo",
          "1.0",
          counter,
          512_000_000,
        );
        const target = demoslots.find((item) => item.slug === slot) ?? demoslots[0];
        target.variants.unshift(created);
        void path;
        return created;
      })(),
    ),

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

  prepareImport: (path) =>
    delay({
      path,
      inspection: demoInspection,
      // 演示里故意标成"自动识别"，好把尽力而为的措辞也演示出来
      source: "inferred" as const,
      slot: "hots",
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

  appVersion: () => delay("0.2.0-alpha.1"),
  networkSettings: () =>
    delay({
      proxy_mode: "auto" as const,
      proxy_url: null,
      use_mirrors: true,
      include_prerelease: true,
    }),
  setNetworkSettings: () => delay(undefined),
  detectedProxy: () => delay({ url: "http://127.0.0.1:7897", source: "Windows 系统代理" }),
  checkUpdate: () =>
    delay({
      current: "0.2.0-alpha.1",
      latest: null,
      available: false,
      via: "http://127.0.0.1:7897（Windows 系统代理）",
      error: null,
    }),
  downloadUpdate: () => delay({} as never),
  applyUpdate: () => delay(undefined),
  openUrl: () => delay(undefined),
  onUpdateProgress: async () => () => {},

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
