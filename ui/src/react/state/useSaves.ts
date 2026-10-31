import { useCallback, useState } from "react";

import { api } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type {
  InstalledCampaign,
  SaveBackup,
  SaveFile,
  SaveInventory,
  SaveIsolation,
  SaveSet,
  SaveTarget,
} from "../../api/types";
import type { Notify } from "./useNotify";
import { usePermissionGate } from "./usePermission";

export interface SavesOptions {
  notify: Notify;
  /** 去跑一个会写盘的动作，前后自动开关忙碌锁。 */
  busy: <T>(action: () => Promise<T>) => Promise<T>;
  /** 收编之后战役库要跟着重读。 */
  refreshLibrary: () => Promise<void>;
}

/**
 * 后端 `Isolation` 现在多带一条 `warning`（`state.json` 读不出来时的提示，见
 * `saves::isolation`）。`api/types` 里还没这个字段（那边有人在改），就在这里就地扩展 ——
 * 别去动 `ui/src/api/**`。
 */
export type SaveIsolationState = SaveIsolation & { warning?: string | null };

export interface SavesApi {
  /** 游戏目录里已经装着的自制战役（还没收进库的那些）。 */
  installedCampaigns: InstalledCampaign[];
  refreshInstalled: () => Promise<void>;
  collectCampaign: (dir: string) => Promise<void>;
  /** 存档（issue #20）：现在这份 + 已经备份的那些。 */
  saves: SaveSet | null;
  saveBackups: SaveBackup[];
  /** 归属下拉的候选：官方战役 + 每部自制战役（键由后端算，界面不要自己拼）。 */
  saveTargets: SaveTarget[];
  saveIsolation: SaveIsolationState;
  refreshSaves: () => Promise<void>;
  toggleSaveIsolation: (enabled: boolean) => Promise<void>;
  saveCurrentSaves: () => Promise<void>;
  switchSaveProfile: (name: string) => Promise<void>;
  assignSaveProfile: (name: string, slot: string | null) => Promise<void>;
  /** 把游戏里这份存档存进指定的组（存档页的「设为当前」）。 */
  saveIntoProfile: (name: string) => Promise<void>;
  backupSaves: (label: string) => Promise<void>;
  restoreSaves: (name: string) => Promise<void>;
  /** 真机存档清单（Accounts/<账号>/<地域>/Saves/** 下的 .SC2Save 与 Banks）。 */
  inventory: SaveInventory | null;
  /** 扫一遍存档目录（只读）。 */
  scanSaves: () => Promise<void>;
  /** 列出一个组里的存档文件（打开组详情时用）。 */
  groupFiles: (name: string) => Promise<SaveFile[]>;
  /** 把组里的一份存档挪到另一个组。 */
  moveSave: (from: string, file: string, to: string, toLabel?: string) => Promise<void>;
  /** 把一份存档按目标键落位（组自动建/自动绑定）。 */
  moveToKey: (from: string, file: string, key: string, label: string) => Promise<void>;
}

export function useSaves(options: SavesOptions): SavesApi {
  const { notify, busy, refreshLibrary } = options;
  /** 提权闸门：写盘动作统一从它过。 */
  const permission = usePermissionGate();
  const [installedCampaigns, setInstalledCampaigns] = useState<InstalledCampaign[]>([]);
  const [saves, setSaves] = useState<SaveSet | null>(null);
  const [saveBackups, setSaveBackups] = useState<SaveBackup[]>([]);
  const [saveTargets, setSaveTargets] = useState<SaveTarget[]>([]);
  // 后端**默认关着**（用户主动打开才生效，见 saves.rs）：初始值跟着来，
  // 免得刷新的一瞬间界面先闪一下"开着"。
  const [saveIsolation, setSaveIsolation] = useState<SaveIsolationState>({
    enabled: false,
    active: null,
    assignments: {},
  });

  const [inventory, setInventory] = useState<SaveInventory | null>(null);

  /** 扫一遍真存档（Accounts/<账号>/<地域>/Saves/**）。只读，失败给提示。 */
  const scanSaves = useCallback(
    () =>
      busy(async () => {
        try {
          setInventory(await api.scanSaves());
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify],
  );

  const refreshInstalled = useCallback(async () => {
    try {
      setInstalledCampaigns(await api.listInstalledCampaigns());
    } catch {
      setInstalledCampaigns([]);
    }
  }, []);

  /**
   * 把游戏目录里已经装着的战役**收进库**。
   *
   * 原目录**一个字节都不动** —— 那是用户自己放的文件夹，删不删他自己决定。
   * 想清干净就先用启动器"启用"一次库里那份，再自己删掉外面这个。
   */
  const collectCampaign = useCallback(
    (dir: string) =>
      busy(async () => {
        try {
          // 收编的就是游戏目录里那堆自作战役文件夹 —— 归属按「自制战役」定死
          // （它们没有任何元数据，交给后端去猜只会得到"无法识别"）
          await permission.run(
            () => api.importPackageWith(dir, "custom", "rename"),
            { op: "import_package", args: { path: dir, slot: "custom", mode: "rename" } },
          );
          await refreshLibrary();
          await refreshInstalled();
          notify("success", "已加入战役库，原目录未改动；可在战役列表中查看");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, permission, refreshInstalled, refreshLibrary],
  );

  const refreshSaves = useCallback(async () => {
    try {
      setSaves(await api.listSaves());
      setSaveBackups(await api.listSaveBackups());
      setSaveTargets(await api.savesTargets());
      setSaveIsolation(await api.saveIsolation());
    } catch (error) {
      notify("error", errorText(error));
    }
  }, [notify]);

  /**
   * 打开 / 关掉隔离。
   *
   * **默认关着**，用户主动打开才生效；关掉之后不再动 Banks，已经存下的组都留着。
   * 打开的一刻后端会把现在这份 Banks 收成「原版」。
   */
  const toggleSaveIsolation = useCallback(
    (enabled: boolean) =>
      busy(async () => {
        try {
          // 存档在用户目录，本身不需要管理员；不经过提权助手。
          const state = await api.setSaveIsolation(enabled);
          if (state) setSaveIsolation(state);
          await refreshSaves();
          // 还没有存档时后端不会收「原版」（没东西可收），别顺着话术说已经收好了。
          // 走助手重放时拿不到 state，就退回一句不带结论的话
          const message = !enabled
            ? "存档隔离已关闭：不再修改 Banks，已有分组保留"
            : state === undefined
              ? "存档隔离已开启。"
              : state.active
                ? "存档隔离已开启。当前进度已保存为「原版」分组。"
                : "存档隔离已开启。暂无存档，首次游戏后的进度会保存为「原版」分组。";
          notify("success", message);
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 把现在这份存档存回它所属的组。 */
  const saveCurrentSaves = useCallback(
    () =>
      busy(async () => {
        try {
          setSaveIsolation(await api.saveCurrentSaves());
          await refreshSaves();
          notify("success", "当前存档已保存");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 切到某个存档组。 */
  const switchSaveProfile = useCallback(
    (name: string) =>
      busy(async () => {
        try {
          const state = await api.switchSaveProfile(name);
          if (state) setSaveIsolation(state);
          await refreshSaves();
          notify("success", "已切换到存档组：" + name);
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 手动改一个组的归属。 */
  const assignSaveProfile = useCallback(
    async (name: string, slot: string | null) => {
      try {
        setSaveIsolation(await api.assignSaveProfile(name, slot));
        notify("success", slot ? "已归属到该战役" : "已改为「不归属任何战役」");
      } catch (error) {
        notify("error", errorText(error));
      }
    },
    [notify],
  );

  /** 把游戏里这份存档存进指定的组（并把它记为当前组）。 */
  const saveIntoProfile = useCallback(
    (name: string) =>
      busy(async () => {
        try {
          setSaveIsolation(await api.saveIntoProfile(name));
          await refreshSaves();
          notify("success", "当前存档已保存到「" + name + "」");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 列出一个组里的存档文件（打开组详情时用）。读失败给空数组并说一声。 */
  const groupFiles = useCallback(
    (name: string) =>
      busy(async () => {
        try {
          return await api.saveGroupFiles(name);
        } catch (error) {
          notify("error", errorText(error));
          return [];
        }
      }),
    [busy, notify],
  );

  /** 把组里的一份存档挪到另一个组 —— 也就是改这一份归属的战役。 */
  const moveSave = useCallback(
    (from: string, file: string, to: string, toLabel?: string) =>
      busy(async () => {
        try {
          await api.moveSaveFile(from, file, to);
          await refreshSaves();
          // 组目录名对用户没意义，优先用显示名
          notify("success", "「" + file + "」已移动到「" + (toLabel ?? to) + "」");
        } catch (error) {
          // 同名不覆盖：后端会回一条「「X」里已经有同名存档 Y，没有覆盖」，原样给用户看
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 把一份存档挪到某个战役/版本（按键落位；目标组不存在会自动建）。 */
  const moveToKey = useCallback(
    (from: string, file: string, key: string, label: string) =>
      busy(async () => {
        try {
          await api.moveSaveToKey(from, file, key);
          await refreshSaves();
          notify("success", "「" + file + "」已归入「" + label + "」");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 备份现在的存档。label 一般填战役名。 */
  const backupSaves = useCallback(
    (label: string) =>
      busy(async () => {
        try {
          const name = await api.backupSaves(label);
          await refreshSaves();
          notify("success", name ? "存档已备份：" + name : "存档已备份");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  /** 还原一份备份。后端会先把现在的存档另存一份。 */
  const restoreSaves = useCallback(
    (name: string) =>
      busy(async () => {
        try {
          const safety = await api.restoreSaves(name);
          await refreshSaves();
          notify("success", safety ? "已还原。还原前的存档已保存为 " + safety : "已还原");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshSaves],
  );

  return {
    installedCampaigns,
    refreshInstalled,
    collectCampaign,
    saves,
    saveBackups,
    saveTargets,
    saveIsolation,
    refreshSaves,
    toggleSaveIsolation,
    saveCurrentSaves,
    switchSaveProfile,
    assignSaveProfile,
    saveIntoProfile,
    inventory,
    scanSaves,
    groupFiles,
    moveSave,
    moveToKey,
    backupSaves,
    restoreSaves,
  };
}
