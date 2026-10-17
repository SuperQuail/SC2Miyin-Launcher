import type { CSSProperties } from "react";

import "./toast.css";

/** 提示的语气。和外壳里的 `Notify` 是同一套词。 */
export type ToastKind = "info" | "success" | "warning" | "error";

export type ToastProps = {
  kind: ToastKind;
  message: string;
  /** 正在退场：外壳在卸载前先把它置 true，让退出动画放完 */
  leaving?: boolean;
  /** 自动消失还剩多久（毫秒）—— 倒计时条的时长从这里来 */
  lifeMs?: number;
  onClose?: () => void;
};

const ICONS: Record<ToastKind, React.ReactNode> = {
  success: (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M20 6 9 17l-5-5" />
    </svg>
  ),
  error: (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M18 6 6 18M6 6l12 12" />
    </svg>
  ),
  warning: (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 9v5M12 17.5v.5" />
      <path d="M10.3 3.9 2.6 17a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
    </svg>
  ),
  info: (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 11v6M12 7.5v.5" />
      <circle cx="12" cy="12" r="9" />
    </svg>
  ),
};

/**
 * 右下角那条提示。
 *
 * 三件事都在这里：**语气色**（左边缘一条 3px 竖条 + 图标）、**入场/退场动画**
 * （从右侧滑入、下滑淡出，`prefers-reduced-motion` 下自动关掉）、
 * **倒计时条**（和外壳的自动消失时长同源，让人知道还剩多久）。
 *
 * 只摆一条 —— 外壳同一时刻只有一条提示（多条的队列等真需要了再说）。
 */
export function Toast({ kind, message, leaving = false, lifeMs = 4200, onClose }: ToastProps) {
  return (
    <div
      className={"toast toast--" + kind + (leaving ? " toast--leaving" : "")}
      style={{ "--toast-life": lifeMs + "ms" } as CSSProperties}
      role="status"
      aria-live="polite"
    >
      <span className="toast__icon">{ICONS[kind]}</span>
      <p className="toast__message">{message}</p>
      {onClose && (
        <button className="toast__close" type="button" aria-label="关掉这条提示" onClick={onClose}>
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      )}
      <span className="toast__life" />
    </div>
  );
}
