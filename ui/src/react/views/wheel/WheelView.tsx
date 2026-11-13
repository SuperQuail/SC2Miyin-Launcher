import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";

import { Button } from "../../ui";
import { WHEEL_SIZE, WheelDisc } from "./WheelDisc";
import { SAMPLE_ENTRIES, pickIndex, targetRotation, type WheelEntry } from "./wheelData";
import { readPalette } from "./wheelPalette";
import "./wheel.css";

/** 指针转一圈的时长；正常 4 秒，减少动态效果时立即到。 */
const SPIN_MS = 4000;
/** 转完之后再留一点余量给 CSS transition 收尾。 */
const REST_MS = 60;

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * 技术验证页：轮盘是自绘的静止 SVG，**转的是圆心指针**。
 *
 * 本轮**不读战役库、不启用、不启动游戏** —— 抽奖池是示例数据。
 */
export function WheelView({
  entries = SAMPLE_ENTRIES,
  theme = "light",
  random = Math.random,
  pointerSrc,
}: {
  entries?: readonly WheelEntry[];
  /** 主题变化后要重新读令牌（库画在 canvas 上，不会自动跟 CSS 变量）。 */
  theme?: "light" | "dark";
  /** 测试注入用。 */
  random?: () => number;
  /** 自定义圆心指针图片；不传用内置 SVG。页面上的「换指针图片…」也会写这里。 */
  pointerSrc?: string;
} = {}) {
  const [spinning, setSpinning] = useState(false);
  const [rotation, setRotation] = useState(0);
  const [result, setResult] = useState<WheelEntry | null>(null);
  const [pointer, setPointer] = useState<string | null>(pointerSrc ?? null);
  const [pointerError, setPointerError] = useState<string | null>(null);
  const pointerInput = useRef<HTMLInputElement>(null);
  const winner = useRef(0);
  const stopTimer = useRef<number | null>(null);

  // theme 只是依赖：令牌值在主题切换后才会变；轮盘实例重建时把指针也归位
  const palette = useMemo(() => readPalette(), [theme]);
  const reducedMotion = prefersReducedMotion();
  const duration = reducedMotion ? 0 : SPIN_MS;
  const canSpin = entries.length >= 2 && !spinning;

  useEffect(() => {
    setSpinning(false);
    setResult(null);
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
    winner.current = next;
    setResult(null);
    setSpinning(true);
    setRotation((current) => targetRotation(current, next, entries.length));

    if (stopTimer.current !== null) window.clearTimeout(stopTimer.current);
    stopTimer.current = window.setTimeout(() => {
      setSpinning(false);
      setResult(entries[next] ?? null);
      // 归一化到一圈内，避免角度越滚越大；此时 transition 已关，视觉不变
      setRotation((current) => ((current % 360) + 360) % 360);
    }, duration + REST_MS);
  }

  /** 选本地图片当指针：读成 data URL，只活在当前页面里（demo 不落盘）。 */
  function onPointerFile(event: ChangeEvent<HTMLInputElement>): void {
    const file = event.target.files?.[0];
    // 同一张图再选一次也要能触发 change
    event.target.value = "";
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) {
      setPointerError("图片最大 2 MB");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === "string") {
        setPointer(reader.result);
        setPointerError(null);
      }
    };
    reader.onerror = () => setPointerError("读不出这张图片");
    reader.readAsDataURL(file);
  }

  return (
    <div className="wheel-page">
      <div className="page">
        <header className="wheel-page__head">
          <div>
            <h2 className="wheel-page__title">今天玩什么？</h2>
            <p className="wheel-page__lede">指针转一圈，指到哪部就玩哪部。</p>
          </div>
          <span className="wheel-page__badge">技术验证 · 示例数据</span>
        </header>

        <p className="wheel-page__note">
          轮盘由 SVG 自绘、保持静止；旋转的是圆心指针。抽奖不依赖任何转盘库。
        </p>

        {entries.length < 2 ? (
          <p className="wheel-page__empty">至少要有两个候选才能转。</p>
        ) : (
          <div className="wheel-stage" style={{ width: WHEEL_SIZE, height: WHEEL_SIZE }}>
            <WheelDisc entries={entries} palette={palette} />
            <div
              className="wheel-pointer"
              aria-hidden="true"
              style={{
                transform: "rotate(" + rotation + "deg)",
                transition: spinning
                  ? "transform " + duration + "ms cubic-bezier(0.15, 0.85, 0.15, 1)"
                  : "none",
              }}
            >
              {pointer ? (
                <img className="wheel-pointer__img" src={pointer} alt="" draggable={false} />
              ) : (
                <svg viewBox="0 0 100 100" focusable="false">
                  <path className="wheel-pointer__arrow" d="M50 22 L56.5 50 L43.5 50 Z" />
                  <circle className="wheel-pointer__hub" cx="50" cy="50" r="7.5" />
                  <circle className="wheel-pointer__pin" cx="50" cy="50" r="2.8" />
                </svg>
              )}
            </div>
          </div>
        )}

        <div className="wheel-page__pointer">
          <Button
            variant="text"
            title="图片以轮盘中心为转轴、朝上；建议正方形、透明背景"
            onClick={() => pointerInput.current?.click()}
          >
            {pointer ? "换一张指针图片…" : "换指针图片…"}
          </Button>
          {pointer && (
            <Button
              variant="text"
              onClick={() => {
                setPointer(null);
                setPointerError(null);
              }}
            >
              恢复默认指针
            </Button>
          )}
          <input
            ref={pointerInput}
            className="wheel-page__pointer-input"
            type="file"
            accept="image/png,image/svg+xml,image/webp,image/jpeg"
            onChange={onPointerFile}
          />
          {pointerError && <span className="wheel-page__pointer-error">{pointerError}</span>}
        </div>

        <div className="wheel-page__actions">
          <Button variant="primary" size="lg" loading={spinning} onClick={spin} disabled={!canSpin}>
            {spinning ? "转动中…" : result ? "再转一次" : "开始转"}
          </Button>
        </div>

        <div className="wheel-result" role="status" aria-live="polite">
          {result ? (
            <>
              <span className="wheel-result__label">指针指向</span>
              <strong className="wheel-result__name">{result.label}</strong>
            </>
          ) : (
            <span className="wheel-result__hint">{spinning ? "正在转…" : "还没转"}</span>
          )}
        </div>

        <section className="wheel-candidates" aria-label="候选列表">
          <h3 className="wheel-candidates__title">候选（{entries.length}）</h3>
          <ul className="wheel-candidates__list">
            {entries.map((entry) => (
              <li
                key={entry.id}
                className={
                  "wheel-candidates__item" +
                  (entry.id === result?.id ? " wheel-candidates__item--on" : "")
                }
              >
                {entry.label}
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
