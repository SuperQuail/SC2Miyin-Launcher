import { useCallback, useEffect, useState } from "react";

import { forcedThemeFromSearch } from "./flags";

/** 主题三档：跟随系统（默认）/ 日间 / 夜间。 */
export type ThemeMode = "system" | "light" | "dark";
/** 实际生效的那一套。 */
export type Theme = "light" | "dark";

export const THEME_KEY = "miyin.theme";

/**
 * 存下来的偏好。
 *
 * 存在 localStorage —— 这是界面偏好，不必让后端知道；读不出来就跟随系统。
 */
export function readStoredTheme(): ThemeMode {
  try {
    const value = localStorage.getItem(THEME_KEY);
    return value === "light" || value === "dark" || value === "system" ? value : "system";
  } catch {
    // 隐私模式之类不让读，忽略即可
    return "system";
  }
}

/** 写偏好；写不进去就只在当次生效。 */
export function writeStoredTheme(mode: ThemeMode): void {
  try {
    localStorage.setItem(THEME_KEY, mode);
  } catch {
    /* 存不了就算了 */
  }
}

/** 系统当前是不是深色。 */
export function systemPrefersDark(): boolean {
  return typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)").matches : false;
}

/** 「跟随系统」时把两档合一档；其余原样。 */
export function activeTheme(mode: ThemeMode, systemDark: boolean): Theme {
  return mode === "system" ? (systemDark ? "dark" : "light") : mode;
}

/** 一把主题贴到 <html> 上。 */
function apply(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
  // 开发者页那套（Ring UI）认的是这个类名，顺手一起切
  document.documentElement.classList.toggle("ring-ui-theme-dark", theme === "dark");
}

/**
 * 首屏主题。
 *
 * 必须在 render 之前调一次：放到 effect 里会先按浅色画一帧再翻黑，闪一下。
 * `?theme=` 是强制一次的调试开关，不写进 localStorage —— 所以这里只贴不存。
 */
export function applyInitialTheme(search: string): Theme {
  const theme = activeTheme(forcedThemeFromSearch(search) ?? readStoredTheme(), systemPrefersDark());
  apply(theme);
  return theme;
}

/** 主题的当前值与切换。 */
export interface ThemeApi {
  mode: ThemeMode;
  theme: Theme;
  /** 日间 → 夜间 → 跟随系统，循环。 */
  cycleTheme: () => void;
}

export function useTheme(): ThemeApi {
  const forced = forcedThemeFromSearch(location.search);
  const [mode, setMode] = useState<ThemeMode>(() => forced ?? readStoredTheme());
  const [systemDark, setSystemDark] = useState<boolean>(systemPrefersDark);
  const theme = activeTheme(mode, systemDark);

  useEffect(() => {
    apply(theme);
    // 强制的主题不该覆盖用户自己选的偏好
    if (!forced) writeStoredTheme(mode);
  }, [theme, mode, forced]);

  // 系统当前是深是浅在别处改了，跟着走
  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const query = matchMedia("(prefers-color-scheme: dark)");
    const onChange = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  const cycleTheme = useCallback(() => {
    setMode((current) => (current === "light" ? "dark" : current === "dark" ? "system" : "light"));
  }, []);

  return { mode, theme, cycleTheme };
}
