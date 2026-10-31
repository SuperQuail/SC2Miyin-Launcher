import "./progress.css";

/**
 * 进度条（React）。
 *
 * 以前 `.progress` 只活在 settings.css 里、别处要用就再手搓一条 —— 现在它是个控件。
 * 语义走 role="progressbar"，读屏能念出当前进度。
 */
export function Progress({
  value,
  max = 100,
  label,
  detail,
}: {
  /** 当前进度；传 null 表示不确定（转圈那种），此时不画填充。 */
  value: number | null;
  max?: number;
  label?: string;
  /** 右下角的补充说明，比如「123 / 2331」。 */
  detail?: string;
}) {
  const safeMax = max > 0 ? max : 1;
  const clamped = value === null ? 0 : Math.min(Math.max(value, 0), safeMax);
  const percent = Math.round((clamped / safeMax) * 100);

  return (
    <div
      className={"progress" + (value === null ? " progress--indeterminate" : "")}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={safeMax}
      aria-valuenow={value === null ? undefined : clamped}
      aria-label={label}
    >
      {(label || detail) && (
        <div className="progress__head">
          {label && <span className="progress__label">{label}</span>}
          {detail && <span className="progress__detail">{detail}</span>}
        </div>
      )}
      <div className="progress__track">
        <div className="progress__bar" style={value === null ? undefined : { width: percent + "%" }} />
      </div>
    </div>
  );
}