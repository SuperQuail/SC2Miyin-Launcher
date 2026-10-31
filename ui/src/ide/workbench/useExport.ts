import { useCallback, useRef } from "react";

import {
  type DevEntry,
  type ExportProgress,
  type PackageMeta,
  closeWindow,
  exportPackage,
  pickExportPath,
} from "../api";
import { formatBytes, parseList } from "../lib/format";
import type { DevMeta } from "../lib/settings";

export type ExportDeps = {
  pkg: string;
  meta: DevMeta;
  /** 扫到的文件（含目录项，导出时把目录也带上） */
  entries: DevEntry[];
  picked: string[];
  coverPath: string;
  docPath: string;
  /** 版本号的校验错误；非空就不给导 */
  versionError: string;
  setStatus: (text: string) => void;
  /** 写进状态栏与离开确认框的进度 */
  setExportProgress: (progress: ExportProgress | null) => void;
  /** 取消导出、真正收尾之后要去哪 */
  pendingLeave: { current: null | "home" | "close" };
};

/**
 * 导出打包。
 *
 * **这个 hook 不持有 state** —— 导出进度要显示在状态栏与离开确认框里，所以进度归
 * Workbench 管，这里只负责跑流程与收尾。收尾那几句留在 finally 里：包删干净了才走人。
 *
 * 参数走 ref 取最新值：导出是几十秒的活，中途改包名不该把这次导出指向别处。
 */
export function useExport({ setStatus, setExportProgress, pendingLeave, ...rest }: ExportDeps) {
  const latest = useRef(rest);
  latest.current = rest;

  const run = useCallback(
    async (files: { path: string; abs: string; is_dir: boolean }[], payload: PackageMeta, dest: string) => {
      try {
        setStatus("正在打包…");
        setExportProgress({ done: 0, total: files.length });
        const report = await exportPackage(dest, payload, files, setExportProgress);
        setStatus("导出完成：" + report.files + " 个文件 · " + formatBytes(report.bytes) + " → " + report.path);
      } catch (error) {
        const text = error instanceof Error ? error.message : String(error);
        setStatus(text.includes("已取消") ? "已取消导出，写了一半的包已删掉" : "导出失败：" + text);
      } finally {
        setExportProgress(null);
        // 取消导出之后才真的走人 —— 走之前包已经删干净了
        const action = pendingLeave.current;
        pendingLeave.current = null;
        if (action === "home") location.href = "/index.html";
        else if (action === "close") void closeWindow();
      }
    },
    [pendingLeave, setExportProgress, setStatus],
  );

  return useCallback(async () => {
    const now = latest.current;
    if (!now.pkg.trim()) {
      setStatus("先给这个包起个名字");
      return;
    }
    if (now.picked.length === 0) {
      setStatus("一个文件都没勾");
      return;
    }
    if (now.versionError) {
      setStatus("版本号有问题：" + now.versionError);
      return;
    }
    const dest = await pickExportPath(now.pkg.trim() + ".zip");
    if (!dest) return;

    // 封面 / 说明书是从别处挑的：给它们在包里安排一个位置，再把路径写进元数据
    const coverName = now.coverPath ? "cover" + (now.coverPath.match(/\.[A-Za-z0-9]+$/)?.[0] ?? "") : "";
    const docName = now.docPath ? (now.docPath.split(/[\\/]/).pop() ?? "") : "";
    const payload: PackageMeta = {
      name: now.pkg.trim(),
      author: now.meta.author.trim() || null,
      version: now.meta.version.trim() || null,
      description: now.meta.description.trim() || null,
      campaign: now.meta.campaign.trim() || null,
      kind: now.meta.kind || null,
      id: now.meta.id.trim() || null,
      tags: parseList(now.meta.tags),
      main_map: now.meta.campaign === "" ? now.meta.mainMap || null : null,
      cover: coverName || null,
      doc: docName || null,
      modid: now.meta.modid.trim() || null,
      mods: parseList(now.meta.mods),
    };
    const files = now.entries
      .filter((entry) => now.picked.includes(entry.path))
      .map((entry) => ({ path: entry.path, abs: entry.abs, is_dir: entry.is_dir }));
    if (coverName) files.push({ path: coverName, abs: now.coverPath, is_dir: false });
    if (docName) files.push({ path: docName, abs: now.docPath, is_dir: false });

    await run(files, payload, dest);
  }, [run, setStatus]);
}
