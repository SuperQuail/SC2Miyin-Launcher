/**
 * 与 Rust 后端通信的桥接层。
 *
 * 如果没有 Tauri 运行时（例如直接用浏览器打开 `vite dev`），会自动退回**演示数据**，
 * 这样界面可以脱离桌面壳独立开发与截图验证 —— 这也是 `pnpm dev` 的默认体验。
 */
import { invoke } from "@tauri-apps/api/core";

import type { Campaign, Installation, LauncherApi, PackageInspection } from "./types";

/** 是否运行在 Tauri 桌面壳里。 */
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** 真实后端。 */
const desktop: LauncherApi = {
  detectInstallation: () => invoke<Installation | null>("detect_installation"),
  setInstallation: (path) => invoke<Installation>("set_installation", { path }),
  listCampaigns: () => invoke<Campaign[]>("list_campaigns"),
  inspectPackage: (path) => invoke<PackageInspection>("inspect_package", { path }),
  installPackage: (path) => invoke<Campaign>("install_package", { path }),
  uninstallCampaign: (id) => invoke<void>("uninstall_campaign", { id }),
  launchGame: () => invoke<void>("launch_game"),
  revealPath: (path) => invoke<void>("reveal_path", { path }),
  pickPackage: () => invoke<string | null>("pick_package"),
  pickGameDirectory: () => invoke<string | null>("pick_game_directory"),
  campaignCover: (dir) => invoke<string | null>("campaign_cover", { dir }),
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

const demoCampaigns: Campaign[] = [
  {
    id: "Wings of Liberty Reborn",
    name: "自由之翼：重生",
    author: "SomeCreator",
    version: "1.4.2",
    description: "重制版自由之翼战役，含 32 张关卡与全语音过场。",
    cover: null,
    path: "D:\\Game\\BLZ\\StarCraft II\\Maps\\CustomCampaigns\\Wings of Liberty Reborn",
    format: "ccm",
    campaign_type: "wol",
    enabled: true,
    health: "ok",
    issues: [],
    map_count: 32,
    size_bytes: 1_284_000_000,
  },
  {
    id: "Heart of the Swarm Evolved",
    name: "虫群之心：进化重制",
    author: "SwarmTeam",
    version: "0.9.1",
    description: "在进化任务线基础上追加了 6 张自制关卡。",
    cover: null,
    path: "D:\\Game\\BLZ\\StarCraft II\\Maps\\CustomCampaigns\\Heart of the Swarm Evolved",
    format: "ccm",
    campaign_type: "hotsevolution",
    enabled: false,
    health: "warning",
    issues: [
      {
        level: "warning",
        code: "MISSING_CAMPAIGN_DIR",
        message: "官方战役目录 Maps/Campaign/swarm/evolution 不存在",
        hint: "启用时会自动创建该目录，无需手工处理",
      },
    ],
    map_count: 14,
    size_bytes: 760_000_000,
  },
  {
    id: "Nova Lost Memories",
    name: "诺娃隐秘行动：失落的记忆",
    author: "GhostOps",
    version: "2.0",
    description: "诺娃任务线的同人续作，共 9 关。",
    cover: null,
    path: "D:\\Game\\BLZ\\StarCraft II\\Maps\\CustomCampaigns\\Nova Lost Memories",
    format: "miyin",
    campaign_type: "nova",
    enabled: false,
    health: "ok",
    issues: [],
    map_count: 9,
    size_bytes: 412_000_000,
  },
  {
    id: "Legacy Broken Pack",
    name: "虚空之遗：残卷",
    author: null,
    version: null,
    description: null,
    cover: null,
    path: "D:\\Game\\BLZ\\StarCraft II\\Maps\\CustomCampaigns\\Legacy Broken Pack",
    format: "plain",
    campaign_type: "lotv",
    enabled: false,
    health: "broken",
    issues: [
      {
        level: "broken",
        code: "NO_CONTENT",
        message: "目录内没有 .SC2Map 地图或 .SC2Mod 模组文件",
        hint: "可能是安装中断，建议删除后重新安装",
      },
    ],
    map_count: 0,
    size_bytes: 2048,
  },
];

/** 让演示模式也有"正在处理"的观感。 */
function delay<T>(value: T, ms = 260): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

const demo: LauncherApi = {
  detectInstallation: () => delay(demoInstallation as Installation | null),
  setInstallation: () => delay(demoInstallation),
  listCampaigns: () => delay(demoCampaigns),
  inspectPackage: (path) =>
    delay({
      path,
      installable: true,
      format: "ccm",
      name: "演示战役包",
      author: "Demo",
      version: "1.0",
      description: "演示模式下的预检结果。",
      campaign_type: "wol",
      content_root: "Demo",
      suggested_dir_name: "演示战役包",
      map_count: 12,
      mod_count: 2,
      entry_count: 148,
      unpacked_bytes: 512_000_000,
      issues: [],
    } satisfies PackageInspection),
  installPackage: (path) =>
    delay({
      ...demoCampaigns[0],
      id: "Demo Installed",
      name: "演示战役包",
      path,
    } satisfies Campaign),
  uninstallCampaign: () => delay(undefined),
  launchGame: () => delay(undefined),
  revealPath: () => delay(undefined),
  pickPackage: () => delay("D:\\Downloads\\demo-campaign.zip"),
  pickGameDirectory: () => delay("D:\\Game\\BLZ\\StarCraft II"),
  campaignCover: () => delay(null),
};

/** 当前可用的后端实现。 */
export const api: LauncherApi = isDesktop ? desktop : demo;
