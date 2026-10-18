import type { ReactNode } from "react";

import "./toast.css";

/** 提示的语气。和外壳里的 `Notify` 是同一套词。 */
export type ToastKind = "info" | "success" | "warning" | "error";

/** 提示上的可选动作按钮。带动作的那条不自动消失（M3 的规矩）。 */
export type ToastAction = { label: string; onClick: () => void };

export type ToastItem = {
  /** 队列内唯一的号 —— ✕ 靠它精确关掉那一条 */
  id: number;
  kind: ToastKind;
  message: string;
  /** 正在退场：外壳先把它置 true，让退出动画放完再卸载 */
  leaving?: boolean;
  action?: ToastAction;
};

export type ToastProps = {
  /** 提示队列，**最新的在前**（离右下角最近的那条）。空数组就什么都不渲染。 */
  toasts: ToastItem[];
  /** 点某一条的 ✕：只关那一条 */
  onClose: (id: number) => void;
  /** 鼠标压在这一叠上：整叠暂停自动消失 */
  onPause?: () => void;
  onResume?: () => void;
};

const ICONS: Record<ToastKind, ReactNode> = {
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
 * 右下角那一叠提示（**正式版**）。样子取自开发者房间的候选 C，
 * 位置按用户的意见从 C 的左下角改到了右下角。
 *
 * 三件事都在这里：**语气**（收在图标颜色上，不整块染色）、**堆叠**
 * （最新那条贴在右下角、完整可读；旧的往左上压暗缩小、只露一行）、
 * **进场/退场动画**（`prefers-reduced-motion` 下自动关掉）。
 *
 * **只有最前那条有 ✕ 和动作按钮** —— 后面那几条只是"还有别的提示"的提示，摸不到。
 * 这里**没有倒计时条**：用户嫌原来那条奇怪，C 也没有。还剩多久靠"悬停暂停 + 到点自己走"，
 * 计时在 `state/useNotify`。
 *
 * 挂在 `AppShell` 上、**不在 `.view` 里** —— `.view` 带着 `animation: view-in ... both`，
 * 计算值是个非 none 的 transform，会成为 `position: fixed` 的包含块，提示会掉到屏幕外
 * （踩过一次）；切页时也该留住没读完的提示。
 */
export function Toast({ toasts, onClose, onPause, onResume }: ToastProps) {
  if (toasts.length === 0) return null;
  const front = toasts[0].id;

  return (
    <div className="toast-stack" aria-live="polite" onMouseEnter={onPause} onMouseLeave={onResume}>
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={"toast toast--" + toast.kind + (toast.leaving ? " toast--leaving" : "")}
          role="status"
        >
          <span className="toast__icon">{ICONS[toast.kind]}</span>
          <p className="toast__message">{toast.message}</p>
          {toast.id === front && toast.action && (
            <button className="toast__action" type="button" onClick={toast.action.onClick}>
              {toast.action.label}
            </button>
          )}
          {toast.id === front && (
            <button
              className="toast__close"
              type="button"
              aria-label="关掉这条提示"
              onClick={() => onClose(toast.id)}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M18 6 6 18M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
