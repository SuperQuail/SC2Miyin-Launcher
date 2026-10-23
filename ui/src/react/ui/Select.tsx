import { useEffect, useId, useRef, useState } from "react";

export interface SelectOption {
  value: string;
  label: string;
  /** 副标题（比如「原版」「2 个版本」这种说明）。 */
  hint?: string;
}

/**
 * 项目自己的下拉。原生 select 的弹层样式各平台不一、也没法加说明文字，
 * 而这里要的是「贴合弥音风格」的控件，所以自己画一个。
 *
 * 只做用得上的：点开、选中、点外面关、Esc 关、上下键移动、Enter 选。
 * 颜色走 tokens（见 styles/tailwind.css 里的 @theme 映射），不写死。
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
  const rootRef = useRef<HTMLDivElement | null>(null);
  const listId = useId();
  const current = options.find((item) => item.value === value);

  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  const choose = (next: string) => {
    onChange(next);
    setOpen(false);
  };

  return (
    <div ref={rootRef} className="relative inline-block text-left">
      <button
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
          "flex min-w-[9.5rem] items-center justify-between gap-2 rounded-md px-3 py-1.5 text-[12.5px]",
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

      {open && (
        <ul
          id={listId}
          role="listbox"
          className="absolute right-0 z-50 mt-1 max-h-64 min-w-[13rem] overflow-auto rounded-lg border border-[color-mix(in_srgb,var(--outline)_80%,transparent)] bg-surface p-1 shadow-lg"
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
        </ul>
      )}
    </div>
  );
}
