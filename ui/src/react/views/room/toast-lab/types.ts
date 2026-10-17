/**
 * 房间里这几个提示候选共用的两只手。
 *
 * 候选之间**样式不共用**（各写各的 CSS，就是拿来看差别的），
 * 共用的只有：语气、演示计时。
 */
import { useEffect, useRef, useState } from "react";

/** 提示的语气。和生产那个 Toast 用同一套词（react/ui/Toast）。 */
export type LabTone = "info" | "success" | "warning" | "error";

export type DemoPhase = "shown" | "leaving" | "gone";

/** `?toast=` 点名的那个：演示时长给足，方便一张一张截（也免得截图正好撞上退场） */
export const AUTO_MS = 24000;

export type Demo = {
  phase: DemoPhase;
  /** 交给 DOM —— 鼠标压上去就暂停自动消失，和市面上那些一样 */
  hover: { onMouseEnter: () => void; onMouseLeave: () => void };
};

/**
 * 演示计时：进场（挂载时 CSS 自己放）→ 停 `lifeMs` 自动退场（退出动画放 `exitMs`）→ 卸载。
 *
 * 分开两步才看得见退场 —— 直接把元素删掉的话退场动画根本没机会跑。
 *
 * `reset` 由调用方在点「播一遍」时 +1：**这里是重置而不是重播**，重播靠调用方换 `key`
 * 把元素重新挂上，入场动画才会从头放（同一个元素上再挂一次 animation 是不动的）。
 *
 * `active` 是 `?toast=` 点名那条：只说"这次演出算不算数"，**变了不算重播**。
 * （踩过的坑：早先把 `reset` 放在 useEffect 里 setPhase，挂载时那次 effect 一定会跑，
 * 于是没点名的那几个也跟着放，五个候选一起糊在屏幕上。）
 */
export function useDemo(active: boolean, reset: number, lifeMs: number, exitMs: number): Demo {
  // 没点名的那几条一开始就是 gone（不渲染）
  const life = active ? Math.max(lifeMs, AUTO_MS) : lifeMs;
  const [phase, setPhase] = useState<DemoPhase>(active ? "shown" : "gone");
  const [paused, setPaused] = useState(false);

  // "该演了"在渲染期算出来：首次挂载算一次，之后只在 reset（重播）或 active（点名）变化时再算。
  const lastRef = useRef({ reset, active });
  const fresh = useRef(true);
  const runRef = useRef(0);
  if (fresh.current || lastRef.current.reset !== reset || lastRef.current.active !== active) {
    fresh.current = false;
    lastRef.current = { reset, active };
    runRef.current += 1;
  }
  const run = runRef.current;

  useEffect(() => {
    if (!active) return;
    setPhase("shown");
    setPaused(false);
  }, [run, active]);

  useEffect(() => {
    if (paused || phase === "gone") return;
    const next = phase === "shown" ? "leaving" : "gone";
    const delay = phase === "shown" ? life : exitMs;
    const id = window.setTimeout(() => setPhase(next), delay);
    return () => window.clearTimeout(id);
  }, [phase, paused, run, life, exitMs]);

  return {
    phase,
    hover: {
      onMouseEnter: () => setPaused(true),
      onMouseLeave: () => setPaused(false),
    },
  };
}