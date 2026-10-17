import { useCallback, useRef, useState } from "react";

/** 提示的语气。 */
export type ToastKind = "info" | "success" | "warning" | "error";

export interface Toast {
  kind: ToastKind;
  message: string;
  /** 正在退场：外壳先把它置 true，让退出动画放完再卸载 */
  leaving?: boolean;
}

/** 说话的口子：别的组件只认它，不认外壳是谁。 */
export type Notify = (kind: ToastKind, message: string) => void;

/** 一条提示挂多久。 */
export const TOAST_MS = 4200;
/** 退场动画时长 —— 和 `ui/toast.css` 里的 toast-out 对齐。 */
const LEAVE_MS = 180;

/**
 * 右下角那条提示。
 *
 * 状态跟着外壳走（不散在模块里）：提示只由外壳渲染，别的组件拿 `notify` 说话。
 *
 * **两段计时**：`TOAST_MS - LEAVE_MS` 的时候先标记 leaving（让退出动画开始），
 * 再过 LEAVE_MS 才真正卸载 —— 只 setToast(null) 的话会硬切，没有退场。
 */
export function useNotify(): { toast: Toast | null; notify: Notify; dismiss: () => void } {
  const [toast, setToast] = useState<Toast | null>(null);
  const timers = useRef<ReturnType<typeof setTimeout>[]>([]);

  const clearTimers = useCallback(() => {
    for (const timer of timers.current) clearTimeout(timer);
    timers.current = [];
  }, []);

  const schedule = useCallback(
    (fn: () => void, ms: number) => {
      timers.current.push(setTimeout(fn, ms));
    },
    [],
  );

  const notify = useCallback<Notify>(
    (kind, message) => {
      clearTimers();
      setToast({ kind, message, leaving: false });
      schedule(() => setToast((current) => (current ? { ...current, leaving: true } : null)), TOAST_MS - LEAVE_MS);
      schedule(() => setToast(null), TOAST_MS);
    },
    [clearTimers, schedule],
  );

  /** 用户点 ✕ —— 立刻开始退场，不等自动消失。 */
  const dismiss = useCallback(() => {
    clearTimers();
    setToast((current) => (current ? { ...current, leaving: true } : null));
    schedule(() => setToast(null), LEAVE_MS);
  }, [clearTimers, schedule]);

  return { toast, notify, dismiss };
}
