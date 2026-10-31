// 桥接契约：真实桌面桥（../bridge.ts）与演示后端（../demo/）都实现这一张表。
//
// 参数名是 camelCase，Rust 侧命令是 snake_case，由调用处的对象显式映射。
import type {
  BoundPatch,
  Composition,
  DocInfo,
  EditorLaunch,
  ExportReport,
  GameLaunch,
  ImportMode,
  ImportPreview,
  InstalledCampaign,
  MainMapChoice,
  MapEntry,
  MergeChoice,
  MergeStatus,
  PackageInspection,
  Patch,
  Preview,
  SlotView,
  UpdateResult,
  Variant,
} from "./campaign";
import type { Installation, ToolEnsure, ToolRelease, ToolStatus } from "./install";
import type {
  GameModEntry,
  LibraryMod,
  ModChanges,
  ModComparison,
  ModEntry,
  ModIdentity,
  ModImport,
  ModImportMode,
  ModPreview,
  StandaloneMod,
} from "./mod";
import type {
  SaveBackup,
  SaveFile,
  SaveInventory,
  SaveIsolation,
  SaveSet,
  SaveTarget,
} from "./saves";
import type {
  DetectedProxy,
  NetworkSettings,
  Staged,
  UpdateCheck,
  UpdateProgress,
} from "./update";

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

export interface LauncherApi {
  detectInstallation(): Promise<Installation | null>;
  /** 拉起（或复用）管理员助手；返回是否已就绪（用户取消 UAC 时是 false）。 */
  requestPrivilegedWorker(): Promise<boolean>;
  /**
   * 把一个写操作转给管理员助手执行（按需提权，docs/design/storage-and-elevation.md §3）。
   *
   * `op` 与参数与同名命令一致；返回值不保证与普通命令同形，
   * 界面做完这一步应当重新读一遍状态。
   */
  privilegedOp(op: string, args: unknown): Promise<unknown>;
  /** 启动时查一次有没有另一份数据要并；没有返回 null。 */
  mergeStatus(): Promise<MergeStatus | null>;
  /** 按 `<slot>/<variant>` 的选择应用合并；空对象 = 冲突全保留游戏目录那份。 */
  mergeApply(choices: Record<string, MergeChoice>): Promise<void>;
  setInstallation(path: string): Promise<Installation>;
  /** 战役库根目录（软件同级的 data 目录）。 */
  libraryRoot(): Promise<string>;
  /** 全部槽位，已按官方发布顺序排列。 */
  listSlots(): Promise<SlotView[]>;
  inspectPackage(path: string): Promise<PackageInspection>;
  /** 启用某个版本；variantId 传 null 表示切回原版战役。 */
  /** 存档隔离状态。 */
  saveIsolation(): Promise<SaveIsolation>;

  /** 打开 / 关掉隔离。**打开时会把现在这份 Banks 收成「原版」。** */
  setSaveIsolation(enabled: boolean): Promise<SaveIsolation>;

  /** 把现在这份存档存回它所属的组。 */
  saveCurrentSaves(): Promise<SaveIsolation>;

  /** 切到某个存档组（先存回现在这份）。 */
  switchSaveProfile(name: string): Promise<SaveIsolation>;

  /** 手动改归属：指给某个战役，或 null 表示算原版。 */
  assignSaveProfile(name: string, slot: string | null): Promise<SaveIsolation>;

  /** 现在的存档里有什么（我的文档 / StarCraft II / Banks）。 */
  listSaves(): Promise<SaveSet>;

  /** 把现在的存档备份一份。label 是备注（一般填战役名）。 */
  backupSaves(label: string): Promise<string>;

  /** 已经备份了哪些。 */
  listSaveBackups(): Promise<SaveBackup[]>;

  /** 还原一份备份；还原前会先把现在的存档另存一份，返回那份的名字。 */
  restoreSaves(name: string): Promise<string>;
  /** 把游戏里这份存档存进**指定的组**（存档页的「设为当前」），并把它记为当前组。 */
  saveIntoProfile(name: string): Promise<SaveIsolation>;
  /** 归属下拉的候选（键的算法在后端，界面别自己拼）。 */
  savesTargets(): Promise<SaveTarget[]>;
  /** 扫一遍**真实**存档目录（`Accounts/<账号>/<地域>/Saves` 与 `Banks`）。只读。 */
  scanSaves(): Promise<SaveInventory>;
  /** 一个存档组里有哪些存档文件（组详情用）。 */
  saveGroupFiles(name: string): Promise<SaveFile[]>;
  /** 把某一份存档挪到另一个组（= 改它归到哪个战役）。同名不覆盖。 */
  moveSaveFile(from: string, file: string, to: string): Promise<void>;
  /**
   * 把某一份存档挪到**某个战役 / 版本名下**（键来自 [savesTargets]）。
   *
   * 组是自动生成的：有现成组就挪进去，没有就新建并自动绑定。返回挪进去的组名。
   */
  moveSaveToKey(from: string, file: string, key: string): Promise<string>;

  /** 游戏是不是正跑着；跑着就返回进程名。切换前先问它。 */

  /** 启用前先看：会往游戏目录里放什么、覆盖什么、删什么（不写盘）。 */
  previewActivation(slot: string, variantId: string): Promise<Preview | null>;

  activateVariant(slot: string, variantId: string | null): Promise<string[]>;
  /** 停用全部战役：撤掉启动器写进游戏目录的全部战役文件（索引与存档组回原版）。 */
  deactivateAll(): Promise<string[]>;
  deleteVariant(slot: string, variantId: string): Promise<void>;
  launchGame(): Promise<void>;
  revealPath(path: string): Promise<void>;
  pickPackage(): Promise<string | null>;
  /** 选一个**文件夹**当战役包（作者直接发的解开的包）。 */
  pickPackageFolder(): Promise<string | null>;
  pickGameDirectory(): Promise<string | null>;
  /** 读取某个版本自带的封面图（data URL）；没有则返回 null。 */
  variantCover(slot: string, variantId: string): Promise<string | null>;

  /** 选完文件后的第一步：预检、判断归属、查冲突。不写任何文件。 */
  /**
   * 预检一个包。
   *
   * `entry` 是**用户从哪个入口点的导入**：`custom` 表示站在「自制战役」页 ——
   * 那就不再判断它属于哪部原版战役，直接按自制战役来。
   */
  /** 游戏目录里已经装着的自制战役（收编进库用）。 */
  listInstalledCampaigns(): Promise<InstalledCampaign[]>;

  prepareImport(
    path: string,
    entry?: "campaign" | "custom",
    password?: string | null,
  ): Promise<ImportPreview>;
  /** 按指定战役导入；传了 slot 就按传的来，覆盖自动判定。 */
  importPackageWith(
    path: string,
    slot: string | null,
    mode: ImportMode,
    password?: string | null,
  ): Promise<Variant>;
  /** 改一个已导入版本的元数据。 */
  updateVariant(
    slot: string,
    variantId: string,
    changes: {
      name?: string;
      author?: string;
      registrationId?: string;
      description?: string;
      /** 包内声明为依赖的模组键；不传表示这一项不动。 */
      declaredMods?: string[];
    },
  ): Promise<Variant>;

  listPatches(): Promise<Patch[]>;
  listBindings(slot: string): Promise<BoundPatch[]>;
  listAvailablePatches(slot: string): Promise<BoundPatch[]>;
  importPatch(path: string, password?: string | null): Promise<Patch>;
  autoBindPatches(): Promise<string[]>;
  bindPatch(slot: string, patchId: string): Promise<void>;
  unbindPatch(slot: string, patchId: string): Promise<void>;
  configurePatch(
    slot: string,
    patchId: string,
    enabled: boolean | null,
    priority: number | null,
  ): Promise<unknown>;
  deletePatch(patchId: string): Promise<void>;
  /** 改一个已导入补丁的元数据。 */
  updatePatch(
    patchId: string,
    changes: {
      name?: string;
      author?: string;
      registrationId?: string;
      description?: string;
      priority?: number;
    },
  ): Promise<Patch>;
  /** 单独导出一个补丁包。 */
  exportPatch(patchId: string, destination: string): Promise<ExportReport>;
  previewComposition(slot: string, variantId: string): Promise<Composition>;

  exportVariant(
    slot: string,
    variantId: string,
    destination: string,
    mergePatches: boolean,
  ): Promise<ExportReport>;
  pickExportPath(defaultName: string): Promise<string | null>;

  /** 当前程序版本。 */
  appVersion(): Promise<string>;
  networkSettings(): Promise<NetworkSettings>;
  setNetworkSettings(settings: NetworkSettings): Promise<void>;
  /** 当前自动探测到的代理；直连时为 null。 */
  detectedProxy(): Promise<DetectedProxy | null>;
  checkUpdate(): Promise<UpdateCheck>;
  downloadUpdate(version: string): Promise<Staged>;
  /** 换上新版本并退出程序。 */
  applyUpdate(): Promise<void>;
  openUrl(url: string): Promise<void>;
  /** 订阅下载进度，返回取消订阅的函数。 */
  onUpdateProgress(handler: (progress: UpdateProgress) => void): Promise<() => void>;
  /** 列出版本里的地图。 */
  variantMaps(slot: string, variantId: string): Promise<MapEntry[]>;
  /** 列出版本里的模组与挂载状态。 */
  variantMods(slot: string, variantId: string): Promise<ModEntry[]>;
  /** 改挂载清单。 */
  setMountedMods(slot: string, variantId: string, mods: string[]): Promise<Variant>;
  /** 改主地图；传 null 清空。 */
  setMainMap(slot: string, variantId: string, map: string | null): Promise<Variant>;
  /** 这个版本该用哪张地图作为入口。 */
  mainMapChoice(slot: string, variantId: string): Promise<MainMapChoice>;
  /** 铺模组并用编辑器打开某张地图。 */
  openMapInEditor(slot: string, variantId: string, map: string): Promise<EditorLaunch>;
  /** 探一下站点支不支持 HTTPS：支持返回 https 那份，不支持返回 http 那份。 */
  resolveSiteUrl(https: string, http: string): Promise<string>;
  /** 用压缩包更新某个战役（删旧的装新的；该跟着走的记录会带过去）。 */
  updateVariantFromPackage(
    slot: string,
    variantId: string | null,
    path: string,
    password?: string | null,
  ): Promise<UpdateResult>;
  /** 直接启动星际争霸并进这张地图（不经过编辑器）。 */
  launchGameWithMap(slot: string, variantId: string, map: string): Promise<GameLaunch>;
  /** 版本自带的说明文档；没有就是 null。 */
  variantDoc(slot: string, variantId: string): Promise<DocInfo | null>;
  /** 读出说明文档的字节，交给 PDF 渲染器。 */
  readDoc(slot: string, variantId: string): Promise<ArrayBuffer>;

  /** 可选工具的清单与安装状态。 */
  listTools(): Promise<ToolStatus[]>;
  /** 最小化窗口。 */
  windowMinimize(): Promise<void>;
  /** 最大化 / 还原；返回操作后是不是最大化。 */
  windowToggleMaximize(): Promise<boolean>;
  /** 关窗口。 */
  windowClose(): Promise<void>;
  /** 现在是不是最大化。 */
  windowIsMaximized(): Promise<boolean>;

  /** 启动时调：没装就静默装上；之前失败过就不再试。 */
  ensureTool(id: string): Promise<ToolEnsure>;
  /** 某个工具有哪些版本可装。 */
  toolReleases(id: string): Promise<ToolRelease[]>;
  /** 装一个工具；version 传 null 表示装最新的。 */
  installTool(id: string, version: string | null): Promise<ToolStatus>;
  /** 卸载一个工具。 */
  uninstallTool(id: string): Promise<void>;
  /** 在浏览器里打开工具的仓库。 */
  openToolRepo(id: string): Promise<void>;
  /** 打开内置的 7-Zip 解压工具（嵌在启动器窗口里）。 */
  openSevenZip(): Promise<void>;
  /** 关闭 7-Zip 覆盖层（顶栏「退出 7-Zip」按钮）。 */
  closeSevenZip(): Promise<void>;

  /** 全库模组汇总。 */
  listLibraryMods(): Promise<LibraryMod[]>;
  /** 游戏目录 Mods/ 里实际放着的模组。 */
  listGameMods(): Promise<GameModEntry[]>;
  /** 独立模组库。 */
  listStandaloneMods(): Promise<StandaloneMod[]>;
  /** 选一个模组包：file（压缩包 / .SC2Mod）或 folder。 */
  pickModSource(kind: "file" | "folder"): Promise<string | null>;
  /** 导入前的预检：这个包是谁、库里有没有同族的。 */
  previewMod(path: string): Promise<ModPreview>;
  /** 导入模组包。 */
  importMod(path: string, mode?: ModImportMode): Promise<ModImport>;
  /** 改模组信息。 */
  updateMod(id: string, changes: ModChanges): Promise<StandaloneMod>;
  /**
   * 改一个模组的元数据。
   *
   * 独立模组和战役包带来的模组走同一条路 —— 改的都是记录，不动包里的原始文件。
   * 战役模组第一次编辑时会先建一条记录。
   */
  editMod(
    recordId: string | null,
    identity: ModIdentity | null,
    changes: ModChanges,
  ): Promise<StandaloneMod>;

  /** 删掉模组。 */
  removeMod(id: string): Promise<void>;
  /** 比两个模组版本差在哪。 */
  compareMods(before: string, after: string): Promise<ModComparison>;
  /** 把选中的若干模组版本打包成一个 zip，返回结果说明。 */
  exportMods(ids: string[]): Promise<string>;
  /** 导出模组包，返回写到哪了。 */
  exportMod(id: string): Promise<string>;
  /** 启用 / 停用独立模组（启用会立刻铺进游戏目录）。 */
  setModEnabled(id: string, enabled: boolean): Promise<StandaloneMod>;

  /** 订阅更新过程的日志（界面渲染成内嵌终端），返回取消订阅的函数。 */
  onUpdateLog(handler: (line: string) => void): Promise<() => void>;

  /**
   * 取走启动时自动挂载模组攒下的提示（取一次就清空）。
   *
   * 空数组表示一切正常；非空说明上次模组没挂全，界面该告诉用户，别让他以为已经生效。
   */
  takeMountWarnings(): Promise<string[]>;
}
