import { useCallback, useRef, useState } from "react";

/** 提示的语气。 */
export type ToastKind = "info" | "success" | "warning" | "error";

export interface Toast {
  kind: ToastKind;
  message: string;
}

/** 说话的口子：别的组件只认它，不认外壳是谁。 */
export type Notify = (kind: ToastKind, message: string) => void;

/** 一条提示挂多久（和原来一样）。 */
const TOAST_MS = 4200;

/**
 * 右下角那条提示。
 *
 * 状态跟着外壳走（不散在模块里）：提示只由外壳渲染，别的组件拿 `notify` 说话。
 */
export function useNotify(): { toast: Toast | null; notify: Notify } {
  const [toast, setToast] = useState<Toast | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const notify = useCallback<Notify>((kind, message) => {
    setToast({ kind, message });
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setToast(null), TOAST_MS);
  }, []);

  return { toast, notify };
}
