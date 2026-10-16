import { useCallback, useState } from "react";

import { api } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type { InstalledCampaign, SaveBackup, SaveIsolation, SaveSet } from "../../api/types";
import type { Notify } from "./useNotify";

export interface SavesOptions {
  notify: Notify;
  /** 去跑一个会写盘的动作，前后自动开关忙碌锁。 */
  busy: <T>(action: () => Promise<T>) => Promise<T>;
  /** 收编之后战役库要跟着重读。 */
  refreshLibrary: () => Promise<void>;
}

export interface SavesApi {
  /** 游戏目录里已经装着的自制战役（还没收进库的那些）。 */
  installedCampaigns: InstalledCampaign[];
  refreshInstalled: () => Promise<void>;
  collectCampaign: (dir: string) => Promise<void>;
  /** 存档（issue #20）：现在这份 + 已经备份的那些。 */
  saves: SaveSet | null;
  saveBackups: SaveBackup[];
  saveIsolation: SaveIsolation;
  refreshSaves: () => Promise<void>;
  toggleSaveIsolation: (enabled: boolean) => Promise<void>;
  saveCurrentSaves: () => Promise<void>;
  switchSaveProfile: (name: string) => Promise<void>;
  assignSaveProfile: (name: string, slot: string | null) => Promise<void>;
  backupSaves: (label: string) => Promise<void>;
  restoreSaves: (name: string) => Promise<void>;
}

export function useSaves(options: SavesOptions): SavesApi {
  const { notify, busy, refreshLibrary } = options;
  const [installedCampaigns, setInstalledCampaigns] = useState<InstalledCampaign[]>([]);
  const [saves, setSaves] = useState<SaveSet | null>(null);
  const [saveBackups, setSaveBackups] = useState<SaveBackup[]>([]);
  const [saveIsolation, setSaveIsolation] = useState<SaveIsolation>({
    enabled: false,
    active: null,
    assignments: {},
  });

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
          await api.importPackageWith(dir, null, "rename");
          await refreshLibrary();
          await refreshInstalled();
          notify("success", "已收进库 —— 原目录没动，战役列表里能看到了");
        } catch (error) {
          notify("error", errorText(error));
        }
      }),
    [busy, notify, refreshInstalled, refreshLibrary],
  );

  const refreshSaves = useCallback(async () => {
    try {
      setSaves(await api.listSaves());
      setSaveBackups(await api.listSaveBackups());
      setSaveIsolation(await api.saveIsolation());
    } catch (error) {
      notify("error", errorText(error));
    }
  }, [notify]);

  /** 打开 / 关掉隔离。打开时后端会把现在这份 Banks 收成「原版」。 */
  const toggleSaveIsolation = useCallback(
    (enabled: boolean) =>
      busy(async () => {
        try {
          setSaveIsolation(await api.setSaveIsolation(enabled));
          await refreshSaves();
          notify(
            "success",
            enabled ? "存档隔离已打开 —— 当前进度存成了「原版」" : "存档隔离已关掉（已有的组都留着）",
          );
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
          notify("success", "现在这份存档已存好");
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
          setSaveIsolation(await api.switchSaveProfile(name));
          await refreshSaves();
          notify("success", "已切到存档组：" + name);
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
        notify("success", slot ? "已指派给这个战役" : "已改为「不算任何战役」");
      } catch (error) {
        notify("error", errorText(error));
      }
    },
    [notify],
  );

  /** 备份现在的存档。label 一般填战役名。 */
  const backupSaves = useCallback(
    (label: string) =>
      busy(async () => {
        try {
          const name = await api.backupSaves(label);
          await refreshSaves();
          notify("success", "存档已备份：" + name);
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
          notify("success", safety ? "已还原，还原前那份存为 " + safety : "已还原");
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
    saveIsolation,
    refreshSaves,
    toggleSaveIsolation,
    saveCurrentSaves,
    switchSaveProfile,
    assignSaveProfile,
    backupSaves,
    restoreSaves,
  };
}
