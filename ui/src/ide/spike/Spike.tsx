// EditorView 从主包拿（它是 codemirror 的 re-export）；@codemirror/view 只是传递依赖，别直接 import
import { basicSetup, EditorView } from "codemirror";
import { json } from "@codemirror/lang-json";
import { useVirtualizer } from "@tanstack/react-virtual";
import Button from "@jetbrains/ring-ui-built/components/button/button";
import { useEffect, useRef, useState } from "react";

/**
 * **实验页**（`#spike`）：验证两个候选小库够不够用 —— 不接真实数据、不进生产路径。
 *
 * 1. CodeMirror 6：能不能在 webview 里给出"编辑器"的手感（高亮 / 行号 / Ctrl+F / 撤销）。
 * 2. @tanstack/react-virtual：5000 行列表，虚拟与全量的 DOM 节点数、切换耗时。
 *
 * 验完就删，或者按结论升级成正式的 EditorPane。见 docs/dev-editor.md §13。
 */

const SAMPLE = JSON.stringify(
  {
    name: "Starcraft Mass Recall",
    version: "8.0",
    author: "SCMR Team",
    campaign: "swarm",
    id: "scmr",
    tags: ["重制", "中文"],
    kind: "campaign",
    miyin: {
      format: 3,
      main_map: "Maps/Campaign/swarm/scmr01.SC2Map",
      mods: ["SCMRmod"],
      overrides: [{ from: "assets", to: "Maps/Campaign/swarm" }],
    },
  },
  null,
  2,
);

const ROWS = 5000;
const rowLabel = (index: number) =>
  "Maps/Campaign/swarm/" + String(index + 1).padStart(4, "0") + ".SC2Map  ·  " + ((index * 37) % 900 + 100) + " KB";

export function Spike() {
  // ---------- ① CodeMirror ----------
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const [chars, setChars] = useState(SAMPLE.length);

  useEffect(() => {
    if (!host.current) return;
    const dark = document.documentElement.dataset.theme !== "light";
    const created = new EditorView({
      doc: SAMPLE,
      extensions: [
        basicSetup, // 行号 + 历史 + Ctrl+F 搜索 + 括号匹配
        json(),
        EditorView.theme(
          {
            "&": { height: "100%", fontSize: "12.5px", background: "transparent" },
            ".cm-content": { fontFamily: "ui-monospace, Consolas, monospace" },
            ".cm-gutters": { background: "transparent", borderRight: "none" },
            "&.cm-focused": { outline: "none" },
          },
          { dark },
        ),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) setChars(update.state.doc.length);
        }),
      ],
      parent: host.current,
    });
    view.current = created;
    return () => {
      view.current = null;
      created.destroy();
    };
  }, []);

  // ---------- ② 虚拟列表 ----------
  // `?spike=full` 直接进全量渲染（截图对比用）
  const [virtual, setVirtual] = useState(() => new URLSearchParams(location.search).get("spike") !== "full");
  const [stats, setStats] = useState<{ nodes: number; ms: number } | null>(null);
  const scroll = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: ROWS,
    getScrollElement: () => scroll.current,
    estimateSize: () => 22,
    overscan: 8,
  });

  const measure = () => {
    const before = performance.now();
    requestAnimationFrame(() => {
      const el = scroll.current;
      setStats({
        nodes: el ? el.querySelectorAll("*").length : 0,
        ms: Math.round(performance.now() - before),
      });
    });
  };

  useEffect(() => {
    measure();
  }, [virtual]);

  return (
    <div className="spike">
      <header className="spike__head">
        <div>
          <h2 className="spike__title">实验页 · 两个候选小库</h2>
          <p className="spike__lede">
            只验证效果，不接真实数据。验完按结论升级或删除（docs/dev-editor.md §13）。
          </p>
        </div>
        <Button onClick={() => (location.hash = "#gallery")}>去展台</Button>
      </header>

      <div className="spike__grid">
        <section className="spike__card">
          <h3 className="spike__card-title">① CodeMirror 6 · 编辑器</h3>
          <p className="spike__note">
            高亮 / 行号 / 括号匹配 / 撤销 / <strong>Ctrl+F 是编辑器内的搜索</strong>（没被浏览器抢走）。
            当前文档 {chars} 字符。
          </p>
          <div className="spike__editor" ref={host} />
        </section>

        <section className="spike__card">
          <h3 className="spike__card-title">② @tanstack/react-virtual · 5000 行</h3>
          <p className="spike__note">
            {stats
              ? "DOM 节点 " + stats.nodes + " 个 · 切换后到下一帧 " + stats.ms + " ms"
              : "测量中…"}
            {" · "}
            <Button onClick={() => setVirtual((value) => !value)}>
              {virtual ? "切到全量渲染" : "切回虚拟渲染"}
            </Button>
          </p>
          <div className="spike__list" ref={scroll}>
            {virtual ? (
              <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
                {virtualizer.getVirtualItems().map((item) => (
                  <div
                    key={item.key}
                    className="spike__row"
                    style={{ position: "absolute", top: 0, left: 0, right: 0, transform: "translateY(" + item.start + "px)" }}
                  >
                    {rowLabel(item.index)}
                  </div>
                ))}
              </div>
            ) : (
              Array.from({ length: ROWS }, (_, index) => (
                <div key={index} className="spike__row">
                  {rowLabel(index)}
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
