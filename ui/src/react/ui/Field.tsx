import type { ReactNode } from "react";

/**
 * 表单一行：标签 + 控件（React）。
 *
 * 原先 **49 处**手写 `.field`，样式还在三个文件里各定义了一遍（类名都不一致）。
 * 这里样式跟着控件走；页面不再写。
 */
export function Field({
  label,
  wide = false,
  invalid = false,
  error,
  hint,
  children,
}: {
  label?: ReactNode;
  /** 占满整行。 */
  wide?: boolean;
  invalid?: boolean;
  error?: string;
  hint?: string;
  children: ReactNode;
}) {
  const className = ["field", wide ? "field--wide" : "", invalid ? "field--bad" : ""]
    .filter(Boolean)
    .join(" ");
  return (
    <label className={className}>
      {label !== undefined && <span className="field__label">{label}</span>}
      {children}
      {error ? <em className="field__error">{error}</em> : hint ? <em className="field__hint">{hint}</em> : null}
    </label>
  );
}
