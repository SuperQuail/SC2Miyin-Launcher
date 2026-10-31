/**
 * 与 Rust 后端通信的桥接层。
 *
 * 真实桌面桥只做一件事：把命令名与参数原样交给 Tauri 的 `invoke`。
 * 没有 Tauri 运行时（例如直接用浏览器打开 `vite dev`）时退回 `./demo` 的演示后端，
 * 界面可以脱离桌面壳独立开发与截图验证；演示数据在 `./demo/` 里按域拆分。
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { delay, demo, demoActivationConflicts } from "./demo";
import type {
  BoundPatch,
  Composition,
  DetectedProxy,
  DocInfo,
  EditorLaunch,
  ExportReport,
  GameLaunch,
  GameModEntry,
  ImportPreview,
  Installation,
  InstalledCampaign,
  LauncherApi,
  LibraryMod,
  MainMapChoice,
  MapEntry,
  MergeStatus,
  ModComparison,
  ModEntry,
  ModImport,
  ModPreview,
  NetworkSettings,
  PackageInspection,
  Patch,
  Preview,
  SaveBackup,
  SaveFile,
  SaveInventory,
  SaveIsolation,
  SaveSet,
  SaveTarget,
  SlotView,
  StandaloneMod,
  Staged,
  ToolEnsure,
  ToolRelease,
  ToolStatus,
  UpdateCheck,
  UpdateProgress,
  UpdateResult,
  Variant,
  ActivationConflict,
} from "./types";

export type { ActivationConflict } from "./types";

/** 是否运行在 Tauri 桌面壳里。 */
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** 真实后端。 */
const desktop: LauncherApi = {
  detectInstallation: () => invoke<Installation | null>("detect_installation"),
  requestPrivilegedWorker: () => invoke<boolean>("request_privileged_worker"),
  privilegedOp: (op, args) => invoke<unknown>("privileged_op", { op, args }),
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
  scanSaves: () => invoke<SaveInventory>("scan_saves"),
  savesTargets: () => invoke<SaveTarget[]>("saves_targets"),
  saveGroupFiles: (name) => invoke<SaveFile[]>("save_group_files", { name }),
  moveSaveFile: (from, file, to) => invoke<void>("move_save_file", { from, file, to }),
  moveSaveToKey: (from, file, key) => invoke<string>("move_save_to_key", { from, file, key }),

  /** 启用前先看：会往游戏目录里放什么、覆盖什么、删什么（不写盘）。 */
  previewActivation: (slot, variantId) =>
    invoke<Preview>("preview_activation", { slot, variantId }),
  activateVariant: (slot, variantId) =>
    invoke<string[]>("activate_variant", { slot, variantId }),
  deactivateAll: () => invoke<string[]>("deactivate_all_campaigns"),
  deleteVariant: (slot, variantId) => invoke<void>("delete_variant", { slot, variantId }),
  launchGame: () => invoke<void>("launch_game"),
  revealPath: (path) => invoke<void>("reveal_path", { path }),
  pickPackage: () => invoke<string | null>("pick_package"),
  pickPackageFolder: () => invoke<string | null>("pick_package_folder"),
  pickGameDirectory: () => invoke<string | null>("pick_game_directory"),
  variantCover: (slot, variantId) => invoke<string | null>("variant_cover", { slot, variantId }),

  listInstalledCampaigns: () => invoke<InstalledCampaign[]>("list_installed_campaigns"),
  prepareImport: (path, entry, password) =>
    invoke<ImportPreview>("prepare_import", { path, entry: entry ?? null, password: password ?? null }),
  importPackageWith: (path, slot, mode, password) =>
    invoke<Variant>("import_package", { path, slot, mode, password: password ?? null }),
  updateVariant: (slot, variantId, changes) =>
    invoke<Variant>("update_variant", { slot, variantId, changes }),

  listPatches: () => invoke<Patch[]>("list_patches"),
  listBindings: (slot) => invoke<BoundPatch[]>("list_bindings", { slot }),
  listAvailablePatches: (slot) => invoke<BoundPatch[]>("list_available_patches", { slot }),
  importPatch: (path, password) =>
    invoke<Patch>("import_patch", { path, password: password ?? null }),
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
  updateVariantFromPackage: (slot, variantId, path, password) =>
    invoke<UpdateResult>("update_variant_from_package", {
      slot,
      variantId,
      path,
      password: password ?? null,
    }),
  variantDoc: (slot, variantId) => invoke<DocInfo | null>("variant_doc", { slot, variantId }),
  listLibraryMods: () => invoke<LibraryMod[]>("list_library_mods"),
  listGameMods: () => invoke<GameModEntry[]>("list_game_mods"),
  listStandaloneMods: () => invoke<StandaloneMod[]>("list_standalone_mods"),
  windowMinimize: () => invoke<void>("window_minimize"),
  windowToggleMaximize: () => invoke<boolean>("window_toggle_maximize"),
  windowClose: () => invoke<void>("window_close"),
  windowIsMaximized: () => invoke<boolean>("window_is_maximized"),

  listTools: () => invoke<ToolStatus[]>("list_tools"),
  openSevenZip: () => invoke<void>("open_sevenzip"),
  closeSevenZip: () => invoke<void>("close_sevenzip"),
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
  takeMountWarnings: () => invoke<string[]>("take_mount_warnings"),
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

/** 当前可用的后端实现。 */
export const api: LauncherApi = isDesktop ? desktop : demo;

/**
 * 启用前查一次：这次会不会覆盖**别的已启用战役**的文件。
 *
 * **只读** —— 后端那条命令一个字节都不写，界面必须在写盘之前调它。
 * 单独导出、没塞进 `LauncherApi`：它只在启用流程里用一次，不值得为它多开一张接口表。
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
