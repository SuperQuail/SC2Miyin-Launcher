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
  /** 交给 DOM —— 鼠标压上去就暂停自动消失（剩余时间冻住，离开接着走） */
  hover: { onMouseEnter: () => void; onMouseLeave: () => void };
};

type Run = { reset: number; life: number; exit: number };

/**
 * 演示计时：进场（挂载时 CSS 自己放）→ 停 `life` 自动退场（退出动画放 `exit`）→ 卸载。
 *
 * 分开两步才看得见退场 —— 直接把元素删掉的话退场动画根本没机会跑。
 *
 * `reset` 由调用方在点「播一遍」时 +1：这里等它真的变了才重新放，
 * 而且**从点击那一刻重新装弹**（不是把旧的剩余时间捡回来），所以连点两次也放得出来。
 * 重放时元素是从"已经卸载"的状态重新挂上去的，CSS 的 animation 自然从头放 —— **调用方不用换 key**。
 *
 * `active` 是 `?toast=` 点名那条：真值时用 `AUTO_MS` 的长时长；变了不算重播。
 */
export function useDemo(active: boolean, reset: number, lifeMs: number, exitMs: number): Demo {
  // 没点名的那几条一开始就是 gone（不渲染）
  const [phase, setPhase] = useState<DemoPhase>(active ? "shown" : "gone");
  const [paused, setPaused] = useState(false);
  // 现算，不等到 effect：挂载当帧就要是"这一条该不该露脸"
  const total = active ? Math.max(lifeMs, AUTO_MS) : lifeMs;

  // 每次演出一个 Run；remainingRef 是"这一条还剩多少毫秒"，悬停时冻住它
  const runRef = useRef<Run>({ reset, life: total, exit: exitMs });
  const remainingRef = useRef(total);

  // 点「播一遍」：reset 变了就把这一条放出来，且从满血重新装弹。
  // （踩过的坑一：早先只重置了剩余时间、没把 phase 从 gone 挪出来 —— 按钮点了没反应。
  //  踩过的坑二：调用方当时还用了 key={+at(id)} 强制重挂，重挂后 runRef 一初始化就等于新值，
  //  这个"变了吗"的判断永远为假 —— 所以那个 key 也一起去掉了：
  //  元素在 gone/leaving 时本来就已经卸载，重新挂载没必要。）
  useEffect(() => {
    if (runRef.current.reset === reset) return;
    runRef.current = { reset, life: total, exit: exitMs };
    remainingRef.current = total;
    setPhase("shown");
  }, [reset, total, exitMs]);

  useEffect(() => {
    if (phase === "gone") return;
    if (paused) return;
    const id = window.setTimeout(() => setPhase((p) => (p === "shown" ? "leaving" : "gone")), remainingRef.current);
    const startedAt = Date.now();
    return () => {
      window.clearTimeout(id);
      const left = remainingRef.current - (Date.now() - startedAt);
      remainingRef.current = Math.max(left, 0);
    };
  }, [phase, paused]);

  return {
    phase,
    hover: {
      onMouseEnter: () => setPaused(true),
      onMouseLeave: () => setPaused(false),
    },
  };
}