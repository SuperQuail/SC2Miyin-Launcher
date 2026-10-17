import { createContext, useCallback, useContext, useState, type ReactNode } from "react";

import { isDesktop } from "../../api/bridge";
import { useBusy } from "./useBusy";
import { useInstallation, type InstallationApi } from "./useInstallation";
import { useLibrary, type LibraryApi } from "./useLibrary";
import { useNavigation, type NavigationApi } from "./useNavigation";
import { useNotify, type Notify, type Toast } from "./useNotify";
import { useSaves, type SavesApi } from "./useSaves";
import { useSiteLinks, type SiteLinksApi } from "./useSiteLinks";
import { useTheme, type ThemeApi } from "./useTheme";
import { useTools } from "./useTools";
import { useUpdate, type UpdateApi } from "./useUpdate";

/**
 * 外壳的全局状态，按领域拆开。
 *
 * 原来这些全在 **697 行的 `useLauncher`** 里（安装 / 战役 / 存档 / 更新 / 主题 / 通知
 * 混成一个对象）。现在一个领域一个 hook，跨页面共享的部分在这里组装成 context ——
 * 不再造一个"什么都装"的对象，也不再靠模块级单例。
 */
export interface AppState {
  /** 是不是跑在 Tauri 桌面壳里。 */
  isDesktop: boolean;
  notify: Notify;
  toast: Toast | null;
  /** 关掉当前这条提示（✕ 按钮用） */
  dismiss: () => void;
  /** 有没有会写盘的动作在跑。 */
  busy: boolean;
  /** 正在读战役库。 */
  loading: boolean;
  installation: InstallationApi;
  library: LibraryApi;
  saves: SavesApi;
  update: UpdateApi;
  theme: ThemeApi;
  nav: NavigationApi;
  sites: SiteLinksApi;
  /** 启动时把可选工具装上（默认静默）。 */
  ensureTools: () => Promise<void>;
}

const AppStateContext = createContext<AppState | null>(null);

export function AppStateProvider({ children }: { children: ReactNode }) {
  const { toast, notify, dismiss } = useNotify();
  const { busy, run: runBusy } = useBusy();
  const [loading, setLoading] = useState(false);
  const [ready, setReady] = useState(false);

  const theme = useTheme();
  const sites = useSiteLinks();
  const nav = useNavigation({ notify });

  const library = useLibrary({ notify, busy: runBusy, setLoading });
  const installation = useInstallation({
    notify,
    ready,
    setReady,
    setLoading,
    refreshLibrary: library.refresh,
    applySlots: library.setSlots,
  });
  const saves = useSaves({ notify, busy: runBusy, refreshLibrary: library.refresh });

  const openSettings = useCallback(() => nav.setView("settings"), [nav.setView]);
  const update = useUpdate({ notify, onOpenSettings: openSettings });
  const { ensureTools } = useTools({ notify });

  const value: AppState = {
    isDesktop,
    notify,
    toast,
    dismiss,
    busy,
    loading,
    installation,
    library,
    saves,
    update,
    theme,
    nav,
    sites,
    ensureTools,
  };

  return <AppStateContext.Provider value={value}>{children}</AppStateContext.Provider>;
}

export function useAppState(): AppState {
  const value = useContext(AppStateContext);
  if (!value) throw new Error("useAppState 只能用在 AppStateProvider 里面");
  return value;
}
