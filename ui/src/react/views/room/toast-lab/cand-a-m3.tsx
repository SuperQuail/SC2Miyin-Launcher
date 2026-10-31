import { useDemo, type LabTone } from "./types";
import { ToneIcon } from "./icons";
import { ToastLayer } from "./ToastLayer";
import "./cand-a-m3.css";

/**
 * 候选 A · Material 3 Snackbar（底部居中、反色深底、一条动作按钮）。
 *
 * 抄的是 m3.material.io 的 Snackbar：**位置在底部居中**（不是右下角），
 * 背景是"反色"（浅色主题下深底浅字，深色主题下浅底深字），圆角很小，
 * 右侧一条动作按钮，**没有 ✕**（点空白/等它自己走）。
 */

export type Action = { label: string; onClick: () => void };

export type CandAProps = {
  tone: LabTone;
  message: string;
  action?: Action;
  /** 演示时长：默认 7 秒，够看清入场也够等着看退场 */
  lifeMs?: number;
  /** 进房间就自动放这一条（`?toast=` 指定）；false 就一直躺着等按钮 */
  active: boolean;
  reset: number;
};

export function CandASnackbar({ tone, message, action, lifeMs = 7000, active, reset }: CandAProps) {
  const demo = useDemo(active, reset, lifeMs, 220);
  if (demo.phase === "gone") return null;

  return (
    <ToastLayer>
      <div
      className={"m3snack m3snack--" + tone + (demo.phase === "leaving" ? " m3snack--out" : "")}
      role="status"
      aria-live="polite"
      data-phase={demo.phase}
      {...demo.hover}
    >
      <ToneIcon tone={tone} className="m3snack__icon" />
      <p className="m3snack__text">{message}</p>
      {action && (
        <button className="m3snack__action" type="button" onClick={action.onClick}>
          {action.label}
        </button>
      )}
      </div>
    </ToastLayer>
  );
}