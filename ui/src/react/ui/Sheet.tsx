import type { ReactNode } from "react";

/**
 * 遮罩弹窗（React）。
 *
 * 原先有 **16 处**手写的 `div.sheet > div.sheet__card > h3.sheet__title`。
 * 样式仍来自全局 base.css —— 迁移期两边观感必须一致（AGENTS §15.3 特意要求
 * `.sheet` 系列放全局，别写进某个组件的 scoped）。
 */
export function Sheet({
  title,
  onClose,
  dismissOnBackdrop = true,
  children,
  actions,
  className = "",
}: {
  title?: ReactNode;
  /** 点遮罩 / 点取消时调它。 */
  onClose?: () => void;
  /** 点遮罩关不关；默认关。原来不关的那些要显式传 false。 */
  dismissOnBackdrop?: boolean;
  children: ReactNode;
  /** 底部按钮那一排。 */
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className="sheet"
      onClick={(event) => {
        if (event.target === event.currentTarget && dismissOnBackdrop) onClose?.();
      }}
    >
      <div className={"sheet__card" + (className ? " " + className : "")}>
        {title !== undefined && <h3 className="sheet__title">{title}</h3>}
        {children}
        {actions && <div className="sheet__actions">{actions}</div>}
      </div>
    </div>
  );
}
