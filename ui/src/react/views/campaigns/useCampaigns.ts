import { useCallback, useEffect, useState } from "react";

import type { SlotView } from "../../../api/types";
import type { CampaignsHost } from "./host";
import { VANILLA } from "./host";

/** 这一页自己的"在跑"标记；业务动作的忙碌由外壳的 `busy` 负责。 */
export type CampaignsBusy = "export" | "patch" | null;

/**
 * 战役列表页的状态。
 *
 * 只拿这一页要的东西：当前打开哪个战役、导入还是更新、这一页自己有没有在跑。
 * 业务动作一律交给外壳（见 `host.ts` 的 CampaignActions）。
 */
export function useCampaigns(host: CampaignsHost) {
  /** 当前打开的战役槽位；null = 还在列表页。 */
  const [opened, setOpened] = useState<string | null>(null);
  /** 导入还是更新 —— 两个按钮共用同一个对话框。 */
  const [importMode, setImportMode] = useState<"import" | "update">("import");
  const [busy, setBusy] = useState<CampaignsBusy>(null);

  useEffect(() => {
    host.bootstrap();
    // 支持 #slot=wol 直接打开某个战役（调试与截图用）
    const match = /slot=([a-z]+)/.exec(typeof window === "undefined" ? "" : window.location.hash);
    if (match) setOpened(match[1]);
    // 只在挂载时跑一次：host 的回调每次渲染都是新的，不该进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /**
   * 打开某个战役 —— **同时写进 hash**。
   *
   * 为什么必须写：切到安装页会把 CampaignsView 卸载掉，"打开了哪个战役"是组件内的 state，
   * 回来重新挂载就变回 null 了 —— 用户装完包会掉回主界面（他就报的这个）。挂载时本来就会读
   * `#slot=`，只是从来没人写；写上之后切页/刷新都能回到同一个战役里。
   */
  const openSlot = useCallback((slug: string | null) => {
    setOpened(slug);
    if (typeof window === "undefined") return;
    const url = slug ? "#slot=" + slug : location.pathname + location.search;
    window.history.replaceState(null, "", url);
  }, []);

  const guard = useCallback(async (key: NonNullable<CampaignsBusy>, action: () => Promise<unknown>) => {
    setBusy(key);
    try {
      await action();
    } finally {
      setBusy(null);
    }
  }, []);

  return { opened, setOpened: openSlot, importMode, setImportMode, busy, guard };
}

/** 列表里那些「原版战役」（自制战役有自己的页签）。 */
export function officialSlots(slots: SlotView[]): SlotView[] {
  return slots.filter((slot) => slot.slug !== "custom");
}

/** 当前打开的槽位。 */
export function findSlot(slots: SlotView[], opened: string | null): SlotView | null {
  if (!opened) return null;
  return slots.find((slot) => slot.slug === opened) ?? null;
}

export { VANILLA };
