import { useDemo, type LabTone } from "./types";
import { ToneIcon } from "./icons";
import { ToastLayer } from "./ToastLayer";
import "./cand-e-card.css";

/**
 * 候选 E · 右下角卡片（生产那个位置，配一版"语气色卡"长相）。
 *
 * 位置**照抄生产那个 Toast**（右下角），但长相不走反色：卡片跟随主题、
 * 语气色给**图标徽章**，动作按钮是实心主色，关掉靠 ✕。
 * `timer` 用来演"要不要那条倒计时条"—— 生产版有，这里一个有一个没有，好对比。
 */

export type CandEProps = {
  tone: LabTone;
  title: string;
  message: string;
  action?: { label: string; onClick: () => void };
  /** 底部那条自动消失倒计时条：生产版有，故意留一个开关好对比 */
  timer?: boolean;
  lifeMs?: number;
  /** 进房间就自动放这一条（`?toast=` 指定）；false 就一直躺着等按钮 */
  active: boolean;
  reset: number;
};

export function CandECard({ tone, title, message, action, timer = false, lifeMs = 8000, active, reset }: CandEProps) {
  const demo = useDemo(active, reset, lifeMs, 220);
  if (demo.phase === "gone") return null;

  return (
    <ToastLayer>
      <div
      className={"cornercard cornercard--" + tone + (demo.phase === "leaving" ? " cornercard--out" : "")}
      style={timer ? { ["--cornercard-life" as string]: lifeMs + "ms" } : undefined}
      role="status"
      aria-live="polite"
      data-phase={demo.phase}
      {...demo.hover}
    >
      <span className="cornercard__badge">
        <ToneIcon tone={tone} className="cornercard__icon" />
      </span>
      <div className="cornercard__body">
        <p className="cornercard__title">{title}</p>
        <p className="cornercard__text">{message}</p>
        {action && (
          <button className="cornercard__action" type="button" onClick={action.onClick}>
            {action.label}
          </button>
        )}
      </div>
      <button className="cornercard__close" type="button" aria-label="关掉这条提示">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
      {timer && <span className="cornercard__life" />}
      </div>
    </ToastLayer>
  );
}