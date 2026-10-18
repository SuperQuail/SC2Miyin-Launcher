import { useCallback, useEffect, useRef, useState } from "react";

/** 提示的语气。 */
export type ToastKind = "info" | "success" | "warning" | "error";

/** 提示上的可选动作按钮。**带动作的那条不自动消失**（Material 3 也是这么定的）。 */
export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  /** 队列内唯一的号 —— ✕ 靠它精确关闭某一条 */
  id: number;
  kind: ToastKind;
  message: string;
  /** 正在退场：外壳先把它置 true，让退出动画放完再卸载 */
  leaving?: boolean;
  action?: ToastAction;
}

/** 说话的口子：别的组件只认它，不认外壳是谁。 */
export type Notify = (kind: ToastKind, message: string) => void;

/** 一条提示挂多久。 */
export const TOAST_MS = 4200;
/** 同时最多显示几条：再来的**挤掉最旧的**（antd 的 maxCount 也是这个语义 —— 最新的那条最重要）。 */
export const TOAST_MAX = 3;
/** 退场动画时长 —— 和 `ui/toast.css` 里的 toast-out 对齐。 */
const LEAVE_MS = 200;
/**
 * 走表的精度。每条自己挂一串 `setTimeout` 也能做，但那样悬停暂停要逐个清、逐个重排；
 * 用**绝对时刻 + 一个定时器扫**就只平移时刻，暂停只是把 now 冻住。
 */
const TICK_MS = 100;

/** 队列里的条目：在 `Toast` 之上多两个绝对时刻。 */
interface Entry extends Toast {
  /** 什么时候开始退场；null = 不自动消失（带动作按钮的那种） */
  expiresAt: number | null;
  /** 什么时候真正卸载 */
  removeAt: number | null;
}

export interface NotifyApi {
  /** 提示队列，**最新的在前**（离屏幕右下角最近的那条）。 */
  toasts: Toast[];
  notify: Notify;
  /** 关掉某一条（✕ 按钮用）—— 精确到 id，不是"关掉当前这条"。 */
  dismiss: (id: number) => void;
  /** 鼠标压在这一叠上：整叠暂停倒计时。 */
  pause: () => void;
  /** 移开：接着走，剩余时间不丢。 */
  resume: () => void;
}

/**
 * 右下角那一叠提示。
 *
 * 状态跟着外壳走（不散在模块里）：提示只由外壳渲染，别的组件拿 `notify` 说话。
 * 挂载点在 `AppShell`、**不在 `.view` 里** —— 切页不能把没读完的提示弄丢。
 *
 * **两段计时**：到 `TOAST_MS` 先标记 leaving（让退出动画开始），再过 `LEAVE_MS` 才真正卸载 ——
 * 直接删掉的话会硬切，没有退场。
 */
export function useNotify(): NotifyApi {
  const [entries, setEntries] = useState<Entry[]>([]);
  const nextId = useRef(1);
  /** 暂停的时刻；null = 没暂停。悬停期间倒计时冻住 */
  const pausedAt = useRef<number | null>(null);

  const notify = useCallback<Notify>((kind, message) => {
    const entry: Entry = {
      id: nextId.current++,
      kind,
      message,
      leaving: false,
      expiresAt: Date.now() + TOAST_MS,
      removeAt: null,
    };
    setEntries((current) => [entry, ...current].slice(0, TOAST_MAX));
  }, []);

  const dismiss = useCallback((id: number) => {
    setEntries((current) =>
      current.map((entry) =>
        entry.id === id && !entry.leaving
          ? { ...entry, leaving: true, expiresAt: null, removeAt: Date.now() + LEAVE_MS }
          : entry,
      ),
    );
  }, []);

  // 连着进来两个 enter 事件时别把暂停的起点往后推 —— 那会白送一段时间
  const pause = useCallback(() => {
    pausedAt.current ??= Date.now();
  }, []);

  const resume = useCallback(() => {
    const at = pausedAt.current;
    if (at === null) return;
    pausedAt.current = null;
    const delta = Date.now() - at;
    setEntries((current) =>
      current.map((entry) => ({
        ...entry,
        expiresAt: entry.expiresAt === null ? null : entry.expiresAt + delta,
        removeAt: entry.removeAt === null ? null : entry.removeAt + delta,
      })),
    );
  }, []);

  useEffect(() => {
    if (entries.length === 0) return;
    const timer = setInterval(() => {
      if (pausedAt.current !== null) return;
      const now = Date.now();
      setEntries((current) => {
        let changed = false;
        const next: Entry[] = [];
        for (const entry of current) {
          if (entry.removeAt !== null && now >= entry.removeAt) {
            changed = true;
            continue;
          }
          if (!entry.leaving && entry.expiresAt !== null && now >= entry.expiresAt) {
            changed = true;
            next.push({ ...entry, leaving: true, removeAt: entry.expiresAt + LEAVE_MS });
            continue;
          }
          next.push(entry);
        }
        return changed ? next : current;
      });
    }, TICK_MS);
    return () => clearInterval(timer);
  }, [entries.length]);

  return { toasts: entries, notify, dismiss, pause, resume };
}
