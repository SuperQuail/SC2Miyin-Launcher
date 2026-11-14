import { useCallback, useEffect, useState } from "react";

import { forcedThemeFromSearch } from "./flags";

/** 主题两档：日间 / 夜间。默认夜间，不再跟随系统。 */
export type ThemeMode = "light" | "dark";
/** 实际生效的那一套。 */
export type Theme = ThemeMode;

export const THEME_KEY = "miyin.theme";

/**
 * 存下来的偏好。
 *
 * 没存过、值坏了、或者旧版本留下的 "system"，一律落到夜间 —— 这是默认主题，
 * 不再去读系统设置。
 */
export function readStoredTheme(): ThemeMode {
  try {
    return localStorage.getItem(THEME_KEY) === "light" ? "light" : "dark";
  } catch {
    // 隐私模式之类不让读，仍按默认夜间
    return "dark";
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

/** 把主题贴到 <html> 上。 */
function apply(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
  // 开发者页那套（Ring UI）认的是这个类名，顺手一起切
  document.documentElement.classList.toggle("ring-ui-theme-dark", theme === "dark");
}

/**
 * 首屏主题。
 *
 * 必须在 render 之前调一次：放到 effect 里会先按默认色画一帧再翻，闪一下。
 * 调试参数 theme 只强制这一次，不写进 localStorage。
 */
export function applyInitialTheme(search: string): Theme {
  const theme = forcedThemeFromSearch(search) ?? readStoredTheme();
  apply(theme);
  return theme;
}

/** 主题的当前值与切换。 */
export interface ThemeApi {
  mode: ThemeMode;
  theme: Theme;
  /** 日间 ↔ 夜间。 */
  cycleTheme: () => void;
}

export function useTheme(): ThemeApi {
  const forced = forcedThemeFromSearch(location.search);
  const [mode, setMode] = useState<ThemeMode>(() => forced ?? readStoredTheme());
  const theme = mode;

  useEffect(() => {
    apply(theme);
    // 强制的主题不该覆盖用户自己选的偏好
    if (!forced) writeStoredTheme(mode);
  }, [theme, mode, forced]);

  const cycleTheme = useCallback(() => {
    setMode((current) => (current === "light" ? "dark" : "light"));
  }, []);

  return { mode, theme, cycleTheme };
}
