import { useCallback, useEffect, useMemo, useState } from "react";
import Button from "@jetbrains/ring-ui-built/components/button/button";
import Checkbox from "@jetbrains/ring-ui-built/components/checkbox/checkbox";
import Group from "@jetbrains/ring-ui-built/components/group/group";
import Input from "@jetbrains/ring-ui-built/components/input/input";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";

import {
  type DevEntry,
  type FilePreview,
  isDesktop,
  pickDirectory,
  readFile,
  scan,
} from "./api";

/**
 * 开发者页。
 *
 * **左边和编辑器接的是真数据**（`dev_scan` / `dev_read_file` 两条命令）：
 * 目录树是扫出来的，勾选清单是真的会进这个包的东西，编辑器打开的是磁盘上的文件。
 *
 * **版本管理（提交 / 回滚 / 日志）还没接后端** —— 那一块要 SC2Diff 和基线的概念，
 * 见 docs/developer-workflow.md §4.3。界面上明确标着「示例」，按钮是禁用的，
 * 不假装能用。
 */
const PALETTE = ["#629755", "#B3893A", "#7A3E9D", "#4A86E8"];

/** 一条提交在图形道上的样子（示例数据）。 */
type Commit = {
  ver: string;
  msg: string;
  who: string;
  when: string;
  lane: number;
  color: string;
  head?: boolean;
  merge?: boolean;
  last?: boolean;
};

const SAMPLE_LOG: Commit[] = [
  { ver: "未提交", msg: "工作区里勾选的东西", who: "—", when: "现在", lane: 0, color: PALETTE[0], head: true },
  { ver: "v8.0.1", msg: "修复：终章关的触发条件", who: "—", when: "示例", lane: 0, color: PALETTE[0] },
  { ver: "v8.0", msg: "基线：Terran 重制 v8.0", who: "—", when: "示例", lane: 0, color: PALETTE[0], merge: true },
  { ver: "v7.9", msg: "数值：Marine 生命 40→45", who: "—", when: "示例", lane: 1, color: PALETTE[1] },
  { ver: "v7.8", msg: "关卡：Rebel Yell 加了一段过场", who: "—", when: "示例", lane: 1, color: PALETTE[2], last: true },
];

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + " MB";
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + " GB";
}

/** IJ 的图形道：HEAD 画环，其余实心点，换道画弧。 */
function Graph({ lane, color, head, merge, last }: { lane: number; color: string; head?: boolean; merge?: boolean; last?: boolean }) {
  const x = 11 + lane * 13;
  return (
    <svg className="graph" width="38" height="26" viewBox="0 0 38 26">
      <line x1={x} y1="0" x2={x} y2={last ? 13 : 26} stroke={color} strokeWidth="2" />
      {merge && <path d={"M11 0 C11 13, " + x + " 13, " + x + " 13"} fill="none" stroke={PALETTE[1]} strokeWidth="2" />}
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

export function IdeApp() {
  const [entries, setEntries] = useState<DevEntry[]>([]);
  const [truncated, setTruncated] = useState(false);
  const [extras, setExtras] = useState<string[]>([]);
  const [picked, setPicked] = useState<string[]>([]);
  const [opened, setOpened] = useState<DevEntry[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [preview, setPreview] = useState<FilePreview | null>(null);
  const [search, setSearch] = useState("");
  const [msg, setMsg] = useState("");
  const [amend, setAmend] = useState(false);
  const [logPicked, setLogPicked] = useState(0);
  const [status, setStatus] = useState(isDesktop ? "正在扫描游戏目录…" : "浏览器预览：没有 IPC，数据是示例");

  const refresh = useCallback(async (extra: string[]) => {
    setStatus("正在扫描游戏目录…");
    const result = await scan(extra);
    if (!result) {
      setStatus("没扫到东西 —— 先在设置里指定星际争霸 II 的安装目录");
      return;
    }
    setEntries(result.entries);
    setTruncated(result.truncated);
    setStatus("扫到 " + result.entries.length + " 项" + (result.truncated ? "（已截断）" : ""));
  }, []);

  useEffect(() => {
    void refresh([]);
  }, [refresh]);

  // 打开一个文件：真去磁盘读，按结果决定编辑器怎么显示
  const open = useCallback(async (entry: DevEntry) => {
    setOpened((prev) => (prev.some((item) => item.abs === entry.abs) ? prev : [...prev, entry]));
    setActive(entry.abs);
    setStatus("正在读 " + entry.name + "…");
    const result = await readFile(entry.abs);
    setPreview(result);
    if (result) {
      setStatus(result.kind === "text" ? "行 " + result.lines + " · " + formatBytes(result.bytes) : result.kind === "image" ? "图片 · " + formatBytes(result.bytes) : "二进制 · 只读");
    } else {
      setStatus("读不了这个文件");
    }
  }, []);

  const addDirectory = useCallback(async () => {
    const dir = await pickDirectory();
    if (!dir) return;
    const next = [...extras, dir];
    setExtras(next);
    await refresh(next);
  }, [extras, refresh]);

  const toggle = (path: string) =>
    setPicked((prev) => (prev.includes(path) ? prev.filter((item) => item !== path) : [...prev, path]));

  // 目录树：按第一段分组，最多每组建 MAX_PER_GROUP 行
  const tree = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    const rows: { group: string; entry: DevEntry }[] = [];
    for (const entry of entries) {
      if (keyword && !entry.path.toLowerCase().includes(keyword)) continue;
      const group = entry.external ? "自定义目录" : entry.path.split("/")[0] || "/";
      rows.push({ group, entry });
    }
    const grouped = new Map<string, DevEntry[]>();
    for (const row of rows) {
      const list = grouped.get(row.group) ?? [];
      if (list.length < 60) list.push(row.entry);
      grouped.set(row.group, list);
    }
    return grouped;
  }, [entries, search]);

  const pickedBytes = useMemo(
    () => entries.filter((entry) => picked.includes(entry.path)).reduce((sum, entry) => sum + entry.bytes, 0),
    [entries, picked],
  );

  return (
    <div className="ide">
      <div className="tb">
        <Button onClick={() => (location.href = "/index.html")}>← 返回启动器</Button>
        <span className="tsep" />
        <Button>复刻战役</Button>
        <Button>基线 v8.0</Button>
        <span className="tsep" />
        <Button onClick={() => void refresh(extras)}>重新扫描</Button>
        <Button onClick={() => void addDirectory()}>添加目录…</Button>
        <span className="tsep" />
        <Button disabled>对比</Button>
        <Button disabled>预览变更</Button>
        <span className="tspacer" />
        <Input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="搜索" className="tsearch" />
        <Button primary disabled title="版本管理后端还没接">提交 v8.1</Button>
      </div>

      <div className="body">
        <nav className="rail">
          <button className="rbtn rbtn--on" title="游戏目录"><svg className="i" viewBox="0 0 16 16"><path d="M2.5 4.5h4l1.2 1.6h5.8v6.4h-11z" /></svg></button>
        </nav>

        <div className="main">
          <div className="work">
            {/* ---------- 目录树（真数据） ---------- */}
            <section className="pane pane--commit">
              <header className="pbar">
                <span className="ptitle">游戏目录</span>
                <span className="psub">{status}</span>
                <span className="ptools">
                  <Button onClick={() => void refresh(extras)} small>刷新</Button>
                </span>
              </header>
              <div className="pbody">
                {[...tree.entries()].map(([group, list]) => (
                  <div key={group} className="treegroup">
                    <p className="group"><span className="caret">▾</span>{group}<span className="meta">{list.length}</span></p>
                    <ul className="flist">
                      {list.map((entry) => (
                        <li
                          key={entry.path}
                          className={"frow" + (active === entry.abs ? " is-open" : "") + (entry.external ? " frow--ext" : "")}
                          onClick={() => (entry.is_dir ? undefined : void open(entry))}
                        >
                          {!entry.is_dir && (
                            <Checkbox checked={picked.includes(entry.path)} onChange={() => toggle(entry.path)} />
                          )}
                          <span className="fname">{entry.name}</span>
                          <span className="fdir">{entry.is_dir ? "" : entry.path.replace("/" + entry.name, "")}</span>
                          <span className="fsize">{entry.is_dir ? "" : formatBytes(entry.bytes)}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
                {entries.length === 0 && (
                  <p className="empty">
                    还没有内容。<br />
                    先确认设置页里指定了星际争霸 II 的安装目录，或者点上面「添加目录…」
                    从别处加一个进来。
                  </p>
                )}
                {truncated && <p className="empty">内容太多，只列了前 4000 项。</p>}
              </div>
            </section>

            {/* ---------- 编辑器（真数据） ---------- */}
            <section className="pane pane--editor">
              <header className="tabsbar">
                {opened.map((entry) => (
                  <button
                    key={entry.abs}
                    type="button"
                    className={"doc" + (active === entry.abs ? " doc--on" : "")}
                    onClick={() => void open(entry)}
                  >
                    {entry.name}
                    <span
                      className="docx"
                      onClick={(event) => {
                        event.stopPropagation();
                        setOpened((prev) => prev.filter((item) => item.abs !== entry.abs));
                      }}
                    >
                      ×
                    </span>
                  </button>
                ))}
                <span className="tspacer" />
              </header>

              <div className="pbody pbody--flat">
                {!preview && (
                  <div className="unsupported">
                    <p className="utitle">左边点一个文件</p>
                    <p className="utext">
                      文本直接按行打开；图片直接显示；<br />
                      二进制（地图、模组这类）会告诉你打不开、可以怎么打开。
                    </p>
                  </div>
                )}

                {preview?.kind === "text" && (
                  <div className="code">
                    {preview.text.split("\n").map((line, index) => (
                      <div className="cline" key={index}>
                        <span className="cno">{index + 1}</span>
                        <span className="ctext">{line}</span>
                      </div>
                    ))}
                    {preview.truncated && <p className="empty">文件太大，只显示了前面一段。</p>}
                  </div>
                )}

                {preview?.kind === "binary" && (
                  <div className="unsupported">
                    <svg className="i" width="44" height="44" viewBox="0 0 16 16"><path d="M8 3.5h6v9H8zM3 6.5h4v6H3z" /></svg>
                    <p className="utitle">二进制文件</p>
                    <p className="utext">
                      这个文件不是文本，编辑器没法按行显示它。<br />
                      地图和模组是 MPQ 归档，要改内容请在游戏编辑器里打开。
                    </p>
                    <p className="uact">
                      <Button disabled>十六进制预览</Button>
                      <Button disabled>用系统默认程序打开</Button>
                    </p>
                    <p className="uhint">大小 {formatBytes(preview.bytes)}</p>
                  </div>
                )}

                {preview?.kind === "image" && (
                  <div className="viewer">
                    <div className="stage">
                      <div className="shot">{opened.find((item) => item.abs === active)?.name ?? "图片"}</div>
                    </div>
                    <p className="vbar"><span>{formatBytes(preview.bytes)}</span><span className="vzoom">适应窗口 · 100% · － ＋</span></p>
                  </div>
                )}
              </div>

              <footer className="status">
                <span>{status}</span>
                <span className="sspacer" />
                <span>勾选 {picked.length} 项 · {formatBytes(pickedBytes) || "0 B"}</span>
              </footer>
            </section>

            {/* ---------- 版本管理（未接，明确标出） ---------- */}
            <section className="pane pane--diff">
              <header className="pbar">
                <span className="ptitle">版本</span>
                <span className="psub">后端未接 · 下面是示例</span>
              </header>
              <div className="pbody">
                <p className="hintbox">
                  版本管理与提交要 SC2Diff 和「基线」的概念，还没实现。
                  界面上这一块的数据是**示例**，按钮是禁用的。
                </p>
                <p className="dunit">这个包会包含</p>
                <p className="drow"><span className="dfield">文件</span><span className="dto">{picked.length}</span></p>
                <p className="drow"><span className="dfield">体积</span><span className="dto">{formatBytes(pickedBytes) || "0 B"}</span></p>
                {extras.length > 0 && (
                  <>
                    <p className="dunit dunit--gap">自定义目录</p>
                    {extras.map((dir) => (
                      <p className="drow" key={dir}><span className="dfield">外部</span><span className="dfrom">{dir}</span></p>
                    ))}
                  </>
                )}
              </div>
            </section>
          </div>

          {/* ---------- 日志（未接） ---------- */}
          <section className="pane pane--log">
            <header className="pbar">
              <span className="ptitle">日志</span>
              <span className="psub">示例</span>
            </header>
            <div className="pbody pbody--flat logwrap">
              <ul className="tree">
                {SAMPLE_LOG.map((row, index) => (
                  <li key={row.ver} className={"trow" + (logPicked === index ? " is-picked" : "")} onClick={() => setLogPicked(index)}>
                    <Graph lane={row.lane} color={row.color} head={row.head} merge={row.merge} last={row.last} />
                    <span className="tver">{row.ver}</span>
                    <span className="tmsg">{row.msg}</span>
                    <span className="refs">{row.head ? <Tag>未提交</Tag> : null}</span>
                    <span className="twho">{row.who}</span>
                    <span className="twhen">{row.when}</span>
                  </li>
                ))}
              </ul>
              <aside className="ldetail">
                <p className="ldtitle">还没接</p>
                <p className="ldmsg">
                  提交图要等版本管理接上 SC2Diff 之后才有真数据。
                </p>
                <p className="ldmsg">目前能用的：扫描目录、勾选、看文件。</p>
              </aside>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
