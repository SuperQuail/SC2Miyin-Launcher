import { useCallback, useState } from "react";

import { isDevMode } from "../lib/devMode";
import { back, navigate, useRoute } from "../lib/router";
import type { ViewId } from "./flags";
import type { Notify } from "./useNotify";

export interface NavigationApi {
  /** 顶层视图 —— 来自 URL 的 `?view=`（见 docs/navigation.md）。 */
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
  /** 安装页的入口：战役包还是自制战役包（决定预检按哪套规则认归属）—— 来自 `?entry=`。 */
  installEntry: "campaign" | "custom";
  /** 进「安装整合包」页。**先到页面、再在那里选文件** —— 别一上来就弹系统选择器。 */
  openInstall: (entry?: "campaign" | "custom") => void;
  /** 回上一处（装完 / 返回）。 */
  back: () => void;
}

/**
 * 导航状态全在 URL 里，这里只是把它读出来 + 提供几个动作。
 *
 * 以前 `view` 是自己的 useState（刷新就丢）、`installReturn` 专门记"从哪来"
 * （自制战役页那条路忘了适配就是 bug）—— 现在都不用：`?view=&slot=` 就是状态，
 * 回原处一律 `back()`。
 */
export function useNavigation(options: { notify: Notify }): NavigationApi {
  const { notify } = options;
  const route = useRoute();
  const [cheatsOpen, setCheatsOpen] = useState(false);

  /** 切页是**页面级**跳转：push，能 back 回上一页。 */
  const setView = useCallback((view: ViewId) => navigate({ view }), []);

  const openCheats = useCallback(() => setCheatsOpen(true), []);
  const closeCheats = useCallback(() => setCheatsOpen(false), []);

  /**
   * 这里只做一次门槛检查 —— 开关存在 localStorage，而房间是"看一眼控件"的页面，
   * 不值得为它把状态同步进外壳。关掉开发者模式后就进不来了
   * （`?view=room` 深链也一样：router 解析时就要求开发者模式）。
   */
  const openRoom = useCallback(() => {
    if (!isDevMode()) {
      notify("info", "先打开「开发者模式」");
      return;
    }
    navigate({ view: "room" });
  }, [notify]);

  const openDevPage = useCallback(() => {
    location.href = "/ide.html";
  }, []);

  const openInstall = useCallback((entry: "campaign" | "custom" = "campaign") => {
    // 只写"位置"：当前页与 `?slot=` 都留着，装完 back() 就回到原处
    navigate({ view: "install", entry });
  }, []);

  return {
    view: route.view,
    setView,
    cheatsOpen,
    openCheats,
    closeCheats,
    openRoom,
    openDevPage,
    installEntry: route.entry === "custom" ? "custom" : "campaign",
    openInstall,
    back,
  };
}
