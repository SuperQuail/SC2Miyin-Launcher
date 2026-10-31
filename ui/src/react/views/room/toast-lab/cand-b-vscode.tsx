import { useDemo, type LabTone } from "./types";
import { ToneIcon } from "./icons";
import { ToastLayer } from "./ToastLayer";
import "./cand-b-vscode.css";

/**
 * 候选 B · VS Code 通知（右上角卡片、带标题、右侧动作按钮 + ✕）。
 *
 * 抄的是 VS Code 的 notification：位置**右上角**，滑入自右侧；卡片式（跟随主题，
 * 不是反色），左侧窄色条只在 warning/error 时出现；标题一行 + 正文若干行；
 * 按钮排在**底部右边**，✕ 常驻。长文案在这里看得最清楚（正文会折行，到 3 行截断）。
 */

export type Action = { label: string; onClick: () => void };

export type CandBProps = {
  tone: LabTone;
  title: string;
  message: string;
  action?: Action;
  lifeMs?: number;
  /** 进房间就自动放这一条（`?toast=` 指定）；false 就一直躺着等按钮 */
  active: boolean;
  reset: number;
};

export function CandBNoticeCard({ tone, title, message, action, lifeMs = 8000, active, reset }: CandBProps) {
  const demo = useDemo(active, reset, lifeMs, 200);
  if (demo.phase === "gone") return null;

  return (
    <ToastLayer>
      <div
      className={"vscnote vscnote--" + tone + (demo.phase === "leaving" ? " vscnote--out" : "")}
      role="status"
      aria-live="polite"
      data-phase={demo.phase}
      {...demo.hover}
    >
      <ToneIcon tone={tone} className="vscnote__icon" />
      <div className="vscnote__body">
        <p className="vscnote__title">{title}</p>
        <p className="vscnote__text">{message}</p>
        {action && (
          <div className="vscnote__actions">
            <button className="vscnote__action" type="button" onClick={action.onClick}>
              {action.label}
            </button>
          </div>
        )}
      </div>
      <button className="vscnote__close" type="button" aria-label="关掉这条提示">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
      </div>
    </ToastLayer>
  );
}