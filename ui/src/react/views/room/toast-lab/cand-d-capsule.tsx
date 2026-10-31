import { useDemo, type LabTone } from "./types";
import { ToneIcon } from "./icons";
import { ToastLayer } from "./ToastLayer";
import "./cand-d-capsule.css";

/**
 * 候选 D · Ant Design message 胶囊（底部居中、一行字、无按钮无 ✕）。
 *
 * 抄的是 antd 的 message：**没有卡片**，就是一个**深色胶囊**浮在底部居中，
 * 只有"图标 + 一行字"，3 秒左右自己走，**不给动作按钮、也不给关闭按钮**。
 * 存在的意义是当**最简对照组** —— 如果只想要"别挡路"，它最省。
 */

export type CandDProps = {
  tone: LabTone;
  message: string;
  lifeMs?: number;
  /** 进房间就自动放这一条（`?toast=` 指定）；false 就一直躺着等按钮 */
  active: boolean;
  reset: number;
};

export function CandDCapsule({ tone, message, lifeMs = 6000, active, reset }: CandDProps) {
  const demo = useDemo(active, reset, lifeMs, 240);
  if (demo.phase === "gone") return null;

  return (
    <ToastLayer>
      <div
      className={"capcap capcap--" + tone + (demo.phase === "leaving" ? " capcap--out" : "")}
      role="status"
      aria-live="polite"
      data-phase={demo.phase}
      {...demo.hover}
    >
      <ToneIcon tone={tone} className="capcap__icon" />
      <span className="capcap__text">{message}</span>
      </div>
    </ToastLayer>
  );
}