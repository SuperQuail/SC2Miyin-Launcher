import { useEffect, useMemo, useRef, useState } from "react";

import { Button } from "../../ui";
import { WHEEL_SIZE, WheelDisc } from "./WheelDisc";
import { SAMPLE_ENTRIES, pickIndex, targetRotation, type WheelEntry } from "./wheelData";
import { readPalette } from "./wheelPalette";
import "./wheel.css";

/** 指针转一圈的时长；正常 4 秒，减少动态效果时立即到。 */
const SPIN_MS = 4000;
/** 转完之后再留一点余量给 CSS transition 收尾。 */
const REST_MS = 60;
/** 默认指针图：大主教头像；图片底部朝外，转轴在从下往上 80% 处。 */
const DEFAULT_POINTER = "/wheel/dajiaozhu-pointer.png";
/** 图片默认朝下；目标角度是「朝上方向」，所以整体要额外转 -180°。 */
const POINTER_ANGLE_OFFSET = -180;

/** 转盘页的两个来源；具体内容页暂不区分。 */
type WheelScope = "owned" | "database";

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function WheelView({
  entries = SAMPLE_ENTRIES,
  theme = "light",
  random = Math.random,
}: {
  entries?: readonly WheelEntry[];
  /** 主题变化后要重新读令牌。 */
  theme?: "light" | "dark";
  /** 测试注入用。 */
  random?: () => number;
} = {}) {
  const [scope, setScope] = useState<WheelScope>("owned");
  const [spinning, setSpinning] = useState(false);
  const [rotation, setRotation] = useState(0);
  const stopTimer = useRef<number | null>(null);

  const palette = useMemo(() => readPalette(), [theme]);
  const reducedMotion = prefersReducedMotion();
  const duration = reducedMotion ? 0 : SPIN_MS;
  const canSpin = entries.length >= 2 && !spinning;

  useEffect(() => {
    setSpinning(false);
    setRotation(0);
  }, [theme]);

  useEffect(() => {
    return () => {
      if (stopTimer.current !== null) window.clearTimeout(stopTimer.current);
    };
  }, []);

  function spin(): void {
    if (!canSpin) return;
    const next = pickIndex(random(), entries.length);
    setSpinning(true);
    setRotation((current) => targetRotation(current, next, entries.length));

    if (stopTimer.current !== null) window.clearTimeout(stopTimer.current);
    stopTimer.current = window.setTimeout(() => {
      setSpinning(false);
      setRotation((current) => ((current % 360) + 360) % 360);
    }, duration + REST_MS);
  }

  return (
    <div className="wheel-page">
      <div className="page">
        <h2 className="wheel-page__title">战役转盘</h2>

        <div className="wheel-subnav" role="tablist" aria-label="战役来源">
          <button
            type="button"
            role="tab"
            aria-selected={scope === "owned"}
            className={
              "wheel-subnav__item" + (scope === "owned" ? " wheel-subnav__item--active" : "")
            }
            onClick={() => setScope("owned")}
          >
            已有战役
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={scope === "database"}
            className={
              "wheel-subnav__item" + (scope === "database" ? " wheel-subnav__item--active" : "")
            }
            onClick={() => setScope("database")}
          >
            战役数据库
          </button>
        </div>

        <div className="wheel-stage" style={{ width: WHEEL_SIZE, height: WHEEL_SIZE }}>
          <WheelDisc entries={entries} palette={palette} />
          <div
            className="wheel-pointer"
            aria-hidden="true"
            style={{
              transform: "rotate(" + (rotation + POINTER_ANGLE_OFFSET) + "deg)",
              transition: spinning
                ? "transform " + duration + "ms cubic-bezier(0.15, 0.85, 0.15, 1)"
                : "none",
            }}
          >
            <img className="wheel-pointer__img" src={DEFAULT_POINTER} alt="" draggable={false} />
          </div>
        </div>

        <div className="wheel-page__actions">
          <Button variant="primary" size="lg" loading={spinning} onClick={spin} disabled={!canSpin}>
            {spinning ? "抽取中…" : "抽取"}
          </Button>
        </div>
      </div>
    </div>
  );
}
