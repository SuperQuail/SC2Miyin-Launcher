import { useCallback, useState } from "react";

import { isDevMode } from "../lib/devMode";
import { resolveStartView, type ViewId } from "./flags";
import type { Notify } from "./useNotify";

export interface NavigationApi {
  /** 顶层视图。 */
  view: ViewId;
  setView: (view: ViewId) => void;
  /** 作弊码这类查询工具走弹层，不占标签位。 */
  cheatsOpen: boolean;
  openCheats: () => void;
  closeCheats: () => void;
  /** 进开发者房间（要求开发者模式开着）。 */
  openRoom: () => void;
  /** 开发者页是独立入口（AGENTS §18），整页跳过去。 */
  openDevPage: () => void;
  /** 安装页的入口：战役包还是自制战役包（决定预检按哪套规则认归属）。 */
  installEntry: "campaign" | "custom";
  /** 进「安装整合包」页。**先到页面、再在那里选文件** —— 别一上来就弹系统选择器。 */
  openInstall: (entry?: "campaign" | "custom") => void;
}

export function useNavigation(options: { notify: Notify }): NavigationApi {
  const { notify } = options;
  const [view, setView] = useState<ViewId>(() =>
    resolveStartView({ search: location.search, hash: location.hash, devMode: isDevMode() }),
  );
  const [cheatsOpen, setCheatsOpen] = useState(false);
  const [installEntry, setInstallEntry] = useState<"campaign" | "custom">("campaign");

  const openCheats = useCallback(() => setCheatsOpen(true), []);
  const closeCheats = useCallback(() => setCheatsOpen(false), []);

  /**
   * 这里只做一次门槛检查 —— 开关存在 localStorage，而房间是"看一眼控件"的页面，
   * 不值得为它把状态同步进外壳。关掉开发者模式后就进不来了。
   */
  const openRoom = useCallback(() => {
    if (!isDevMode()) {
      notify("info", "先打开「开发者模式」");
      return;
    }
    setView("room");
  }, [notify]);

  const openDevPage = useCallback(() => {
    location.href = "/ide.html";
  }, []);

  const openInstall = useCallback((entry: "campaign" | "custom" = "campaign") => {
    setInstallEntry(entry);
    setView("install");
  }, []);

  return { view, setView, cheatsOpen, openCheats, closeCheats, openRoom, openDevPage, installEntry, openInstall };
}
