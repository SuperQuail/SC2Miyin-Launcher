import type { ReactNode } from "react";

import "./checkbox.css";

/**
 * 复选框。
 *
 * 结构是原生 checkbox（键盘与读屏行为白给），外观按控件层令牌画。
 * 页面不直接写 <input type="checkbox">；开关类设置用它。
 */
export function Checkbox({
  checked,
  onChange,
  children,
  name,
  disabled = false,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  children: ReactNode;
  name?: string;
  disabled?: boolean;
}) {
  return (
    <label className={"checkbox" + (disabled ? " is-disabled" : "")}>
      <input
        className="checkbox__input"
        type="checkbox"
        name={name}
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        disabled={disabled}
      />
      <span className="checkbox__box" aria-hidden="true" />
      <span className="checkbox__label">{children}</span>
    </label>
  );
}
