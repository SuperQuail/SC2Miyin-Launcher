import { useCallback, useEffect, useState } from "react";

import type { SlotView } from "../../../api/types";
import { navigate, setParams, useRoute } from "../../lib/router";
import type { CampaignsHost } from "./host";
import { VANILLA } from "./host";

/** 这一页自己的"在跑"标记；业务动作的忙碌由外壳的 `busy` 负责。 */
export type CampaignsBusy = "export" | "patch" | null;

/**
 * 战役列表页的状态。
 *
 * 「打开的是哪一部」**不在组件里，在 URL 的 `?slot=`**（见 docs/navigation.md）：
 * 切页会把这一页卸载，刷新更是什么都不剩 —— 放进 URL 之后，装完 `back()` 回来、
 * 刷新、深链都自动对。
 *
 * 业务动作一律交给外壳（见 `host.ts` 的 CampaignActions）。
 */
export function useCampaigns(host: CampaignsHost) {
  const route = useRoute();
  /** 导入还是更新 —— 两个按钮共用同一个对话框。 */
  const [importMode, setImportMode] = useState<"import" | "update">("import");
  const [busy, setBusy] = useState<CampaignsBusy>(null);

  /**
   * 当前打开的战役；null = 还在列表页。
   *
   * 只认**这一页真的会列出来的**槽位（自制战役有自己的页签，`slot=custom` 在这儿一律忽略）——
   * 否则从自制战役页回来时，这里会去开一个不属于本页的槽位。
   */
  const opened =
    route.slot && officialSlots(host.slots).some((slot) => slot.slug === route.slot) ? route.slot : null;

  /** 进某个战役是**页面级**跳转（push，能 back）；返回列表是页内收尾（replace，不攒历史）。 */
  const setOpened = useCallback((slug: string | null) => {
    if (slug) navigate({ slot: slug });
    else setParams({ slot: "" });
  }, []);

  useEffect(() => {
    host.bootstrap();
    // 只在挂载时跑一次：host 的回调每次渲染都是新的，不该进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const guard = useCallback(async (key: NonNullable<CampaignsBusy>, action: () => Promise<unknown>) => {
    setBusy(key);
    try {
      await action();
    } finally {
      setBusy(null);
    }
  }, []);

  return { opened, setOpened, importMode, setImportMode, busy, guard };
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
