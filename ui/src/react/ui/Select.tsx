import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";

export interface SelectOption {
  value: string;
  label: string;
  /** 副标题（比如「原版」「2 个版本」这种说明）。 */
  hint?: string;
}

/**
 * 项目自己的下拉。
 *
 * **弹层挂在 document.body 上（portal）+ position: fixed** —— 不能放在触发器里面：
 * 它在滚动容器（比如组详情那份文件列表）里会被裁掉，容器一滚动位置也会错。
 * 坐标按触发器的屏幕矩形算，空间不够就往上翻。
 *
 * 只做用得上的：点开、选中、点外面关、Esc 关、上下键移动、Enter 选。
 */
export function Select({
  value,
  options,
  placeholder = "请选择",
  disabled = false,
  onChange,
  ariaLabel,
}: {
  value: string;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  onChange: (value: string) => void;
  ariaLabel?: string;
}) {
  const [open, setOpen] = useState(false);
  const [cursor, setCursor] = useState(0);
  const [box, setBox] = useState<{ left: number; top: number; width: number; up: boolean } | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const popupRef = useRef<HTMLUListElement | null>(null);
  const listId = useId();
  const current = options.find((item) => item.value === value);

  /** 按触发器的屏幕矩形定位；下面放不下就往上翻。 */
  const place = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    const up = spaceBelow < 260 && rect.top > spaceBelow;
    setBox({
      left: rect.left,
      top: up ? rect.top : rect.bottom + 4,
      width: rect.width,
      up,
    });
  }, []);

  useLayoutEffect(() => {
    if (open) place();
  }, [open, place]);

  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (triggerRef.current?.contains(target)) return;
      if (popupRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onMove = () => place();
    document.addEventListener("mousedown", onDown);
    // 滚动/缩放时跟着走：fixed 坐标是屏幕坐标，容器滚动它不会自己动
    window.addEventListener("scroll", onMove, true);
    window.addEventListener("resize", onMove);
    return () => {
      document.removeEventListener("mousedown", onDown);
      window.removeEventListener("scroll", onMove, true);
      window.removeEventListener("resize", onMove);
    };
  }, [open, place]);

  const choose = (next: string) => {
    onChange(next);
    setOpen(false);
  };

  const popup = open && box
    ? createPortal(
        <ul
          ref={popupRef}
          id={listId}
          role="listbox"
          style={{
            position: "fixed",
            left: box.left,
            top: box.top,
            minWidth: box.width,
            maxWidth: Math.min(420, window.innerWidth - box.left - 12),
            transform: box.up ? "translateY(-100%)" : undefined,
            zIndex: 70,
          }}
          className="m-0 max-h-64 list-none overflow-auto rounded-lg border border-[color-mix(in_srgb,var(--outline)_80%,transparent)] bg-surface p-1 shadow-lg"
        >
          {options.length === 0 && (
            <li className="px-3 py-2 text-[12px] text-muted">没有可选项</li>
          )}
          {options.map((option, index) => {
            const selected = option.value === value;
            return (
              <li key={option.value}>
                <button
                  type="button"
                  role="option"
                  aria-selected={selected}
                  onMouseEnter={() => setCursor(index)}
                  onClick={() => choose(option.value)}
                  className={[
                    "flex w-full flex-col items-start gap-0.5 rounded-md px-3 py-1.5 text-left text-[12.5px]",
                    index === cursor ? "bg-surface-2" : "",
                    selected ? "text-accent" : "text-on-surface",
                  ].join(" ")}
                >
                  <span className="w-full truncate">{option.label}</span>
                  {option.hint && <span className="text-[11px] text-muted">{option.hint}</span>}
                </button>
              </li>
            );
          })}
        </ul>,
        document.body,
      )
    : null;

  return (
    <div className="relative inline-block w-full text-left">
      <button
        ref={triggerRef}
        type="button"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={ariaLabel}
        onClick={() => {
          setOpen((was) => !was);
          setCursor(Math.max(0, options.findIndex((item) => item.value === value)));
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") setOpen(false);
          if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            if (!open) {
              setOpen(true);
              return;
            }
            const step = event.key === "ArrowDown" ? 1 : -1;
            setCursor((index) => (index + step + options.length) % Math.max(1, options.length));
          }
          if (event.key === "Enter" && open && options[cursor]) {
            event.preventDefault();
            choose(options[cursor].value);
          }
        }}
        className={[
          "flex w-full min-w-[9.5rem] items-center justify-between gap-2 rounded-md px-3 py-1.5 text-[12.5px]",
          "border border-[color-mix(in_srgb,var(--outline)_85%,transparent)] bg-surface-2 text-on-surface",
          "transition-colors hover:border-accent/60 disabled:cursor-not-allowed disabled:opacity-50",
          open ? "border-accent" : "",
        ].join(" ")}
      >
        <span className="truncate">{current ? current.label : placeholder}</span>
        <svg aria-hidden viewBox="0 0 20 20" className="h-3.5 w-3.5 shrink-0 opacity-70">
          <path d="M5 7.5 10 12.5 15 7.5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      {/* createPortal 的返回类型在 @types/react@19 与 React 18 的 ReactNode 不兼容（已知冲突），
          运行时没问题，这里只做类型转换 */}
      {popup as unknown as ReactNode}
    </div>
  );
}
