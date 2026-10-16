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
}

export function useNavigation(options: { notify: Notify }): NavigationApi {
  const { notify } = options;
  const [view, setView] = useState<ViewId>(() =>
    resolveStartView({ search: location.search, hash: location.hash, devMode: isDevMode() }),
  );
  const [cheatsOpen, setCheatsOpen] = useState(false);

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

  return { view, setView, cheatsOpen, openCheats, closeCheats, openRoom, openDevPage };
}
