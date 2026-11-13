import { useMemo, useRef, useState, type ChangeEvent } from "react";
import { Wheel } from "react-custom-roulette";

import { Button } from "../../ui";
import { SAMPLE_ENTRIES, pickIndex, toWheelData, type WheelEntry } from "./wheelData";
import { readPalette } from "./wheelPalette";
import "./wheel.css";

/**
 * 转速：库的总时长 = 11350ms × spinDuration。
 * 默认 1.0 要 11 秒多，太长；0.5 约 5.7 秒，还带减速收尾。
 */
const SPIN_DURATION = 0.5;
/** 「减少动态效果」时用库允许的最小系数：约 0.11 秒。 */
const REDUCED_SPIN_DURATION = 0.01;

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * 技术验证页：验证 react-custom-roulette 的接入。
 *
 * 本轮**不读战役库、不启用、不启动游戏** —— 抽奖池是示例数据，
 * 只验证渲染、旋转、结果、主题与 DPI 这些技术点。接真实数据见 docs/design/wheel.md。
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
  const [prizeNumber, setPrizeNumber] = useState(0);
  const [spinning, setSpinning] = useState(false);
  const [mustStartSpinning, setMustStartSpinning] = useState(false);
  const [result, setResult] = useState<WheelEntry | null>(null);
  const [pointer, setPointer] = useState<string | null>(pointerSrc ?? null);
  const [pointerError, setPointerError] = useState<string | null>(null);
  const pointerInput = useRef<HTMLInputElement>(null);

  // theme 只是依赖：令牌值在主题切换后才会变。
  const palette = useMemo(() => readPalette(), [theme]);
  const data = useMemo(() => toWheelData(entries, palette), [entries, palette]);
  const canSpin = entries.length >= 2 && !spinning;

  function spin(): void {
    if (!canSpin) return;
    setPrizeNumber(pickIndex(random(), entries.length));
    setResult(null);
    setMustStartSpinning(true);
    setSpinning(true);
  }

  function handleStop(): void {
    setMustStartSpinning(false);
    setSpinning(false);
    setResult(entries[prizeNumber] ?? null);
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
            <p className="wheel-page__lede">转一下，从候选里抽一部。</p>
          </div>
          <span className="wheel-page__badge">技术验证 · 示例数据</span>
        </header>

        <p className="wheel-page__note">
          本轮只验证转盘的技术栈（react-custom-roulette）：不读战役库，也不会启用或启动游戏。
        </p>

        {entries.length < 2 ? (
          <p className="wheel-page__empty">至少要有两个候选才能转。</p>
        ) : (
          <div className="wheel-frame" role="group" aria-label="抽战役转盘">
            <div className="wheel-rotor">
              <Wheel
                mustStartSpinning={mustStartSpinning}
                prizeNumber={prizeNumber}
                data={data}
                onStopSpinning={handleStop}
                backgroundColors={palette.background}
                textColors={[palette.text]}
                outerBorderColor={palette.border}
                outerBorderWidth={2}
                radiusLineColor={palette.border}
                radiusLineWidth={2}
                innerRadius={8}
                innerBorderColor={palette.border}
                innerBorderWidth={2}
                fontFamily="sans-serif"
                fontSize={14}
                fontWeight={600}
                textDistance={70}
                perpendicularText
                pointerProps={{ style: { display: "none" } }}
                disableInitialAnimation
                spinDuration={prefersReducedMotion() ? REDUCED_SPIN_DURATION : SPIN_DURATION}
              />
            </div>
            <div className="wheel-needle" aria-hidden="true">
              {pointer ? (
                <img className="wheel-needle__img" src={pointer} alt="" draggable={false} />
              ) : (
                <svg viewBox="0 0 100 100" focusable="false">
                  <path className="wheel-needle__arrow" d="M50 22 L56.5 50 L43.5 50 Z" />
                  <circle className="wheel-needle__hub" cx="50" cy="50" r="7.5" />
                  <circle className="wheel-needle__pin" cx="50" cy="50" r="2.8" />
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
            {spinning ? "转动中…" : result ? "再抽一次" : "开始抽"}
          </Button>
        </div>

        <div className="wheel-result" role="status" aria-live="polite">
          {result ? (
            <>
              <span className="wheel-result__label">抽到</span>
              <strong className="wheel-result__name">{result.label}</strong>
            </>
          ) : (
            <span className="wheel-result__hint">{spinning ? "正在转…" : "还没抽"}</span>
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
