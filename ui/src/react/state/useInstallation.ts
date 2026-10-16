import { useCallback, useState } from "react";

import { api } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type { Installation, SlotView } from "../../api/types";
import type { Notify } from "./useNotify";

export interface InstallationOptions {
  notify: Notify;
  /** 战役库读完了没有；bootstrap 只认真第一次。 */
  ready: boolean;
  setReady: (value: boolean) => void;
  setLoading: (value: boolean) => void;
  /** 重新读一遍战役库（选了新目录之后）。 */
  refreshLibrary: () => Promise<void>;
  /** bootstrap 读到的槽位交给库那侧。 */
  applySlots: (slots: SlotView[]) => void;
}

export interface InstallationApi {
  installation: Installation | null;
  libraryRoot: string;
  /** 启动器自己的版本（对外写法，如 0.1.0a3）。 */
  launcherVersion: string;
  /** 首次进入时探测游戏并载入战役库；`ready` 之后是空转。 */
  bootstrap: () => Promise<void>;
  /** 手动指定游戏目录。 */
  chooseGameDirectory: () => Promise<void>;
  /** 在资源管理器中打开路径。 */
  reveal: (path: string) => Promise<void>;
}

export function useInstallation(options: InstallationOptions): InstallationApi {
  const { notify, ready, setReady, setLoading, refreshLibrary, applySlots } = options;
  const [installation, setInstallation] = useState<Installation | null>(null);
  const [libraryRoot, setLibraryRoot] = useState("");
  const [launcherVersion, setLauncherVersion] = useState("");

  const bootstrap = useCallback(async () => {
    // 界面要显示的是**自己**的版本，不是游戏构建号；拿不到就不显示
    void api
      .appVersion()
      .then(setLauncherVersion)
      .catch(() => {
        /* 不该因此打扰用户 */
      });

    if (ready) return;
    setLoading(true);
    try {
      setInstallation(await api.detectInstallation());
      setLibraryRoot(await api.libraryRoot());
      applySlots(await api.listSlots());
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setLoading(false);
      setReady(true);
    }
  }, [ready, applySlots, notify, setLoading, setReady]);

  const chooseGameDirectory = useCallback(async () => {
    try {
      const path = await api.pickGameDirectory();
      if (!path) return;
      setInstallation(await api.setInstallation(path));
      await refreshLibrary();
      notify("success", "已设置游戏目录");
    } catch (error) {
      notify("error", errorText(error));
    }
  }, [notify, refreshLibrary]);

  const reveal = useCallback(async (path: string) => {
    try {
      await api.revealPath(path);
    } catch (error) {
      notify("error", errorText(error));
    }
  }, [notify]);

  return { installation, libraryRoot, launcherVersion, bootstrap, chooseGameDirectory, reveal };
}
