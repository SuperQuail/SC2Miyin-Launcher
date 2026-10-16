import { useEffect, useMemo, useState } from "react";

import { CHEATS, CHEAT_SCOPES, matchCheats, type Cheat, type CheatScope } from "../../../api/cheats";
import { copyText } from "../../lib/clipboard";
import { Button } from "../../ui";
import "./cheats.css";

/** 外面（Vue 外壳）能提供的东西：提示与关闭。React 侧不直接碰外壳的状态。 */
export interface CheatsHandlers {
  notify: (kind: "success" | "error", text: string) => void;
  onClose: () => void;
}

/**
 * 作弊码查询页（React 版，从 `views/CheatsView.vue` 整块迁过来）。
 *
 * 职责没变：查得到、抄得走 —— 搜一行、点一下复制。复制两条路都留着
 * （Clipboard API 与 execCommand），WebView 里哪条通不一定。
 */
export function CheatsView({ notify, onClose }: CheatsHandlers) {
  const [keyword, setKeyword] = useState("");
  const [scope, setScope] = useState<CheatScope | "全部">("全部");
  /** 刚复制过的码，用来给一格"已复制"的反馈。 */
  const [copied, setCopied] = useState("");

  const list = useMemo(() => matchCheats(keyword, scope), [keyword, scope]);
  const scopeOptions = useMemo(() => ["全部", ...CHEAT_SCOPES] as const, []);

  /** 每个适用范围下有多少条，显示在筛选按钮上。 */
  const countOf = (option: CheatScope | "全部") => matchCheats("", option).length;

  // Esc 关掉；这是覆盖层不是页面，得有个退路
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  async function copy(cheat: Cheat): Promise<void> {
    const ok = await copyText(cheat.code);
    if (!ok) {
      notify("error", "复制失败，请手动选中");
      return;
    }
    setCopied(cheat.code);
    notify("success", "已复制 " + cheat.code);
    // 过一会儿把"已复制"收回去，避免满屏都是勾
    window.setTimeout(() => setCopied((current) => (current === cheat.code ? "" : current)), 1600);
  }

  return (
    <div className="sheet" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <section className="cheats">
        <header className="cheats__head">
          <h2 className="cheats__title">作弊码</h2>
          <Button variant="text" title="关闭" onClick={onClose}>
            <span className="cheats__close">关闭</span>
          </Button>
          <p className="cheats__hint">
            在战役里按 <kbd>Enter</kbd> 打开聊天框，粘贴后回车。
            <strong>单人战役有效，对战时无效。</strong>
          </p>
        </header>

        <div className="cheats__bar">
          <input
            className="cheats__search"
            type="search"
            value={keyword}
            placeholder="搜索效果或作弊码，例如「晶矿」「无敌」"
            onChange={(event) => setKeyword(event.target.value)}
          />
          <div className="cheats__scopes">
            {scopeOptions.map((option) => (
              <button
                key={option}
                className={"chip" + (scope === option ? " chip--on" : "")}
                type="button"
                onClick={() => setScope(option)}
              >
                {option}
                <span className="chip__count">{countOf(option)}</span>
              </button>
            ))}
          </div>
        </div>

        {list.length === 0 ? (
          <p className="cheats__empty">没有匹配的作弊码。换个关键词试试，或者点「全部」。</p>
        ) : (
          <ul className="cheats__list">
            {list.map((cheat) => (
              <li key={cheat.code} className="cheat">
                <div className="cheat__main">
                  <code className="cheat__code">{cheat.code}</code>
                  <span className="cheat__effect">{cheat.effect}</span>
                </div>
                <div className="cheat__meta">
                  {cheat.scope !== "通用" && <span className="cheat__scope">{cheat.scope}</span>}
                  {cheat.note && <span className="cheat__note">{cheat.note}</span>}
                </div>
                <button
                  className={"cheat__copy" + (copied === cheat.code ? " cheat__copy--done" : "")}
                  type="button"
                  title={"复制 " + cheat.code}
                  onClick={() => void copy(cheat)}
                >
                  {copied === cheat.code ? "已复制" : "复制"}
                </button>
              </li>
            ))}
          </ul>
        )}

        <footer className="cheats__foot">
          共 {CHEATS.length} 条。带战役名的只在对应战役里生效，其余通用。
        </footer>
      </section>
    </div>
  );
}
