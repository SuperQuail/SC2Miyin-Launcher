import { useCallback, useEffect, useState } from "react";

import {
  type CommitRecord,
  type DevEntry,
  type HistoryDiff,
  commit as commitVersion,
  diff as diffVersions,
  forgetCommit,
  history as loadHistory,
  isDesktop,
} from "../api";
import type { DevMeta } from "../lib/settings";

export type HistoryDeps = {
  pkg: string;
  picked: string[];
  entries: DevEntry[];
  meta: DevMeta;
  coverPath: string;
  docPath: string;
  setStatus: (text: string) => void;
  setPkg: (value: string) => void;
  setMeta: (meta: DevMeta) => void;
  setPicked: (paths: string[]) => void;
};

/**
 * 版本管理（提交 / 日志）的状态与动作。
 *
 * 说清楚一件事：我们只记了路径 + 大小 + 修改时间，**没有存内容副本**，
 * 所以"应用这一版"是恢复那一版的勾选与包信息，不是把文件内容回退。
 * 内容级的回退要 SC2Diff（见 docs/developer-workflow.md §4.3）。
 */
export function useHistory({
  pkg,
  picked,
  entries,
  meta,
  coverPath,
  docPath,
  setStatus,
  setPkg,
  setMeta,
  setPicked,
}: HistoryDeps) {
  const [commits, setCommits] = useState<CommitRecord[]>([]);
  const [logPicked, setLogPicked] = useState<number | null>(null);
  const [pendingDiff, setPendingDiff] = useState<HistoryDiff | null>(null);
  const [message, setMessage] = useState("");

  const refreshHistory = useCallback(async () => {
    if (!pkg.trim()) {
      setCommits([]);
      return;
    }
    setCommits(await loadHistory(pkg.trim()));
    setLogPicked(null);
    setPendingDiff(null);
  }, [pkg]);

  useEffect(() => {
    void refreshHistory();
  }, [refreshHistory]);

  const submit = useCallback(async () => {
    if (!pkg.trim()) {
      setStatus("先给这个包起个名字");
      return;
    }
    if (!message.trim()) {
      setStatus("写一句这次改了什么再提交");
      return;
    }
    // 提交历史只记文件 —— 那是一条条"这一版有哪些文件"
    const files = entries
      .filter((entry) => picked.includes(entry.path) && !entry.is_dir)
      .map((entry) => ({ path: entry.path, abs: entry.abs }));
    const item = await commitVersion(pkg.trim(), message, null, files, {
      name: pkg.trim(),
      author: meta.author.trim() || null,
      version: meta.version.trim() || null,
      description: meta.description.trim() || null,
      campaign: meta.campaign.trim() || null,
      kind: meta.kind || null,
      id: meta.id.trim() || null,
      tags: meta.tags.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean),
      main_map: meta.campaign === "" ? meta.mainMap || null : null,
      cover: coverPath ? "cover" + (coverPath.match(/\.[A-Za-z0-9]+$/)?.[0] ?? "") : null,
      doc: docPath ? (docPath.split(/[\\/]/).pop() ?? null) : null,
      modid: meta.modid.trim() || null,
      mods: meta.mods.split(/[,，]/).map((mod) => mod.trim()).filter(Boolean),
    });
    if (!item) {
      setStatus(isDesktop ? "提交失败" : "浏览器预览提交不了 —— 没有 IPC");
      return;
    }
    setMessage("");
    await refreshHistory();
    setStatus("已提交 #" + item.id + "：" + item.files.length + " 个文件");
  }, [pkg, message, entries, picked, meta, coverPath, docPath, refreshHistory, setStatus]);

  /** 把包信息恢复成某次提交时的样子 —— 手填的东西不用再填一遍。 */
  const applyMeta = useCallback(
    (item: CommitRecord) => {
      const saved = item.meta;
      if (!saved) return;
      if (saved.name) setPkg(saved.name);
      setMeta({
        author: saved.author ?? "",
        version: saved.version ?? "",
        description: saved.description ?? "",
        tags: (saved.tags ?? []).join(", "),
        id: saved.id ?? "",
        kind: saved.kind ?? "campaign",
        campaign: saved.campaign ?? "",
        mainMap: saved.main_map ?? "",
        modid: saved.modid ?? "",
        mods: (saved.mods ?? []).join(", "),
      });
      setStatus("包信息已恢复成 #" + item.id + " 那一版");
    },
    [setPkg, setMeta, setStatus],
  );

  /** 回到某次提交：**把勾选恢复成那一版的文件清单**。 */
  const applyCommit = useCallback(
    (item: CommitRecord) => {
      const known = new Set(entries.map((entry) => entry.path));
      const paths = item.files.map((file) => file.path).filter((path) => known.has(path));
      const missing = item.files.length - paths.length;
      setPicked(paths);
      if (item.meta) applyMeta(item);
      setStatus(
        "已回到 #" + item.id + " 的勾选：" + paths.length + " 个文件" +
          (missing > 0 ? "（有 " + missing + " 个现在扫不到了，没勾上）" : "") +
          " —— 注意：这是勾选回退，内容回退要 SC2Diff",
      );
    },
    [entries, applyMeta, setStatus, setPicked],
  );

  /** 删掉一条提交记录 —— 只删记录，不动文件。 */
  const forget = useCallback(
    async (item: CommitRecord) => {
      const next = await forgetCommit(pkg.trim(), item.id);
      setCommits(next);
      setLogPicked(null);
      setPendingDiff(null);
      setStatus("已删掉记录 #" + item.id + "（文件和包信息都没动）");
    },
    [pkg, setStatus],
  );

  /** 点一条提交记录：选中它并算出跟上一条的差。 */
  const pickCommit = useCallback(
    async (item: CommitRecord, previous: CommitRecord | undefined) => {
      setLogPicked(item.id);
      setPendingDiff(previous ? await diffVersions(pkg.trim(), previous.id, item.id) : null);
    },
    [pkg],
  );

  return {
    commits,
    logPicked,
    pendingDiff,
    message,
    setMessage,
    setLogPicked,
    setPendingDiff,
    submit,
    applyCommit,
    forget,
    pickCommit,
  };
}
