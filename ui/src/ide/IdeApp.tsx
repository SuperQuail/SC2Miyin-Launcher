import { useCallback, useEffect, useMemo, useState } from "react";

// 窗口按钮走启动器那套命令（bridge 是框架无关的纯 TS，两边共用 —— AGENTS.md §18）
import { api as launcher } from "../api/bridge";
import Button from "@jetbrains/ring-ui-built/components/button/button";
import Input from "@jetbrains/ring-ui-built/components/input/input";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";

import {
  type CommitRecord,
  type DevEntry,
  type FilePreview,
  type HistoryDiff,
  type PackageMeta,
  commit as commitVersion,
  diff as diffVersions,
  exportPackage,
  history as loadHistory,
  isDesktop,
  pickDirectory,
  pickExportPath,
  readFile,
  scan,
} from "./api";

/**
 * 开发者页：挑文件 → 填包信息 → 导出压缩包；顺带记版本、看文件。
 *
 * 页面上的数据**全部来自真实扫描**，没有内置的示例战役 / 版本号 ——
 * 包名一开始是空的，写什么由用户决定（只记住"上次用的那个"）。
 */

/** 包名存在本地：它是"我在给哪个包干活"，不是随包走的数据。 */
const PACKAGE_KEY = "miyin.dev.campaign";

function loadPackageName(): string {
  try {
    return localStorage.getItem(PACKAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

/** 树：目录节点没有 entry，文件节点有。 */
type TreeNode = { name: string; path: string; entry?: DevEntry; children: TreeNode[] };

function buildTree(entries: DevEntry[]): TreeNode[] {
  const roots: TreeNode[] = [];
  const index = new Map<string, TreeNode>();

  const ensureDir = (path: string, list: TreeNode[]): TreeNode => {
    const existing = index.get(path);
    if (existing) return existing;
    const parents = path.split("/").slice(0, -1);
    const parentPath = parents.join("/");
    const parentList = parentPath ? ensureDir(parentPath, list).children : roots;
    const node: TreeNode = { name: path.split("/").pop() ?? path, path, children: [] };
    index.set(path, node);
    parentList.push(node);
    return node;
  };

  for (const entry of entries) {
    const parts = entry.path.split("/").filter(Boolean);
    if (parts.length === 0) continue;
    if (entry.is_dir) {
      // 目录：可能是别人先给它建好了父节点
      const node = index.get(entry.path) ?? ensureDir(entry.path, roots);
      node.entry = entry;
      continue;
    }
    const parentPath = parts.slice(0, -1).join("/");
    const list = parentPath ? ensureDir(parentPath, roots).children : roots;
    const node: TreeNode = { name: entry.name, path: entry.path, entry, children: [] };
    index.set(entry.path, node);
    list.push(node);
  }

  const sort = (list: TreeNode[]) => {
    list.sort((left, right) => {
      const leftDir = left.children.length > 0 || left.entry?.is_dir ? 0 : 1;
      const rightDir = right.children.length > 0 || right.entry?.is_dir ? 0 : 1;
      return leftDir === rightDir ? left.name.localeCompare(right.name, "zh") : leftDir - rightDir;
    });
    list.forEach((node) => sort(node.children));
  };
  sort(roots);
  return roots;
}

/** 这个节点底下的全部文件（含自己）。 */
function filesUnder(node: TreeNode, out: string[] = []): string[] {
  if (node.entry && !node.entry.is_dir) out.push(node.path);
  node.children.forEach((child) => filesUnder(child, out));
  return out;
}

function bytesUnder(node: TreeNode): number {
  if (node.entry && !node.entry.is_dir) return node.entry.bytes;
  return node.children.reduce((sum, child) => sum + bytesUnder(child), 0);
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + " MB";
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + " GB";
}

function when(at: number): string {
  if (!at) return "";
  const delta = Date.now() / 1000 - at;
  if (delta < 60) return "刚刚";
  if (delta < 3600) return Math.floor(delta / 60) + " 分钟前";
  if (delta < 86400) return Math.floor(delta / 3600) + " 小时前";
  if (delta < 172800) return "昨天";
  return new Date(at * 1000).toLocaleDateString("zh-CN");
}

const PALETTE = ["#629755", "#B3893A", "#7A3E9D", "#4A86E8"];

/** IJ 的图形道：HEAD 画环，其余实心点。 */
function Graph({ color, head, last }: { color: string; head?: boolean; last?: boolean }) {
  return (
    <svg className="graph" width="38" height="26" viewBox="0 0 38 26">
      <line x1="11" y1="0" x2="11" y2={last ? 13 : 26} stroke={color} strokeWidth="2" />
      {head ? (
        <>
          <circle cx="11" cy="13" r="5.5" fill="var(--surface-1)" stroke={color} strokeWidth="2.5" />
          <circle cx="11" cy="13" r="2" fill={color} />
        </>
      ) : (
        <circle cx="11" cy="13" r="4.5" fill={color} />
      )}
    </svg>
  );
}

/** 勾选框：目录要显示"部分选中"，所以用原生 input（能设 indeterminate）。 */
function Toggle({
  checked,
  partial,
  disabled,
  onChange,
  title,
}: {
  checked: boolean;
  partial: boolean;
  disabled?: boolean;
  onChange: () => void;
  title?: string;
}) {
  return (
    <input
      className="tcheck"
      type="checkbox"
      checked={checked}
      disabled={disabled}
      title={title}
      ref={(node) => {
        // 半选态：只传 checked 的话，React 不会帮你设 indeterminate
        if (node) node.indeterminate = !checked && partial;
      }}
      onChange={onChange}
      onClick={(event) => event.stopPropagation()}
    />
  );
}

/**
 * 按住工具栏拖动窗口。
 *
 * 这一页和启动器共用同一个无边框窗口，所以窗口控件得自己画。
 * `data-tauri-drag-region` 只认事件落在**带这个属性的元素本身** ——
 * 点在里面的文字上就不响应，用户感觉是"有时能拖有时不能"。所以自己接管：
 * 落点不在按钮/输入框/窗口控件上，就调系统拖动。
 */
function startWindowDrag(event: React.MouseEvent): void {
  if (event.button !== 0) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, a, input, select, textarea, .winctl")) return;
  void (async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // 浏览器演示模式没有这个能力，忽略
    }
  })();
}

export function IdeApp() {
  const [entries, setEntries] = useState<DevEntry[]>([]);
  const [truncated, setTruncated] = useState(false);
  const [extras, setExtras] = useState<string[]>([]);
  const [picked, setPicked] = useState<string[]>([]);
  const [expanded, setExpanded] = useState<string[]>([]);
  const [opened, setOpened] = useState<DevEntry[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [preview, setPreview] = useState<FilePreview | null>(null);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState(isDesktop ? "正在扫描游戏目录…" : "浏览器预览：没有 IPC，数据是示例");
  const [pkg, setPkg] = useState(loadPackageName);
  const [meta, setMeta] = useState({ author: "", version: "", description: "", tags: "", id: "", kind: "campaign", campaign: "", mainMap: "" });
  const [commits, setCommits] = useState<CommitRecord[]>([]);
  const [logPicked, setLogPicked] = useState<number | null>(null);
  const [pendingDiff, setPendingDiff] = useState<HistoryDiff | null>(null);
  const [message, setMessage] = useState("");
  const [maximized, setMaximized] = useState(false);

  // 双击标题栏 / Win+↑ 是系统改的最大化状态，只能靠 resize 补上
  useEffect(() => {
    if (!isDesktop) return;
    const sync = () => void launcher.windowIsMaximized().then(setMaximized).catch(() => {});
    sync();
    addEventListener("resize", sync);
    return () => removeEventListener("resize", sync);
  }, []);

  const renamePackage = (value: string) => {
    setPkg(value);
    try {
      localStorage.setItem(PACKAGE_KEY, value);
    } catch {
      // 隐私模式下存不了，当次有效就行
    }
  };

  const refresh = useCallback(async (extra: string[]) => {
    setStatus("正在扫描游戏目录…");
    const result = await scan(extra);
    if (!result) {
      setStatus("没扫到东西 —— 先在设置里指定星际争霸 II 的安装目录");
      return;
    }
    setEntries(result.entries);
    setTruncated(result.truncated);
    // 顶层默认展开，省得每次点开
    const tops = new Set(result.entries.map((entry) => entry.path.split("/")[0]).filter(Boolean));
    setExpanded((prev) => (prev.length ? prev : [...tops]));
    setStatus("扫到 " + result.entries.length + " 项" + (result.truncated ? "（已截断）" : ""));
  }, []);

  const refreshHistory = useCallback(async () => {
    if (!pkg.trim()) {
      setCommits([]);
      return;
    }
    setCommits(await loadHistory(pkg.trim()));
    setLogPicked(null);
    setPendingDiff(null);
  }, [pkg]);

  useEffect(() => {
    void refresh([]);
  }, [refresh]);

  useEffect(() => {
    void refreshHistory();
  }, [refreshHistory]);

  const open = useCallback(async (entry: DevEntry) => {
    setOpened((prev) => (prev.some((item) => item.abs === entry.abs) ? prev : [...prev, entry]));
    setActive(entry.abs);
    setStatus("正在读 " + entry.name + "…");
    const result = await readFile(entry.abs);
    setPreview(result);
    setStatus(
      result
        ? result.kind === "text"
          ? "行 " + result.lines + " · " + formatBytes(result.bytes)
          : result.kind === "image"
            ? "图片 · " + formatBytes(result.bytes)
            : "二进制 · 只读"
        : "读不了这个文件",
    );
  }, []);

  const addDirectory = useCallback(async () => {
    const dir = await pickDirectory();
    if (!dir) return;
    const next = [...extras, dir];
    setExtras(next);
    await refresh(next);
  }, [extras, refresh]);

  const toggleExpand = (path: string) =>
    setExpanded((prev) => (prev.includes(path) ? prev.filter((item) => item !== path) : [...prev, path]));

  const toggleFile = (path: string) =>
    setPicked((prev) => (prev.includes(path) ? prev.filter((item) => item !== path) : [...prev, path]));

  /**
   * 目录：整棵子树一起选 / 一起撤。
   *
   * 空目录（底下扫不到文件）直接不动 —— 以前会"点了没反应"，因为
   * `[].every(...)` 是 true，走进"全选"分支却什么也没得选。
   */
  const toggleDir = (node: TreeNode) => {
    const files = filesUnder(node);
    if (files.length === 0) return;
    const all = files.every((file) => picked.includes(file));
    setPicked((prev) =>
      all ? prev.filter((item) => !files.includes(item)) : [...new Set([...prev, ...files])],
    );
  };

  const tree = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    const roots = buildTree(entries);
    if (!keyword) return roots;

    // 搜索：留下命中的和它们的祖先
    const filter = (nodes: TreeNode[]): TreeNode[] =>
      nodes
        .map((node) => ({ ...node, children: filter(node.children) }))
        .filter(
          (node) =>
            node.name.toLowerCase().includes(keyword) ||
            node.path.toLowerCase().includes(keyword) ||
            node.children.length > 0,
        );
    return filter(roots);
  }, [entries, search]);

  const pickedBytes = useMemo(
    () => entries.filter((entry) => picked.includes(entry.path)).reduce((sum, entry) => sum + entry.bytes, 0),
    [entries, picked],
  );

  const maps = useMemo(
    () => picked.filter((path) => path.toLowerCase().endsWith(".sc2map")),
    [picked],
  );

  const submit = useCallback(async () => {
    if (!pkg.trim()) {
      setStatus("先给这个包起个名字");
      return;
    }
    if (!message.trim()) {
      setStatus("写一句这次改了什么再提交");
      return;
    }
    const files = entries
      .filter((entry) => picked.includes(entry.path))
      .map((entry) => ({ path: entry.path, abs: entry.abs }));
    const item = await commitVersion(pkg.trim(), message, null, files);
    if (!item) {
      setStatus(isDesktop ? "提交失败" : "浏览器预览提交不了 —— 没有 IPC");
      return;
    }
    setMessage("");
    await refreshHistory();
    setStatus("已提交 #" + item.id + "：" + item.files.length + " 个文件");
  }, [pkg, message, entries, picked, refreshHistory]);

  const exportNow = useCallback(async () => {
    if (!pkg.trim()) {
      setStatus("先给这个包起个名字");
      return;
    }
    if (picked.length === 0) {
      setStatus("一个文件都没勾");
      return;
    }
    const dest = await pickExportPath(pkg.trim() + ".zip");
    if (!dest) return;
    const payload: PackageMeta = {
      name: pkg.trim(),
      author: meta.author.trim() || null,
      version: meta.version.trim() || null,
      description: meta.description.trim() || null,
      campaign: meta.campaign.trim() || null,
      kind: meta.kind || null,
      id: meta.id.trim() || null,
      tags: meta.tags.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean),
      main_map: meta.mainMap || null,
    };
    const files = entries
      .filter((entry) => picked.includes(entry.path))
      .map((entry) => ({ path: entry.path, abs: entry.abs }));
    try {
      setStatus("正在打包…");
      const report = await exportPackage(dest, payload, files);
      setStatus("导出完成：" + report.files + " 个文件 · " + formatBytes(report.bytes) + " → " + report.path);
    } catch (error) {
      setStatus("导出失败：" + (error instanceof Error ? error.message : String(error)));
    }
  }, [pkg, picked, entries, meta]);

  const renderNode = (node: TreeNode, depth: number) => {
    const isDir = !node.entry || node.entry.is_dir;
    const files = isDir ? filesUnder(node) : [node.path];
    const selected = files.filter((file) => picked.includes(file)).length;
    const isOpen = !isDir || expanded.includes(node.path) || search.trim().length > 0;

    return (
      <div key={node.path} className="tnode">
        <div
          className={"trow2" + (active && node.entry && active === node.entry.abs ? " is-open" : "")}
          style={{ paddingLeft: 6 + depth * 14 }}
          // 点文件名打开它；点目录行就是展开 / 收起
          onClick={() => {
            if (isDir) {
              toggleExpand(node.path);
            } else if (node.entry) {
              void open(node.entry);
            }
          }}
        >
          {isDir && node.children.length > 0 ? (
            <button
              className="tcaret"
              type="button"
              title={isOpen ? "收起" : "展开"}
              aria-label={isOpen ? "收起" : "展开"}
              onClick={(event) => {
                event.stopPropagation();
                toggleExpand(node.path);
              }}
            >
              {isOpen ? "−" : "+"}
            </button>
          ) : (
            <span className="tcaret tcaret--leaf" />
          )}
          <Toggle
            checked={files.length > 0 && selected === files.length}
            partial={selected > 0 && selected < files.length}
            disabled={isDir && (files.length === 0 || truncated)}
            title={
              isDir
                ? files.length === 0
                  ? "这个目录里没有扫到文件"
                  : truncated
                    ? "扫描被截断了，整目录勾选可能漏文件 —— 请先缩小范围"
                    : "整棵子树一起选（" + files.length + " 个文件）"
                : "选它"
            }
            onChange={() => (isDir ? toggleDir(node) : toggleFile(node.path))}
          />
          <span className={"tname" + (node.entry?.external ? " tname--ext" : "")} title={node.path}>
            {node.name}
          </span>
          {node.entry?.external && <span className="tag tag--ext">外部</span>}
          <span className="tsize">{isDir && node.children.length > 0 ? formatBytes(bytesUnder(node)) : formatBytes(node.entry?.bytes ?? 0)}</span>
        </div>
        {isOpen && node.children.map((child) => renderNode(child, depth + 1))}
      </div>
    );
  };

  return (
    <div className="ide">
      <div className="tb" data-tauri-drag-region onMouseDown={startWindowDrag}>
        <Button onClick={() => (location.href = "/index.html")}>← 返回启动器</Button>
        <span className="tsep" />
        <label className="pkgnamectl" title="这个包的名字 —— 提交历史和导出都用它">
          <span>包名</span>
          <input
            value={pkg}
            onChange={(event) => renamePackage(event.target.value)}
            placeholder="给你的包起个名字"
            spellCheck={false}
          />
        </label>
        <span className="tsep" />
        <Button onClick={() => void refresh(extras)}>重新扫描</Button>
        <Button onClick={() => void addDirectory()}>添加目录…</Button>
        <span className="tspacer" />
        <Input value={search} onChange={(event) => setSearch(event.target.value)} placeholder="搜索" className="tsearch" />
        <Button primary disabled={!pkg.trim() || picked.length === 0} onClick={() => void exportNow()}>
          导出压缩包
        </Button>

        {/* 窗口按钮：这页用的是同一个无边框窗口，不画就没有最小化/关闭 */}
        <div className="winctl">
          <button className="winctl__btn" type="button" title="最小化" onClick={() => void launcher.windowMinimize()}>
            <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6h7" /></svg>
          </button>
          <button
            className="winctl__btn"
            type="button"
            title={maximized ? "还原" : "最大化"}
            onClick={() => void launcher.windowToggleMaximize().then(setMaximized).catch(() => {})}
          >
            {maximized ? (
              <svg viewBox="0 0 12 12" aria-hidden="true">
                <rect x="2.5" y="4.5" width="5" height="5" rx="1" />
                <path d="M4.5 4.5v-2h5v5h-2" />
              </svg>
            ) : (
              <svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2.5" y="2.5" width="7" height="7" rx="1.5" /></svg>
            )}
          </button>
          <button
            className="winctl__btn winctl__btn--close"
            type="button"
            title="关闭"
            onClick={() => void launcher.windowClose()}
          >
            <svg viewBox="0 0 12 12" aria-hidden="true">
              <path d="M3 3l6 6" />
              <path d="M9 3l-6 6" />
            </svg>
          </button>
        </div>
      </div>

      <div className="body">
        <nav className="rail">
          <button className="rbtn rbtn--on" title="游戏目录">
            <svg className="glyph" width={16} height={16} viewBox="0 0 16 16"><path d="M2.5 4.5h4l1.2 1.6h5.8v6.4h-11z" /></svg>
          </button>
        </nav>

        <div className="main">
          <div className="work">
            {/* ---------- 文件树 ---------- */}
            <section className="pane pane--tree">
              <header className="pbar">
                <span className="ptitle">游戏目录</span>
                <span className="psub">勾 {picked.length} 项 · {formatBytes(pickedBytes) || "0 B"}</span>
                <span className="ptools">
                  <Button small onClick={() => void refresh(extras)}>刷新</Button>
                </span>
              </header>
              <div className="pbody pbody--flat">
                {tree.length === 0 ? (
                  <p className="empty">
                    还没有内容。<br />
                    先确认设置页里指定了星际争霸 II 的安装目录，或者点上面「添加目录…」加一个进来。
                  </p>
                ) : (
                  tree.map((node) => renderNode(node, 0))
                )}
                {truncated && (
                  <p className="warnbox">
                    内容太多，扫描被截断了 —— 整目录勾选已禁用（勾了也会漏文件）。
                    请用右上角搜索缩小范围，或者「添加目录…」只加你要打包的那一层。
                  </p>
                )}
              </div>
            </section>

            {/* ---------- 编辑器 ---------- */}
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
                    <p className="uhint">大小 {formatBytes(preview.bytes)}</p>
                  </div>
                )}

                {preview?.kind === "image" && (
                  <div className="viewer">
                    <div className="stage"><div className="shot">{opened.find((item) => item.abs === active)?.name ?? "图片"}</div></div>
                    <p className="vbar"><span>{formatBytes(preview.bytes)}</span><span className="vzoom">适应窗口 · 100% · － ＋</span></p>
                  </div>
                )}
              </div>

              <footer className="status">
                <span>{status}</span>
                <span className="sspacer" />
                <span>{extras.length > 0 ? extras.length + " 个自定义目录" : ""}</span>
              </footer>
            </section>

            {/* ---------- 包信息 ---------- */}
            <section className="pane pane--meta">
              <header className="pbar"><span className="ptitle">包信息</span><span className="psub">导出时写进 metadata.json</span></header>
              <div className="pbody">
                <label className="field"><span>名称</span><input value={pkg} onChange={(event) => renamePackage(event.target.value)} placeholder="必填" /></label>
                <label className="field"><span>作者</span><input value={meta.author} onChange={(event) => setMeta({ ...meta, author: event.target.value })} placeholder="你的名字" /></label>
                <label className="field"><span>版本</span><input value={meta.version} onChange={(event) => setMeta({ ...meta, version: event.target.value })} placeholder="比如 1.0" /></label>
                <label className="field"><span>注册 ID</span><input value={meta.id} onChange={(event) => setMeta({ ...meta, id: event.target.value })} placeholder="同一个战役的多个版本靠它认亲" /></label>
                <label className="field"><span>标签</span><input value={meta.tags} onChange={(event) => setMeta({ ...meta, tags: event.target.value })} placeholder="逗号分开，比如 重制, 剧情" /></label>
                <label className="field"><span>归属战役</span>
                  <select value={meta.campaign} onChange={(event) => setMeta({ ...meta, campaign: event.target.value })}>
                    <option value="">自制战役（不依附官方）</option>
                    <option value="wol">自由之翼改版</option>
                    <option value="hots">虫群之心改版</option>
                    <option value="lotv">虚空之遗改版</option>
                    <option value="nova">诺娃隐秘行动改版</option>
                  </select>
                </label>
                <label className="field"><span>类型</span>
                  <select value={meta.kind} onChange={(event) => setMeta({ ...meta, kind: event.target.value })}>
                    <option value="campaign">campaign（战役本体）</option>
                    <option value="patch">patch（覆盖层）</option>
                  </select>
                </label>
                <label className="field field--full"><span>说明</span>
                  <textarea rows={3} value={meta.description} onChange={(event) => setMeta({ ...meta, description: event.target.value })} placeholder="一两句说清这个包是什么" />
                </label>
                <label className="field field--full"><span>主地图</span>
                  <select value={meta.mainMap} onChange={(event) => setMeta({ ...meta, mainMap: event.target.value })}>
                    <option value="">{maps.length ? "不指定" : "先勾选一张 .SC2Map"}</option>
                    {maps.map((path) => <option key={path} value={path}>{path}</option>)}
                  </select>
                </label>
                <p className="hintbox">
                  自制战役建议指定主地图 —— 它是玩家从启动器进游戏的入口。
                  包内路径保持游戏目录里的相对路径，导入时不用重新猜落点。
                </p>
                <p className="hintbox">
                  这个包会包含 <strong>{picked.length}</strong> 个文件，共 <strong>{formatBytes(pickedBytes) || "0 B"}</strong>。
                </p>
              </div>
            </section>
          </div>

          {/* ---------- 日志 ---------- */}
          <section className="pane pane--log">
            <header className="pbar">
              <span className="ptitle">日志</span>
              <span className="psub">{pkg.trim() ? (commits.length ? commits.length + " 次提交" : "还没有提交") : "先起个包名"}</span>
              <span className="ptools">
                <input
                  className="commitmsg"
                  value={message}
                  onChange={(event) => setMessage(event.target.value)}
                  placeholder="这次改了什么"
                />
                <Button small disabled={!pkg.trim() || picked.length === 0 || !message.trim()} onClick={() => void submit()}>
                  提交
                </Button>
              </span>
            </header>
            <div className="pbody pbody--flat logwrap">
              <ul className="tree">
                <li className="trow" onClick={() => { setLogPicked(null); setPendingDiff(null); }}>
                  <Graph color={PALETTE[0]} head />
                  <span className="tver">未提交</span>
                  <span className="tmsg">工作区里勾选的 {picked.length} 个文件</span>
                  <span className="refs">{picked.length > 0 ? <Tag>未提交</Tag> : null}</span>
                  <span className="twhen">现在</span>
                </li>
                {[...commits].reverse().map((item, index) => (
                  <li
                    key={item.id}
                    className={"trow" + (logPicked === item.id ? " is-picked" : "")}
                    onClick={async () => {
                      setLogPicked(item.id);
                      const previous = [...commits].reverse()[index + 1];
                      setPendingDiff(previous ? await diffVersions(pkg.trim(), previous.id, item.id) : null);
                    }}
                  >
                    <Graph color={PALETTE[index % PALETTE.length]} last={index === commits.length - 1} />
                    <span className="tver">{item.label ?? "#" + item.id}</span>
                    <span className="tmsg">{item.message || "（没写说明）"}</span>
                    <span className="refs">{index === 0 ? <Tag>最新</Tag> : null}</span>
                    <span className="twhen">{when(item.at)}</span>
                  </li>
                ))}
              </ul>
              <aside className="ldetail">
                {logPicked === null ? (
                  <>
                    <p className="ldtitle">工作区</p>
                    <p className="ldmsg">勾选 {picked.length} 个文件，共 {formatBytes(pickedBytes) || "0 B"}。</p>
                    <p className="ldmsg">提交记的是「这一版包含哪些文件」（路径 + 大小 + 修改时间），用来回答"跟上一版差在哪"。内容哈希要 SC2Diff。</p>
                  </>
                ) : (
                  (() => {
                    const item = commits.find((entry) => entry.id === logPicked);
                    if (!item) return null;
                    return (
                      <>
                        <p className="ldtitle">{item.label ?? "#" + item.id}</p>
                        <p className="ldmsg">{item.message || "（没写说明）"} · {when(item.at)}</p>
                        {pendingDiff && (
                          <p className="ldmsg">
                            比上一版：新增 {pendingDiff.added.length} · 改动 {pendingDiff.changed.length} · 删掉 {pendingDiff.removed.length}
                          </p>
                        )}
                        <p className="ldmsg">这一版有 {item.files.length} 个文件：</p>
                        <ul className="mini">
                          {item.files.slice(0, 8).map((file) => (
                            <li key={file.path}><span>{formatBytes(file.bytes)}</span>{file.path}</li>
                          ))}
                        </ul>
                      </>
                    );
                  })()
                )}
              </aside>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
