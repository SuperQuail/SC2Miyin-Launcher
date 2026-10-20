import { useCallback, useState } from "react";

import { activationConflicts, api } from "../../api/bridge";
import type { ActivationConflict } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type { Preview, SlotView } from "../../api/types";
import type { Notify } from "./useNotify";

/** 待确认的启用：预演已经算好，等用户点头。 */
export interface PendingActivation {
  slot: string;
  variantId: string;
  preview: Preview;
}

/**
 * 待用户裁决的启用冲突：这个战役要写的位置正被**另一个已启用战役**占着。
 *
 * 到这一步**一个字节都还没写盘** —— 查冲突那条命令是只读的。
 */
export interface PendingConflict {
  slot: string;
  variantId: string;
  conflicts: ActivationConflict[];
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
  /** 待裁决的冲突（会覆盖别的已启用战役）；null = 没有冲突要问。 */
  pendingConflict: PendingConflict | null;
  /** 请求启用 —— **先查冲突、再算预演**，两道都过了才写盘。返回是否已经切好。 */
  requestActivation: (slot: string, variantId: string | null) => Promise<boolean>;
  /** 用户在预演里点了「继续」。 */
  confirmActivation: () => Promise<boolean>;
  cancelActivation: () => void;
  /** 用户在冲突框里选了「用新战役的」：照常启用（覆盖对方）。 */
  confirmConflict: () => Promise<boolean>;
  /** 用户在冲突框里选了「保持另一个」：**什么都不做**（状态完全不变）。 */
  cancelConflict: () => void;
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
  const [pendingConflict, setPendingConflict] = useState<PendingConflict | null>(null);

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
  /** 冲突已经问过之后走这里：预演那一关。 */
  const proceedActivation = useCallback(
    async (slot: string, variantId: string) => {
      // 这个战役已经点头过一次了，别再拦预演
      if (acknowledgedSlots().includes(slot)) return activate(slot, variantId);
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
    },
    [activate, notify],
  );

  const requestActivation = useCallback(
    (slot: string, variantId: string | null) => {
      if (variantId === null) return activate(slot, null);
      return busy(async () => {
        try {
          // **先查冲突**：这一步只读，用户点「保持另一个」时一个字节都没动过。
          // 每次都查（不像预演那样只提示一次）—— 撞不撞取决于**别的**战役现在开着什么，
          // 这个前提会变。
          const conflicts = await activationConflicts(slot, variantId);
          if (conflicts.length > 0) {
            setPendingConflict({ slot, variantId, conflicts });
            return false;
          }
          return await proceedActivation(slot, variantId);
        } catch (error) {
          notify("error", errorText(error));
          return false;
        }
      });
    },
    [activate, busy, notify, proceedActivation],
  );

  /** 「用新战役的」：继续走正常流程（预演 → 写盘），对方的文件会被接管。 */
  const confirmConflict = useCallback(async () => {
    const pending = pendingConflict;
    if (!pending) return false;
    setPendingConflict(null);
    return busy(async () => {
      try {
        return await proceedActivation(pending.slot, pending.variantId);
      } catch (error) {
        notify("error", errorText(error));
        return false;
      }
    });
  }, [busy, notify, pendingConflict, proceedActivation]);

  /** 「保持另一个」：什么都不做 —— 现在生效的还是另一个战役。 */
  const cancelConflict = useCallback(() => setPendingConflict(null), []);

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
    pendingConflict,
    requestActivation,
    confirmActivation,
    cancelActivation,
    confirmConflict,
    cancelConflict,
    activate,
    removeVariant,
    launch,
  };
}
