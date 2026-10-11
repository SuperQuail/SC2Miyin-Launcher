import type { ReactNode } from "react";

/**
 * 按钮（React）。
 *
 * 和 Vue 侧那份 `ui/Button.vue` 是同一件事：把 100 多处手写的
 * `<button class="btn btn-xxx">` 收成一层。样式仍来自全局的 base.css ——
 * 迁移期间两边观感必须一致，所以这里**不碰样式**，只收结构。
 */
export type ButtonVariant = "primary" | "tonal" | "text" | "outline" | "danger";

export function Button({
  variant = "text",
  size = "md",
  disabled = false,
  title,
  type = "button",
  onClick,
  children,
}: {
  variant?: ButtonVariant;
  size?: "md" | "lg";
  disabled?: boolean;
  title?: string;
  type?: "button" | "submit";
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void;
  children: ReactNode;
}) {
  const className = ["btn", `btn-${variant}`, size === "lg" ? "btn-lg" : ""].filter(Boolean).join(" ");
  return (
    <button className={className} type={type} disabled={disabled} title={title} onClick={onClick}>
      {children}
    </button>
  );
}
