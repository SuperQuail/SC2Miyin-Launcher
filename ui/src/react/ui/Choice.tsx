import type { ReactNode } from "react";

import "./choice.css";

export type ChoiceOption<T extends string> = {
  value: T;
  label: ReactNode;
  /** 每个选项下面的一行小字（可选）。 */
  hint?: string;
};

/**
 * 二选一 / 多选一（React）。
 *
 * 收的是「覆盖更新 / 重命名后导入」这种**必须选一个**的场合 —— 以前是拿 Button 的
 * tonal/outline 拼出来的，视觉上看不出"这是单选"，键盘也得一路 Tab 过去。
 *
 * 结构就是**原生 radio**（无障碍与键盘行为白给），样式才做成段落按钮的样子。
 */
export function Choice<T extends string>({
  label,
  value,
  options,
  onChange,
  name,
  hint,
  disabled = false,
}: {
  label?: ReactNode;
  value: T;
  options: ChoiceOption<T>[];
  onChange: (value: T) => void;
  /** 原生 radio 的 name；不给就按 options 生成一个稳定的。 */
  name?: string;
  hint?: string;
  disabled?: boolean;
}) {
  const group = name ?? "choice-" + options.map((option) => option.value).join("-");
  return (
    <fieldset className="choice" disabled={disabled}>
      {label !== undefined && <legend className="choice__label">{label}</legend>}
      <div className="choice__options">
        {options.map((option) => (
          <label
            key={option.value}
            className={
              "choice__option" + (value === option.value ? " choice__option--on" : "") + (disabled ? " is-disabled" : "")
            }
          >
            <input
              type="radio"
              className="choice__radio"
              name={group}
              value={option.value}
              checked={value === option.value}
              onChange={() => onChange(option.value)}
              disabled={disabled}
            />
            <span className="choice__text">{option.label}</span>
            {option.hint && <span className="choice__hint">{option.hint}</span>}
          </label>
        ))}
      </div>
      {hint && <em className="choice__foot">{hint}</em>}
    </fieldset>
  );
}
