import { invoke } from "@tauri-apps/api/core";

/** 和 Rust 那边 `miyin_core::dev` 一一对应。 */
export type DevEntry = {
  path: string;
  abs: string;
  name: string;
  is_dir: boolean;
  bytes: number;
  external: boolean;
};

export type DevScan = {
  entries: DevEntry[];
  external_roots: string[];
};

export type FilePreview =
  | { kind: "text"; text: string; lines: number; truncated: boolean; bytes: number }
  | { kind: "binary"; bytes: number }
  | { kind: "image"; bytes: number };

/** 在不在 Tauri 里 —— 浏览器打开时没有 IPC。 */
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * 浏览器演示数据。
 *
 * 只有 `pnpm -C ui dev` 直接在浏览器里打开 ide.html 时才会用到 —— 那里没有 IPC，
 * 不给一份数据整页就是空的，看不出界面长什么样。**桌面版永远走真扫描。**
 */
const SAMPLE: DevScan = {
  external_roots: [],
  entries: [
    { path: "Maps", abs: "", name: "Maps", is_dir: true, bytes: 0, external: false },
    { path: "Maps/CustomCampaigns", abs: "", name: "CustomCampaigns", is_dir: true, bytes: 0, external: false },
    // 空壳目录：只有目录、里面一个文件都没有 —— 示例数据里留一个，界面才有机会练到这种情况
    { path: "Maps/Campaign", abs: "", name: "Campaign", is_dir: true, bytes: 0, external: false },
    { path: "Maps/Campaign/void", abs: "", name: "void", is_dir: true, bytes: 0, external: false },
    { path: "Maps/CustomCampaigns/示例战役", abs: "", name: "示例战役", is_dir: true, bytes: 0, external: false },
    { path: "Maps/CustomCampaigns/示例战役/01.SC2Map", abs: "C:\\示例\\01.SC2Map", name: "01.SC2Map", is_dir: false, bytes: 1284096, external: false },
    { path: "Maps/CustomCampaigns/示例战役/02.SC2Map", abs: "C:\\示例\\02.SC2Map", name: "02.SC2Map", is_dir: false, bytes: 1171456, external: false },
    { path: "Maps/CustomCampaigns/示例战役/说明.txt", abs: "C:\\示例\\说明.txt", name: "说明.txt", is_dir: false, bytes: 4096, external: false },
    { path: "Maps/CustomCampaigns/示例战役/封面.png", abs: "C:\\示例\\封面.png", name: "封面.png", is_dir: false, bytes: 317440, external: false },
    { path: "Mods", abs: "", name: "Mods", is_dir: true, bytes: 0, external: false },
    { path: "Mods/SCMRmod.SC2Mod", abs: "C:\\示例\\SCMRmod.SC2Mod", name: "SCMRmod.SC2Mod", is_dir: false, bytes: 984064, external: false },
    { path: "Mods/界面包.SC2Mod", abs: "", name: "界面包.SC2Mod", is_dir: true, bytes: 0, external: false },
    { path: "Mods/界面包.SC2Mod/Base.SC2Data", abs: "", name: "Base.SC2Data", is_dir: true, bytes: 0, external: false },
    { path: "Mods/界面包.SC2Mod/Base.SC2Data/ComponentList.SC2Components", abs: "C:\\示例\\ComponentList.SC2Components", name: "ComponentList.SC2Components", is_dir: false, bytes: 462, external: false },
    { path: "E:\\临时\\新单位包", abs: "", name: "新单位包", is_dir: true, bytes: 0, external: true },
    { path: "E:\\临时\\新单位包/UnitData.xml", abs: "E:\\临时\\新单位包\\UnitData.xml", name: "UnitData.xml", is_dir: false, bytes: 2048, external: true },
  ],
};

/** 扫游戏目录。不在桌面壳里时给一份演示数据。 */
export async function scan(extra: string[]): Promise<DevScan | null> {
  if (!isDesktop) return SAMPLE;
  try {
    return await invoke<DevScan>("dev_scan", { extra });
  } catch {
    return null;
  }
}

/** 只读预览一个文件。 */
export async function readFile(path: string): Promise<FilePreview | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<FilePreview>("dev_read_file", { path });
  } catch {
    return null;
  }
}

/** 选一个游戏目录外的自定义目录（走 Rust 侧的 rfd，和别处一致）。 */
export async function pickDirectory(): Promise<string | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<string | null>("dev_pick_directory");
  } catch {
    return null;
  }
}

/** 一次提交里的一条。对应 `miyin_core::dev::history::Snapshot`。 */
export type CommitSnapshot = { path: string; bytes: number; modified_ms: number };

export type CommitRecord = {
  id: number;
  label: string | null;
  message: string;
  at: number;
  files: CommitSnapshot[];
  /** 提交那一刻的包信息 —— 可捡回来用 */
  meta?: PackageMeta | null;
};

export type HistoryDiff = {
  added: string[];
  removed: string[];
  changed: string[];
  unchanged: number;
};

/** 这个包的提交历史。没人提交过就是空的。 */
export async function history(pkg: string): Promise<CommitRecord[]> {
  if (!isDesktop) return [];
  try {
    return await invoke<CommitRecord[]>("dev_history", { pkg });
  } catch {
    return [];
  }
}

/** 提交一次：把当前勾选的这批文件记成一个版本。 */
export async function commit(
  pkg: string,
  message: string,
  label: string | null,
  files: { path: string; abs: string }[],
  meta: PackageMeta,
): Promise<CommitRecord | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<CommitRecord>("dev_commit", { pkg, message, label, files, meta });
  } catch {
    return null;
  }
}

/** 两版之间差在哪。 */
export async function diff(pkg: string, from: number, to: number): Promise<HistoryDiff | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<HistoryDiff>("dev_diff", { pkg, from, to });
  } catch {
    return null;
  }
}

/** 导出时用的包元数据。对应 `miyin_core::dev::export::PackageMeta`。 */
export type PackageMeta = {
  name: string;
  author?: string | null;
  version?: string | null;
  description?: string | null;
  campaign?: string | null;
  kind?: string | null;
  id?: string | null;
  tags?: string[];
  main_map?: string | null;
  doc?: string | null;
  cover?: string | null;
  modid?: string | null;
  mods?: string[];
};

export type ExportReport = { path: string; files: number; bytes: number };

/** 导出进度：写完几个 / 一共几个。 */
export type ExportProgress = { done: number; total: number };

/** 选一个导出路径（走 Rust 侧的 rfd）。 */
export async function pickExportPath(defaultName: string): Promise<string | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<string | null>("dev_pick_export_path", { defaultName });
  } catch {
    return null;
  }
}

/** 选一个要带进包的文件（封面图 / 说明书 PDF）。 */
export async function pickDocFile(kind: "cover" | "doc"): Promise<string | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<string | null>("dev_pick_doc", { kind });
  } catch {
    return null;
  }
}

/** 取消正在进行的导出（后端会把写了一半的包删掉）。 */
export async function cancelExport(): Promise<void> {
  if (!isDesktop) return;
  try {
    await invoke("dev_cancel_export");
  } catch {
    // 取消了就取消了，没别的可做
  }
}

/** 把勾选的文件打成一个包。 */
export async function exportPackage(
  dest: string,
  meta: PackageMeta,
  files: { path: string; abs: string; is_dir: boolean }[],
  onProgress?: (progress: ExportProgress) => void,
): Promise<ExportReport> {
  if (!isDesktop) throw new Error("浏览器预览导不了 —— 没有 IPC");

  // 大包要几十秒 —— 订阅后端每写完一个文件报的进度
  let unlisten: (() => void) | undefined;
  if (onProgress) {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<ExportProgress>("dev://export", (event) => onProgress(event.payload));
  }
  try {
    // **不吞错**：导出失败的原因必须原样给用户看
    return await invoke<ExportReport>("dev_export", { dest, meta, files });
  } finally {
    unlisten?.();
  }
}

/** 删掉一条提交记录（只删记录，不动任何文件）。 */
export async function forgetCommit(pkg: string, id: number): Promise<CommitRecord[]> {
  if (!isDesktop) return [];
  try {
    return await invoke<CommitRecord[]>("dev_forget_commit", { pkg, id });
  } catch {
    return [];
  }
}
