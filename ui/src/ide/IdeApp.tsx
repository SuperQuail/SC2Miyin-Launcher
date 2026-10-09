import { useState } from "react";
import Button from "@jetbrains/ring-ui-built/components/button/button";
import Checkbox from "@jetbrains/ring-ui-built/components/checkbox/checkbox";
import Group from "@jetbrains/ring-ui-built/components/group/group";
import Input from "@jetbrains/ring-ui-built/components/input/input";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";

/**
 * 开发者页 —— 控件用 JetBrains 官方的 Ring UI，外壳（分栏、图形道、状态栏）
 * 还是我们自己的 CSS：Ring UI 是控件库，不是 IDE 骨架库。
 */
type FileId = "text" | "binary" | "image";

const TABS = [
  { id: "text" as FileId, name: "说明.txt", dir: "Terran 重制", st: "A" },
  { id: "binary" as FileId, name: "Terran01.SC2Map", dir: "1. Rebel Yell", st: "M" },
  { id: "image" as FileId, name: "封面.png", dir: "Terran 重制", st: "A" },
];

const FILES = [
  { st: "M", name: "Terran01.SC2Map", dir: "1. Rebel Yell", size: "1.2 MB" },
  { st: "M", name: "Terran02.SC2Map", dir: "1. Rebel Yell", size: "1.1 MB" },
  { st: "A", name: "说明.txt", dir: "Terran 重制", size: "4 KB" },
  { st: "A", name: "封面.png", dir: "Terran 重制", size: "310 KB" },
  { st: "D", name: "Old.SC2Mod", dir: "Mods", size: "12 KB" },
];

const SOURCE = [
  "复刻战役 · Terran 重制",
  "",
  "1. 解压到星际争霸 II 安装目录，覆盖同名文件。",
  "2. 启动器里点「启用这个版本」，再点「开始游戏」。",
  "",
  "# 这次改了什么",
  "- Marine 生命 45 → 60",
  "- 武器 Gauss Rifle → Impaler",
  "- 终章关补了一段过场",
  "",
  "联系方式：群 123456789",
];

/** IJ 的图形道：每条分支一个颜色，HEAD 画成环，其余是实心点。 */
const LOG = [
  { ver: "未提交", msg: "12 项改动 · 340 MB", refs: [] as string[], who: "—", when: "现在", lane: 0, color: "#629755", head: true },
  { ver: "v8.0.1", msg: "修复：终章关的触发条件", refs: ["当前"], who: "唐天", when: "昨天 22:10", lane: 0, color: "#629755" },
  { ver: "v8.0", msg: "基线：Terran 重制 v8.0", refs: ["基线"], who: "唐天", when: "2 天前", lane: 0, color: "#629755", merge: true },
  { ver: "v7.9", msg: "数值：Marine 生命 40→45", refs: [], who: "唐天", when: "4 天前", lane: 1, color: "#B3893A" },
  { ver: "v7.8", msg: "关卡：Rebel Yell 加了一段过场", refs: [], who: "唐天", when: "上周", lane: 1, color: "#7A3E9D", last: true },
];

/**
 * 图形道。照 IJ 的画法：分支线是竖线，HEAD 画成**环**（外圈描边 + 内芯），
 * 其余是实心点；换道的地方用一段弧接过去。
 */
function Graph({ lane, color, head, merge, last }: { lane: number; color: string; head?: boolean; merge?: boolean; last?: boolean }) {
  const x = 11 + lane * 13;
  return (
    <svg className="graph" width="38" height="26" viewBox="0 0 38 26">
      <line x1={x} y1="0" x2={x} y2={last ? 13 : 26} stroke={color} strokeWidth="2" />
      {merge && <path d={"M11 0 C11 13, " + x + " 13, " + x + " 13"} fill="none" stroke="#B3893A" strokeWidth="2" />}
      {head ? (
        <>
          <circle cx={x} cy="13" r="5.5" fill="var(--surface-1)" stroke={color} strokeWidth="2.5" />
          <circle cx={x} cy="13" r="2" fill={color} />
        </>
      ) : (
        <circle cx={x} cy="13" r="4.5" fill={color} />
      )}
    </svg>
  );
}

const GLYPH = {
  menu: "M2.5 4h11M2.5 8h11M2.5 12h11",
  refresh: "M13 8a5 5 0 1 1-1.5-3.5 M13 2.5V5h-2.5",
  undo: "M3 8a5 5 0 1 0 1.5-3.5 M3 2.5V5h2.5",
  close: "M4 4l8 8M12 4l-8 8",
  diff: "M4 3v10M12 3v10M4 8h8",
  commit: "M8 1.5v4.3M8 10.2v4.3 M8 8m-2.2 0a2.2 2.2 0 1 0 4.4 0a2.2 2.2 0 1 0 -4.4 0",
  folder: "M2.5 4.5h4l1.2 1.6h5.8v6.4h-11z",
  graph: "M4.5 2.2a1.8 1.8 0 1 0 0 3.6a1.8 1.8 0 1 0 0-3.6M4.5 10.2a1.8 1.8 0 1 0 0 3.6a1.8 1.8 0 1 0 0-3.6M4.5 5.8v4.4M8 4h5.5M8 12h5.5",
  gear: "M8 6a2 2 0 1 0 0 4a2 2 0 1 0 0-4M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4",
  scan: "M2.5 5.5h11v8h-11zM2.5 5.5l1.6-3h7.8l1.6 3M8 8v3M6.5 9.5 8 11l1.5-1.5",
  export: "M8 2.5v7M5 6.5 8 9.5l3-3M3 12.5h10",
  caretDown: "M4 6.5 8 10.5l4-4",
};

function Glyph({ d, size = 14 }: { d: string; size?: number }) {
  return (
    <svg className="glyph" width={size} height={size} viewBox="0 0 16 16">
      <path d={d} />
    </svg>
  );
}

/**
 * Ring UI 的 icon 属性要的是**组件类型**（它内部自己渲染 <Icon glyph={...}/>），
 * 传 <svg/> 这样的**元素**会被当成对象，直接抛 Element type is invalid。
 * 这里按 d 缓存成稳定的组件类型，免得每次 render 都换一个类型导致重挂载。
 */
const GLYPH_CACHE: Record<string, () => JSX.Element> = {};
function glyph(name: keyof typeof GLYPH): () => JSX.Element {
  if (!GLYPH_CACHE[name]) {
    const d = GLYPH[name];
    GLYPH_CACHE[name] = () => (
      <svg className="glyph" width={14} height={14} viewBox="0 0 16 16">
        <path d={d} />
      </svg>
    );
  }
  return GLYPH_CACHE[name];
}

export function IdeApp({ file: initial }: { file: string }) {
  const [file, setFile] = useState<FileId>((initial as FileId) ?? "text");
  const [picked, setPicked] = useState(0);
  const [search, setSearch] = useState("");
  const [msg, setMsg] = useState("");
  const [amend, setAmend] = useState(false);
  const [names, setNames] = useState<string[]>(FILES.map((f) => f.name));

  const toggle = (name: string) =>
    setNames((prev) => (prev.includes(name) ? prev.filter((n) => n !== name) : [...prev, name]));

  return (
    <div className="ide">
      <div className="tb">
        <Button icon={glyph("menu")} />
        <Button>复刻战役</Button>
        <Button>基线 v8.0</Button>
        <span className="tsep" />
        <Button icon={glyph("scan")}>扫描</Button>
        <Button icon={glyph("diff")}>对比</Button>
        <Button icon={glyph("export")}>预览变更</Button>
        <span className="tsep" />
        <Button>导出差异</Button>
        <span className="tspacer" />
        <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="搜索" className="tsearch" />
        <Button icon={glyph("gear")} />
        <Button primary>提交 v8.1</Button>
      </div>

      <div className="body">
        <nav className="rail">
          <button className="rbtn rbtn--on" title="提交"><Glyph d={GLYPH.commit} size={16} /></button>
          <button className="rbtn" title="游戏目录"><Glyph d={GLYPH.folder} size={16} /></button>
          <button className="rbtn" title="日志"><Glyph d={GLYPH.graph} size={16} /></button>
          <span className="rspacer" />
          <button className="rbtn" title="设置"><Glyph d={GLYPH.gear} size={16} /></button>
        </nav>

        <div className="main">
          <div className="work">
            <section className="pane pane--commit">
              <header className="pbar"><span className="ptitle">提交</span>
                <span className="ptools">
                  <Button icon={glyph("refresh")} />
                  <Button icon={glyph("undo")} />
                  <Button icon={glyph("close")} />
                </span>
              </header>
              <div className="pbody">
                <p className="group"><Checkbox checked={names.length > 0} onChange={() => {}} /><span className="caret">▾</span>改 {names.length} 个文件<span className="meta">340 MB</span></p>
                <ul className="flist">
                  {FILES.map((f) => (
                    <li key={f.name} className={"frow" + (f.name === TABS.find((t) => t.id === file)?.name ? " is-open" : "")}>
                      <Checkbox checked={names.includes(f.name)} onChange={() => toggle(f.name)} />
                      <span className={"badge badge--" + (f.st === "A" ? "add" : f.st === "D" ? "del" : "mod")}>{f.st}</span>
                      <span className="fname">{f.name}</span>
                      <span className="fdir">{f.dir}</span>
                      <span className="fsize">{f.size}</span>
                    </li>
                  ))}
                </ul>
                <div className="msg">
                  <p className="amend"><Checkbox checked={amend} onChange={() => setAmend(!amend)} label="修正上一版" /><span>上次：修了一张图 · 昨天</span></p>
                  <textarea rows={3} value={msg} onChange={(e) => setMsg(e.target.value)} placeholder="内容：一句话说清这次改了什么" />
                  <p className="hint">
                    <span className="chips">
                      <Button small>内容：</Button>
                      <Button small>数值：</Button>
                      <Button small>修复：</Button>
                    </span>
                    <span className="auto">v8.0 → v8.1</span>
                  </p>
                </div>
              </div>
              <footer className="pfoot">
                <Button>提交并导出…</Button>
                <span className="pspacer" />
                <Group>
                  <Button primary>提交 v8.1</Button>
                  <Button primary icon={glyph("caretDown")} />
                </Group>
              </footer>
            </section>

            <section className="pane pane--editor">
              <header className="tabsbar">
                {TABS.map((tab) => (
                  <button
                    key={tab.id}
                    type="button"
                    className={"doc" + (file === tab.id ? " doc--on" : "")}
                    onClick={() => setFile(tab.id)}
                  >
                    <span className={"badge badge--" + (tab.st === "A" ? "add" : "mod")}>{tab.st}</span>
                    {tab.name}
                    <span className="docx">×</span>
                  </button>
                ))}
                <span className="tspacer" />
              </header>

              <div className="pbody pbody--flat">
                {file === "text" && (
                  <div className="code">
                    {SOURCE.map((line, i) => (
                      <div className="cline" key={i}>
                        <span className="cno">{i + 1}</span>
                        <span className="ctext">{line}</span>
                      </div>
                    ))}
                  </div>
                )}

                {file === "binary" && (
                  <div className="unsupported">
                    <Glyph d="M8 3.5h6v9H8zM3 6.5h4v6H3z" size={44} />
                    <p className="utitle">二进制文件</p>
                    <p className="utext">这个文件不是文本，编辑器没法按行显示它。<br />它是一张地图（MPQ 归档），要改内容请在游戏编辑器里打开。</p>
                    <p className="uact"><Button>十六进制预览</Button><Button>用系统默认程序打开</Button></p>
                    <p className="uhint">大小 1.2 MB · 修改时间 今天 14:02 · 类型 SC2Map</p>
                  </div>
                )}

                {file === "image" && (
                  <div className="viewer">
                    <div className="stage"><div className="shot">封面.png</div></div>
                    <p className="vbar"><span>310 KB · 512 × 512</span><span className="vzoom">适应窗口 · 100% · － ＋</span></p>
                  </div>
                )}
              </div>

              <footer className="status">
                <span>{file === "text" ? "行 7，列 1" : file === "binary" ? "二进制 · 只读" : "图片"}</span>
                <span className="sspacer" />
                <span>UTF-8</span><span>空格: 2</span><span>{file === "text" ? "纯文本" : "只读"}</span>
              </footer>
            </section>

            <section className="pane pane--diff">
              <header className="pbar"><span className="ptitle">语义 diff</span><span className="psub">Terran01.SC2Map</span></header>
              <div className="pbody">
                <p className="dunit">Marine</p>
                <p className="drow"><span className="dfield">生命</span><span className="dfrom">45</span><span className="darrow">→</span><span className="dto">60</span></p>
                <p className="drow"><span className="dfield">武器</span><span className="dfrom">Gauss Rifle</span><span className="darrow">→</span><span className="dto">Impaler</span></p>
                <p className="dunit dunit--gap">Medivh</p>
                <p className="drow"><span className="dfield">护盾</span><span className="dfrom">0</span><span className="darrow">→</span><span className="dto">50</span></p>
              </div>
            </section>
          </div>

          <section className="pane pane--log">
            <header className="pbar"><span className="ptitle">日志</span>
              <span className="ptools"><Button icon={glyph("refresh")} /></span>
            </header>
            <div className="pbody pbody--flat logwrap">
              <ul className="tree">
                {LOG.map((row, i) => (
                  <li key={row.ver} className={"trow" + (picked === i ? " is-picked" : "")} onClick={() => setPicked(i)}>
                    <Graph lane={row.lane} color={row.color} head={row.head} merge={row.merge} last={row.last} />
                    <span className="tver">{row.ver}</span>
                    <span className="tmsg">{row.msg}</span>
                    <span className="refs">{row.refs.map((r) => <Tag key={r}>{r}</Tag>)}</span>
                    <span className="twho">{row.who}</span>
                    <span className="twhen">{row.when}</span>
                  </li>
                ))}
              </ul>
              <aside className="ldetail">
                <p className="ldtitle">{LOG[picked].ver}</p>
                <p className="ldmsg">{LOG[picked].msg}</p>
                <ul className="mini">{FILES.slice(0, 3).map((f) => <li key={f.name}><span>{f.st}</span>{f.name}</li>)}</ul>
              </aside>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
