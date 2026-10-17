import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// 窗口按钮走启动器那套命令（bridge 是框架无关的纯 TS，两边共用 —— AGENTS.md §18）
import { api as launcher } from "../../api/bridge";
import Button from "@jetbrains/ring-ui-built/components/button/button";
import folderIcon from "@jetbrains/icons/folder";

import {
  type CommitRecord,
  type DevEntry,
  type ExportProgress,
  type FilePreview,
  cancelExport,
  closeWindow,
  isDesktop,
  pickDirectory,
  pickDocFile,
  readFile,
  scan,
} from "../api";
import { formatBytes } from "../lib/format";
import { EMPTY_META, loadSettings, saveSettings, type DevMeta } from "../lib/settings";
import { buildTree, selectionUnder, type TreeNode } from "../lib/tree";
import { bareMenuItems, commitMenuItems, nodeMenuItems } from "./menus";
import { ContextMenuLayer, MenuProvider, useContextMenu } from "./parts/MenuLayer";
import { EditorPane } from "./parts/EditorPane";
import { FileTree } from "./parts/FileTree";
import { HistoryPanel } from "./parts/HistoryPanel";
import { LeaveGuard } from "./parts/LeaveGuard";
import { MetaPane } from "./parts/MetaPane";
import { Toolbar } from "./parts/Toolbar";
import { useExport } from "./useExport";
import { useHistory } from "./useHistory";

/**
 * 版本管理（提交 / 日志）暂时**藏起来** —— 现在这套只记路径+大小+修改时间，
 * 给不了"内容回退"这种真正有用的能力，摆在那儿只会误导人。代码留着，
 * 等接上 SC2Diff 再打开。
 */
const SHOW_VERSION_CONTROL = false;

/** 包信息里那几个"要挑一个文件"的字段，记着现在挑的是哪一个。 */
type Picking = null | "cover" | "doc";

/**
 * 开发者页的工作台：挑文件 → 填包信息 → 导出压缩包；顺带记版本、看文件。
 *
 * 页面上的数据**全部来自真实扫描**，没有内置的示例战役 / 版本号 ——
 * 包名一开始是空的，写什么由用户决定（只记住"上次用的那个"）。
 * 界面按职责拆在 ./parts，导出与历史分别在 ./useExport、./useHistory，菜单项在 ./menus。
 */
export function Workbench() {
  const [entries, setEntries] = useState<DevEntry[]>([]);
  const [extras, setExtras] = useState<string[]>([]);
  const [picked, setPicked] = useState<string[]>(() => loadSettings()?.picked ?? []);
  const [expanded, setExpanded] = useState<string[]>([]);
  const [opened, setOpened] = useState<DevEntry[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [preview, setPreview] = useState<FilePreview | null>(null);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState(
    isDesktop ? "正在扫描游戏目录…" : "浏览器预览：没有 IPC，数据是示例",
  );
  const [pkg, setPkg] = useState(() => loadSettings()?.pkg ?? "");
  const [meta, setMeta] = useState<DevMeta>(() => loadSettings()?.meta ?? EMPTY_META);
  // 封面 / 说明书是从**别的目录**挑的文件，导出时按包内路径一起带上
  const [coverPath, setCoverPath] = useState(() => loadSettings()?.cover ?? "");
  const [docPath, setDocPath] = useState(() => loadSettings()?.doc ?? "");
  const [maximized, setMaximized] = useState(false);
  const [picking, setPicking] = useState<Picking>(null);
  /** 导出进度：写完几个 / 一共几个。null 表示没在导出。 */
  const [exportProgress, setExportProgress] = useState<ExportProgress | null>(null);
  /** 正在重新扫描 —— 刷新 / 重新扫描按钮转 loader */
  const [scanning, setScanning] = useState(false);
  /** 打包没完就想走 —— 问一句。null 表示没在问。 */
  const [leaveGuard, setLeaveGuard] = useState<null | "home" | "close">(null);

  /** 取消导出之后要去哪 —— 等导出真的收尾了再走。 */
  const pendingLeave = useRef<null | "home" | "close">(null);

  const busyExport = exportProgress !== null;
  const menu = useContextMenu();

  const { commits, logPicked, pendingDiff, message, submitting, setMessage, setLogPicked, setPendingDiff, submit, applyCommit, forget, pickCommit } =
    useHistory({ pkg, picked, entries, meta, coverPath, docPath, setStatus, setPkg, setMeta, setPicked });

  const exportNow = useExport({
    pkg,
    meta,
    entries,
    picked,
    coverPath,
    docPath,
    versionError: versionErrorFor(meta.version),
    setStatus,
    setExportProgress,
    pendingLeave,
  });

  /**
   * 打包没完就别走：一次导出几十秒到几分钟，中途走掉留下的是写了一半的包。
   *
   * **故意不用 Tauri 的 `onCloseRequested`** —— 一旦注册过它，这个窗口的关闭
   * 就交给 JS 决定了（Tauri 不再自己关），而注册是跟着窗口而不是跟着页面的：
   * 打开过一次开发者页，之后回到启动器点 X 也关不掉窗。踩过。
   * 现在拦两条路：页面内点 X / 点返回（自己弹框），以及浏览器级的刷新与关标签。
   */
  useEffect(() => {
    if (!busyExport) return;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    addEventListener("beforeunload", warn);
    return () => removeEventListener("beforeunload", warn);
  }, [busyExport]);

  /** 回启动器：打包没完先问一句。 */
  const leaveHome = () => {
    if (busyExport) {
      setLeaveGuard("home");
      return;
    }
    location.href = "/index.html";
  };

  /**
   * 空白处的右键菜单。
   *
   * 这个页面里的文件行 / 提交行各有自己的菜单（它们在自己的 onContextMenu 里
   * preventDefault 了）—— 这里只兜住**没被任何人处理**的右键：
   * 原生浏览器菜单一律不露（AGENTS §18.2）。
   */
  useEffect(() => {
    const onContextMenu = (event: MouseEvent) => {
      if (event.defaultPrevented) return;
      event.preventDefault();
      menu.open(event, bareMenuItems(menu.close));
    };
    addEventListener("contextmenu", onContextMenu);
    return () => removeEventListener("contextmenu", onContextMenu);
  }, [menu]);

  // 双击标题栏 / Win+↑ 是系统改的最大化状态，只能靠 resize 补上
  useEffect(() => {
    if (!isDesktop) return;
    const sync = () => void launcher.windowIsMaximized().then(setMaximized).catch(() => {});
    sync();
    addEventListener("resize", sync);
    return () => removeEventListener("resize", sync);
  }, []);

  // 包信息一变就存 —— 重开页面原样回来
  useEffect(() => {
    saveSettings({ pkg, meta, cover: coverPath, doc: docPath, picked });
  }, [pkg, meta, coverPath, docPath, picked]);

  const refresh = useCallback(async (extra: string[]) => {
    setStatus("正在扫描游戏目录…");
    setScanning(true);
    const result = await scan(extra);
    setScanning(false);
    if (!result) {
      setStatus("没扫到东西 —— 先在设置里指定星际争霸 II 的安装目录");
      return;
    }
    setEntries(result.entries);
    // 上次勾的文件里，已经不在的剔掉；还在的原样保留
    const known = new Set(result.entries.map((entry) => entry.path));
    setPicked((prev) => prev.filter((path) => known.has(path)));
    // 顶层默认展开，省得每次点开
    const tops = new Set(result.entries.map((entry) => entry.path.split("/")[0]).filter(Boolean));
    setExpanded((prev) => (prev.length ? prev : [...tops]));
    setStatus("扫到 " + result.entries.length + " 项");
  }, []);

  useEffect(() => {
    void refresh([]);
  }, [refresh]);

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

  /**
   * 关一个文档标签。
   *
   * 关掉的若是**当前选中**的那个，必须把选中与预览一起挪到相邻的一个 ——
   * 否则选中会停在一个已经关掉的条目上（Ring UI 的标签条会退化成高亮第一个，
   * 而预览还停在被关掉的那份内容上）。一个都不剩时才清空。
   */
  const close = useCallback(
    (entry: DevEntry) => {
      setOpened((prev) => prev.filter((item) => item.abs !== entry.abs));
      if (active !== entry.abs) return;

      const index = opened.findIndex((item) => item.abs === entry.abs);
      const neighbour = opened[index + 1] ?? opened[index - 1] ?? null;
      if (neighbour) {
        void open(neighbour);
      } else {
        setActive(null);
        setPreview(null);
      }
    },
    [active, opened, open],
  );

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

  const toggleNode = (node: TreeNode) => {
    const all = selectionUnder(node);
    const every = all.every((path) => picked.includes(path));
    setPicked((prev) => (every ? prev.filter((item) => !all.includes(item)) : [...new Set([...prev, ...all])]));
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

  /** 每个节点"勾了它等于勾了哪些路径"—— 预计算一遍，渲染里别再递归。 */
  const selections = useMemo(() => {
    const map = new Map<string, string[]>();
    const walk = (node: TreeNode): string[] => {
      const all = [node.path, ...node.children.flatMap(walk)];
      map.set(node.path, all);
      return all;
    };
    tree.forEach(walk);
    return map;
  }, [tree]);

  const pickedBytes = useMemo(
    () => entries.filter((entry) => picked.includes(entry.path)).reduce((sum, entry) => sum + entry.bytes, 0),
    [entries, picked],
  );

  const maps = useMemo(() => picked.filter((path) => path.toLowerCase().endsWith(".sc2map")), [picked]);

  /** 挑封面图 / 说明书 —— 它们不在游戏目录里，导出时一起打进去。 */
  const pickDoc = useCallback(async (kind: "cover" | "doc") => {
    setPicking(kind);
    const path = await pickDocFile(kind);
    setPicking(null);
    if (!path) return;
    if (kind === "cover") setCoverPath(path);
    else setDocPath(path);
  }, []);

  const nodeActions = {
    open: (node: TreeNode) => {
      if (node.entry) void open(node.entry);
    },
    toggleNode,
    toggleExpand,
    copyPath: (node: TreeNode) => {
      void navigator.clipboard?.writeText(node.entry?.abs || node.path);
      setStatus("路径已复制");
    },
    reveal: (node: TreeNode) => {
      if (node.entry?.abs) void launcher.revealPath(node.entry.abs).catch(() => {});
    },
    setCover: setCoverPath,
    setDoc: setDocPath,
  };

  const commitActions = {
    apply: applyCommit,
    copyMessage: (item: CommitRecord) => {
      void navigator.clipboard?.writeText(item.message || "");
      setStatus("说明已复制");
    },
    forget: (item: CommitRecord) => void forget(item),
  };

  const canExport = !!pkg.trim() && picked.length > 0 && !versionErrorFor(meta.version);

  return (
    <MenuProvider value={menu}>
      <div className="ide">
        <Toolbar
          pkg={pkg}
          onPkg={setPkg}
          onHome={leaveHome}
          onRescan={() => void refresh(extras)}
          onAddDirectory={() => void addDirectory()}
          search={search}
          onSearch={setSearch}
          canExport={canExport}
          exporting={busyExport}
          scanning={scanning}
          onExport={() => void exportNow()}
          maximized={maximized}
          onMinimize={() => void launcher.windowMinimize()}
          onToggleMaximize={() => void launcher.windowToggleMaximize().then(setMaximized).catch(() => {})}
          onClose={() => (busyExport ? setLeaveGuard("close") : void closeWindow())}
        />

        <div className="body">
          <nav className="rail">
            <Button icon={folderIcon} active title="游戏目录" />
          </nav>

          <div className="main">
            <div className="work">
              <FileTree
                tree={tree}
                picked={picked}
                expanded={expanded}
                active={active}
                search={search}
                selections={selections}
                pickedBytes={pickedBytes}
                scanning={scanning}
                onRefresh={() => void refresh(extras)}
                onToggleExpand={toggleExpand}
                onToggleNode={toggleNode}
                onToggleFile={toggleFile}
                onOpen={(entry) => void open(entry)}
                onContextMenu={(node, event) => menu.open(event, nodeMenuItems(node, expanded, picked, nodeActions))}
              />

              <EditorPane
                opened={opened}
                active={active}
                preview={preview}
                status={status}
                exportProgress={exportProgress}
                extras={extras}
                onOpen={(entry) => void open(entry)}
                onClose={close}
              />

              <MetaPane
                pkg={pkg}
                onPkg={setPkg}
                meta={meta}
                onMeta={setMeta}
                versionError={versionErrorFor(meta.version)}
                coverPath={coverPath}
                docPath={docPath}
                pickingCover={picking === "cover"}
                pickingDoc={picking === "doc"}
                onPickCover={() => void pickDoc("cover")}
                onPickDoc={() => void pickDoc("doc")}
                maps={maps}
                pickedCount={picked.length}
                pickedBytes={pickedBytes}
              />
            </div>

            {/* ---------- 日志（暂时藏起来，见 SHOW_VERSION_CONTROL）---------- */}
            {SHOW_VERSION_CONTROL && (
              <HistoryPanel
                pkg={pkg}
                commits={commits}
                picked={picked}
                pickedBytes={pickedBytes}
                message={message}
                onMessage={setMessage}
                onSubmit={() => void submit()}
                submitting={submitting}
                logPicked={logPicked}
                pendingDiff={pendingDiff}
                onPickWorktree={() => {
                  setLogPicked(null);
                  setPendingDiff(null);
                }}
                onPick={pickCommit}
                onContextMenu={(item, event) => {
                  setLogPicked(item.id);
                  menu.open(event, commitMenuItems(item, commitActions));
                }}
                onApply={applyCommit}
              />
            )}
          </div>
        </div>

        {leaveGuard && (
          <LeaveGuard
            done={exportProgress?.done ?? 0}
            total={exportProgress?.total ?? 0}
            onStay={() => setLeaveGuard(null)}
            onLeave={() => {
              pendingLeave.current = leaveGuard;
              setLeaveGuard(null);
              void cancelExport();
            }}
          />
        )}

        <ContextMenuLayer />
      </div>
    </MenuProvider>
  );
}

/**
 * 版本号校验。
 *
 * 约定里版本号是自由字符串（`1.2` / `v8.0.1` / `1.0.2a2` 都行），但**必须能比** ——
 * 导入时判断"这是新版本还是旧版本"靠的是抽数字段比较。不写版本号没关系，
 * 写了就得让人能比。规则写在 docs/package-format.md §1.2。
 */
function versionErrorFor(version: string): string {
  const value = version.trim();
  if (!value) return "";
  if (!/^[A-Za-z0-9._+-]+$/.test(value)) return "只能用字母、数字和 . - _ +";
  if (!/[0-9]/.test(value)) return "至少要有一个数字 —— 不然导入时没法判断新旧";
  if (value.length > 32) return "太长了（32 个字符以内）";
  return "";
}
