import { useCallback, useRef, useState } from "react";

import { api, isDesktop } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type { Staged, UpdateCheck, UpdateProgress } from "../../api/types";
import { skipsAutoUpdate } from "./flags";
import { appendLog } from "./log";
import type { Notify } from "./useNotify";

/** 「不再提示」记在这里；只记版本号，下次开新版还会提示。 */
const MUTED_KEY = "miyin.update.mutedVersion";

/** 读「不再提示这个版本」。隐私模式之类不让读，当没设过。 */
export function readMutedVersion(): string {
  try {
    return localStorage.getItem(MUTED_KEY) ?? "";
  } catch {
    return "";
  }
}

/** 记「不再提示这个版本」；空串表示清掉。 */
export function writeMutedVersion(version: string): void {
  try {
    if (version) localStorage.setItem(MUTED_KEY, version);
    else localStorage.removeItem(MUTED_KEY);
  } catch {
    /* 存不了就算了，顶多多提示几次 */
  }
}

export interface UpdateOptions {
  notify: Notify;
  /** 公告里点「下载并安装」要先跳到设置页，用户才看得到进度条与终端。 */
  onOpenSettings: () => void;
}

export interface UpdateApi {
  /** 更新过程的日志（内嵌终端的内容），最多 400 行。 */
  updateLogs: string[];
  /** 最近一次检查结果。 */
  updateCheck: UpdateCheck | null;
  /** 下载好、等着装的东西。 */
  updateStaged: Staged | null;
  updateProgress: UpdateProgress | null;
  updateChecking: boolean;
  updateDownloading: boolean;
  /** 有可用更新时的版本号，供顶栏角标；null 表示没有。 */
  updateAvailable: string | null;
  /** 启动时的更新公告。 */
  updateNoticeVisible: boolean;
  /** 公告上的「不再提示这个版本」勾选状态。 */
  updateNoticeMuted: boolean;
  setUpdateNoticeMuted: (value: boolean) => void;
  /** 下载完成后弹的那个"要重启了"对话框。 */
  showRestartPrompt: boolean;
  setShowRestartPrompt: (value: boolean) => void;
  dismissUpdateNotice: () => void;
  muteThisVersion: () => void;
  checkUpdateNow: (quiet?: boolean) => Promise<void>;
  autoCheckUpdate: () => Promise<void>;
  downloadUpdateNow: () => Promise<void>;
  downloadFromNotice: () => Promise<void>;
  applyUpdateNow: () => Promise<void>;
}

export function useUpdate(options: UpdateOptions): UpdateApi {
  const { notify, onOpenSettings } = options;

  const [updateLogs, setUpdateLogs] = useState<string[]>([]);
  const [updateCheck, setUpdateCheck] = useState<UpdateCheck | null>(null);
  const [updateStaged, setUpdateStaged] = useState<Staged | null>(null);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateDownloading, setUpdateDownloading] = useState(false);
  const [updateNoticeVisible, setUpdateNoticeVisible] = useState(false);
  const [updateNoticeMuted, setUpdateNoticeMuted] = useState(false);
  const [showRestartPrompt, setShowRestartPrompt] = useState(false);

  /**
   * 检查结果也留一份 ref。
   *
   * `autoCheckUpdate` 在 `await checkUpdateNow()` 之后要立刻读它，而 setState
   * 在同一个闭包里还看不到新值 —— 原来那套响应式是同步的，这里得自己补上。
   */
  const checkRef = useRef<UpdateCheck | null>(null);
  const checkingRef = useRef(false);
  const downloadingRef = useRef(false);
  const eventsBound = useRef(false);

  const pushUpdateLog = useCallback((line: string) => {
    setUpdateLogs((current) => appendLog(current, line));
  }, []);

  /**
   * 订阅后端推来的日志与进度（只订阅一次）。
   *
   * **不要在这里判断 isDesktop** —— 浏览器演示模式的桥接也提供了这两个订阅，
   * 加了判断的话演示模式就收不到任何日志（终端永远只有一行），踩过一次。
   */
  const bindUpdateEvents = useCallback(async () => {
    if (eventsBound.current) return;
    eventsBound.current = true;
    try {
      await api.onUpdateLog((line) => pushUpdateLog(line));
      await api.onUpdateProgress((progress) => setUpdateProgress(progress));
    } catch {
      eventsBound.current = false;
    }
  }, [pushUpdateLog]);

  /** 跑一次检查。quiet=true 时不弹提示（启动时自动扫描用）。 */
  const checkUpdateNow = useCallback(
    async (quiet = false) => {
      if (checkingRef.current) return;
      checkingRef.current = true;
      setUpdateChecking(true);
      setUpdateProgress(null);
      setUpdateStaged(null);

      try {
        await bindUpdateEvents();
        pushUpdateLog("—— 开始检查更新 ——");
        const result = await api.checkUpdate();
        checkRef.current = result;
        setUpdateCheck(result);

        if (result.error) {
          pushUpdateLog("检查失败：" + result.error);
          if (!quiet) notify("info", "检查更新失败：" + result.error);
        } else if (result.available && result.latest) {
          if (!quiet) notify("success", "发现新版本 " + result.latest.version);
        } else if (!quiet) {
          notify("info", "已经是最新版本");
        }
      } catch (error) {
        pushUpdateLog("检查出错：" + errorText(error));
        if (!quiet) notify("error", errorText(error));
      } finally {
        checkingRef.current = false;
        setUpdateChecking(false);
      }
    },
    [bindUpdateEvents, notify, pushUpdateLog],
  );

  /** 下载更新包。 */
  const downloadUpdateNow = useCallback(async () => {
    const release = checkRef.current?.latest;
    if (!release || downloadingRef.current) return;

    downloadingRef.current = true;
    setUpdateDownloading(true);
    setUpdateProgress(null);
    pushUpdateLog("—— 开始下载 " + release.version + " ——");

    try {
      const staged = await api.downloadUpdate(release.version);
      setUpdateStaged(staged);
      // 公告与重启弹窗不同时出现
      setUpdateNoticeVisible(false);
      pushUpdateLog("下载完成。点击「安装并重启」以更新到 " + staged.version);
      // 下载完成 → 弹窗告诉用户需要重启
      setShowRestartPrompt(true);
    } catch (error) {
      pushUpdateLog("下载失败：" + errorText(error));
      notify("error", errorText(error));
    } finally {
      downloadingRef.current = false;
      setUpdateDownloading(false);
    }
  }, [notify, pushUpdateLog]);

  /**
   * 启动时自动扫一次：**不需要用户手点**；有新版就把公告弹出来。
   *
   * `?noupdate=1` 跳过这次自动检查 —— 截图 / 排查用。
   */
  const autoCheckUpdate = useCallback(async () => {
    if (skipsAutoUpdate(location.search)) return;

    await checkUpdateNow(true);

    const result = checkRef.current;
    if (result?.available && result.latest) {
      const version = result.latest.version;

      if (readMutedVersion() === version) {
        // 用户勾过「不再提示这个版本」—— 角标照常有，但不再弹窗
        pushUpdateLog("已设置不再提示 " + version + "，仅显示右上角更新标记");
      } else {
        setUpdateNoticeVisible(true);
      }

      // 浏览器演示模式：把下载流程也演一遍，
      // 好让开发与截图能看到进度条与重启弹窗的实际样子（真实运行不会这样）。
      if (!isDesktop) {
        setTimeout(() => void downloadUpdateNow(), 6000);
      }
    }
  }, [checkUpdateNow, downloadUpdateNow, pushUpdateLog]);

  /** 关掉公告（这次不提示了，下次启动还会提示）。 */
  const dismissUpdateNotice = useCallback(() => {
    setUpdateNoticeVisible(false);
    setUpdateNoticeMuted(false);
  }, []);

  /** 「不再提示这个版本」：记下版本号，以后不再弹；等出更新的版本再弹。 */
  const muteThisVersion = useCallback(() => {
    const version = checkRef.current?.latest?.version ?? "";
    writeMutedVersion(version);
    setUpdateNoticeVisible(false);
    setUpdateNoticeMuted(false);
    pushUpdateLog("已设置为不再提示 " + version);
  }, [pushUpdateLog]);

  /**
   * 公告里点「下载并安装」。
   *
   * **不在公告里直接下载** —— 那样用户只看到一个转圈，看不到进度条和内嵌终端，
   * 容易以为卡死了。正确做法是：收起公告 -> 跳到设置页的更新面板 -> 在那里开始下载。
   */
  const downloadFromNotice = useCallback(async () => {
    dismissUpdateNotice();
    onOpenSettings();

    // 等面板挂载并滚到位置，再开始下载 —— 太早开始，进度条会先于用户视线出现
    await new Promise((resolve) => setTimeout(resolve, 350));
    await downloadUpdateNow();
  }, [dismissUpdateNotice, downloadUpdateNow, onOpenSettings]);

  /** 换上新版本（程序会退出，由替换脚本接管）。 */
  const applyUpdateNow = useCallback(async () => {
    try {
      pushUpdateLog("正在准备替换，启动器即将退出…");
      await api.applyUpdate();
    } catch (error) {
      pushUpdateLog("替换失败：" + errorText(error));
      notify("error", errorText(error));
      setShowRestartPrompt(false);
    }
  }, [notify, pushUpdateLog]);

  return {
    updateLogs,
    updateCheck,
    updateStaged,
    updateProgress,
    updateChecking,
    updateDownloading,
    updateAvailable: updateCheck?.available ? (updateCheck.latest?.version ?? null) : null,
    updateNoticeVisible,
    updateNoticeMuted,
    setUpdateNoticeMuted,
    showRestartPrompt,
    setShowRestartPrompt,
    dismissUpdateNotice,
    muteThisVersion,
    checkUpdateNow,
    autoCheckUpdate,
    downloadUpdateNow,
    downloadFromNotice,
    applyUpdateNow,
  };
}
