import { useDemo, type LabTone } from "./types";
import { ToneIcon } from "./icons";
import { ToastLayer } from "./ToastLayer";
import "./cand-c-sonner.css";

/**
 * 候选 C · Sonner 式紧凑堆叠（**已采用**：生产版就是它，位置改成了右下角）。
 *
 * 抄的是 sonner（shadcn 的默认提示）与 macOS 通知中心的两件事：
 * 卡片**贴得紧**、**超过一条就往左上叠一层**并压暗缩小，
 * 最前面一条才完整可读、带动作按钮与 ✕。看的是"同时来两条"的样子。
 *
 * 这里按**正式版**摆（右下角），所以它和 `react/ui/Toast` 看起来是同一个东西 ——
 * 差别只在候选用假数据、正式版接的是真的 `notify` 队列。
 */

export type StackItem = {
  tone: LabTone;
  title: string;
  message?: string;
  action?: { label: string; onClick: () => void };
};

export type CandCProps = {
  items: StackItem[];
  lifeMs?: number;
  /** 进房间就自动放这一条（`?toast=` 指定）；false 就一直躺着等按钮 */
  active: boolean;
  reset: number;
};

export function CandCSonnerStack({ items, lifeMs = 9000, active, reset }: CandCProps) {
  const demo = useDemo(active, reset, lifeMs, 240);

  return (
    <ToastLayer>
      {/* 没点名时整叠都不渲染 —— 这一版有好几张卡，hide 要落在容器上 */}
      <div
        className="sonnerstack"
        aria-live="polite"
        data-phase={demo.phase}
        style={{ display: demo.phase === "gone" ? "none" : undefined }}
      >
      {/* items[0] 是最新的一条（离屏幕边最近），后面的往后叠 */}
      {items.map((item, index) => (
        <div
          key={item.title}
          className={"sonner" + (demo.phase === "leaving" ? " sonner--out" : "")}
          style={{ zIndex: items.length - index }}
          role="status"
          {...demo.hover}
        >
          <ToneIcon tone={item.tone} className="sonner__icon" />
          <div className="sonner__body">
            <p className="sonner__title">{item.title}</p>
            {index === 0 && item.message && <p className="sonner__text">{item.message}</p>}
            {index === 0 && item.action && (
              <div className="sonner__actions">
                <button className="sonner__action" type="button" onClick={item.action.onClick}>
                  {item.action.label}
                </button>
              </div>
            )}
          </div>
          {index === 0 && (
            <button className="sonner__close" type="button" aria-label="关掉这条提示">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M18 6 6 18M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      ))}
      </div>
    </ToastLayer>
  );
}