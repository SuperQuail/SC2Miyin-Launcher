import { useCallback, useState } from "react";

import { api } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type { Preview, SlotView } from "../../api/types";
import type { Notify } from "./useNotify";

/** 待确认的启用：预演已经算好，等用户点头。 */
export interface PendingActivation {
  slot: string;
  variantId: string;
  preview: Preview;
}

export interface LibraryOptions {
  notify: Notify;
  /** 去跑一个会写盘的动作，前后自动开关忙碌锁。 */
  busy: <T>(action: () => Promise<T>) => Promise<T>;
  setLoading: (value: boolean) => void;
}

export interface LibraryApi {
  slots: SlotView[];
  /** bootstrap 把读到的槽位交进来。 */
  setSlots: (slots: SlotView[]) => void;
  /** 重新读取槽位。 */
  refresh: () => Promise<void>;
  /** 被拖进窗口的压缩包路径（一次可以拖好几个）；安装页取走后会清空。 */
  droppedPackages: string[];
  setDroppedPackages: (paths: string[]) => void;
  pendingActivation: PendingActivation | null;
  /** 请求启用 —— **先算预演，再让用户点头**。返回是否已经切好。 */
  requestActivation: (slot: string, variantId: string | null) => Promise<boolean>;
  /** 用户在预演里点了「继续」。 */
  confirmActivation: () => Promise<boolean>;
  cancelActivation: () => void;
  /** 启用某个版本（variantId 为 null 表示切回原版战役）。 */
  activate: (slot: string, variantId: string | null) => Promise<boolean>;
  /** 从库里删除一个版本。 */
  removeVariant: (slot: string, variantId: string) => Promise<boolean>;
  /** 启动游戏。 */
  launch: () => Promise<void>;
}

/** 已经跟他确认过"会改动哪些文件"的战役（每个战役只提示一次）。 */
const ACK_KEY = "miyin.activationAcknowledged";

function acknowledgedSlots(): string[] {
  try {
    const raw = localStorage.getItem(ACK_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : [];
  } catch {
    // 读不出来就当没记过 —— 多提示一次，总比静默改文件强
    return [];
  }
}

function rememberAcknowledged(slot: string): void {
  try {
    const list = acknowledgedSlots();
    if (!list.includes(slot)) localStorage.setItem(ACK_KEY, JSON.stringify([...list, slot]));
  } catch {
    // 存不下就每次都提示，功能不受影响
  }
}

export function useLibrary(options: LibraryOptions): LibraryApi {
  const { notify, busy, setLoading } = options;
  const [slots, setSlots] = useState<SlotView[]>([]);
  const [droppedPackages, setDroppedPackages] = useState<string[]>([]);
  const [pendingActivation, setPendingActivation] = useState<PendingActivation | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setSlots(await api.listSlots());
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setLoading(false);
    }
  }, [notify, setLoading]);

  const activate = useCallback(
    (slot: string, variantId: string | null) =>
      busy(async () => {
        try {
          const warnings = await api.activateVariant(slot, variantId);
          await refresh();
          if (warnings.length) {
            notify("info", warnings[0]);
          } else if (variantId === null) {
            notify("success", "已切回原版战役");
          } else {
            notify("success", "已启用：" + variantId);
          }
          return true;
        } catch (error) {
          notify("error", errorText(error));
          return false;
        }
      }),
    [busy, notify, refresh],
  );

  /**
   * 请求启用 —— **先算预演，再让用户点头**。
   *
   * **每个战役只提示一次**：预演说的是"这个包会往游戏目录里放什么"，同一个战役的
   * 第二次启用基本是同一份清单，每次都拦一下只会烦人。承认过一次就记下来（见
   * `acknowledgedSlots`），之后直接启用 —— 想看清单还有预检那一步。
   *
   *
   * 写盘闸门放开到「安装目录里任意位置」之后，这是唯一一眼：
   * 会新增什么、覆盖什么（覆盖前会备份）、把谁的东西接管过来、删掉什么。
   * 切回原版不拦（那只是撤下我们装的东西）。
   */
  const requestActivation = useCallback(
    (slot: string, variantId: string | null) => {
      if (variantId === null) return activate(slot, null);
      // 这个战役已经点头过一次了，别再拦
      if (acknowledgedSlots().includes(slot)) return activate(slot, variantId);
      return busy(async () => {
        try {
          const preview = await api.previewActivation(slot, variantId);
          if (!preview) return await activate(slot, variantId);
          const touched =
            preview.add.length +
            preview.overwrite.length +
            preview.takeover.length +
            preview.delete.length;
          // 没什么可看的就别拦路
          if (touched === 0) return await activate(slot, variantId);
          setPendingActivation({ slot, variantId, preview });
          return false;
        } catch (error) {
          notify("error", errorText(error));
          return false;
        }
      });
    },
    [activate, busy, notify],
  );

  const confirmActivation = useCallback(async () => {
    if (!pendingActivation) return false;
    rememberAcknowledged(pendingActivation.slot);
    setPendingActivation(null);
    return activate(pendingActivation.slot, pendingActivation.variantId);
  }, [activate, pendingActivation]);

  const cancelActivation = useCallback(() => setPendingActivation(null), []);

  const removeVariant = useCallback(
    (slot: string, variantId: string) =>
      busy(async () => {
        try {
          await api.deleteVariant(slot, variantId);
          await refresh();
          notify("success", "已删除：" + variantId);
          return true;
        } catch (error) {
          notify("error", errorText(error));
          return false;
        }
      }),
    [busy, notify, refresh],
  );

  const launch = useCallback(async () => {
    try {
      await api.launchGame();
      notify("success", "正在启动星际争霸 II…");
    } catch (error) {
      notify("error", errorText(error));
    }
  }, [notify]);

  return {
    slots,
    setSlots,
    refresh,
    droppedPackages,
    setDroppedPackages,
    pendingActivation,
    requestActivation,
    confirmActivation,
    cancelActivation,
    activate,
    removeVariant,
    launch,
  };
}
