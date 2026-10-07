/**
 * 与 Rust 后端通信的桥接层。
 *
 * 如果没有 Tauri 运行时（例如直接用浏览器打开 `vite dev`），会自动退回**演示数据**，
 * 这样界面可以脱离桌面壳独立开发与截图验证。演示数据是可交互的：
 * 导入、切换、删除都会即时反映在界面上。
 */
import { invoke } from "@tauri-apps/api/core";

import type {
  Installation,
  LauncherApi,
  PackageInspection,
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
};

/** 当前可用的后端实现。 */
export const api: LauncherApi = isDesktop ? desktop : demo;
