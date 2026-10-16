import { useCallback, useEffect, useState } from "react";

/**
 * 开发者模式。
 *
 * 存哪：`localStorage`。它是个"开关"而不是业务数据 —— 不进 `data/`，
 * 换台机器/重装启动器就该重新打开一次，这样也不会把作者模式误带给普通用户。
 *
 * 读的一侧（react/state/useNavigation.ts）在启动时直接读 localStorage，所以这里不必广播事件；
 * 只有本 hook 的宿主需要重渲染。
 */
const KEY = "miyin.devMode";

/** 现在开着吗。 */
export function isDevMode(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

/** 开关它。 */
export function setDevMode(on: boolean): void {
  try {
    if (on) localStorage.setItem(KEY, "1");
    else localStorage.removeItem(KEY);
  } catch {
    // 隐私模式之类写不进去 —— 那就当没开，别因此报错
  }
}

/** 给设置页用的开关状态。 */
export function useDevMode(): [boolean, (on: boolean) => void] {
  const [on, setOn] = useState(() => isDevMode());

  // 别的标签页改了也跟上
  useEffect(() => {
    const sync = () => setOn(isDevMode());
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);

  const toggle = useCallback((next: boolean) => {
    setDevMode(next);
    setOn(next);
  }, []);

  return [on, toggle];
}
