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
  truncated: boolean;
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
  truncated: false,
  external_roots: [],
  entries: [
    { path: "Maps", abs: "", name: "Maps", is_dir: true, bytes: 0, external: false },
    { path: "Maps/Starcraft Mass Recall", abs: "", name: "Starcraft Mass Recall", is_dir: true, bytes: 0, external: false },
    { path: "Maps/Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map", abs: "", name: "SCMR Campaign Launcher.SC2Map", is_dir: false, bytes: 984064, external: false },
    { path: "Maps/CustomCampaigns/说明.txt", abs: "C:\\示例\\说明.txt", name: "说明.txt", is_dir: false, bytes: 4096, external: false },
    { path: "Maps/CustomCampaigns/封面.png", abs: "C:\\示例\\封面.png", name: "封面.png", is_dir: false, bytes: 317440, external: false },
    { path: "Mods", abs: "", name: "Mods", is_dir: true, bytes: 0, external: false },
    { path: "Mods/SCMRmod.SC2Mod", abs: "C:\\示例\\SCMRmod.SC2Mod", name: "SCMRmod.SC2Mod", is_dir: false, bytes: 984064, external: false },
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
): Promise<CommitRecord | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<CommitRecord>("dev_commit", { pkg, message, label, files });
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
};

export type ExportReport = { path: string; files: number; bytes: number };

/** 选一个导出路径（走 Rust 侧的 rfd）。 */
export async function pickExportPath(defaultName: string): Promise<string | null> {
  if (!isDesktop) return null;
  try {
    return await invoke<string | null>("dev_pick_export_path", { defaultName });
  } catch {
    return null;
  }
}

/** 把勾选的文件打成一个包。 */
export async function exportPackage(
  dest: string,
  meta: PackageMeta,
  files: { path: string; abs: string }[],
): Promise<ExportReport> {
  if (!isDesktop) throw new Error("浏览器预览导不了 —— 没有 IPC");
  // **不吞错**：导出失败的原因（文件没了、勾太多、路径不合法）必须原样给用户看
  return await invoke<ExportReport>("dev_export", { dest, meta, files });
}
