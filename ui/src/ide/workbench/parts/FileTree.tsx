import Button from "@jetbrains/ring-ui-built/components/button/button";
import Checkbox from "@jetbrains/ring-ui-built/components/checkbox/checkbox";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";
import Tooltip from "@jetbrains/ring-ui-built/components/tooltip/tooltip";
import caretDownIcon from "@jetbrains/icons/caret-down";
import caretRightIcon from "@jetbrains/icons/caret-right";

import type { DevEntry } from "../../api";
import { formatBytes } from "../../lib/format";
import { bytesUnder, filesUnder, isDirNode, type TreeNode } from "../../lib/tree";

export type FileTreeProps = {
  tree: TreeNode[];
  /** 已勾选的路径 */
  picked: string[];
  expanded: string[];
  active: string | null;
  search: string;
  /** 每个节点"勾了它等于勾了哪些路径"—— 在 Workbench 里预计算好了 */
  selections: Map<string, string[]>;
  pickedBytes: number;
  onRefresh: () => void;
  onToggleExpand: (path: string) => void;
  onToggleNode: (node: TreeNode) => void;
  onToggleFile: (path: string) => void;
  onOpen: (entry: DevEntry) => void;
  onContextMenu: (node: TreeNode, x: number, y: number) => void;
};

/** 左栏：游戏目录的文件树（勾选 / 展开 / 打开 / 右键）。 */
export function FileTree({
  tree,
  picked,
  expanded,
  active,
  search,
  selections,
  pickedBytes,
  onRefresh,
  onToggleExpand,
  onToggleNode,
  onToggleFile,
  onOpen,
  onContextMenu,
}: FileTreeProps) {
  const renderNode = (node: TreeNode, depth: number) => {
    const isDir = isDirNode(node);
    const selection = selections.get(node.path) ?? [node.path];
    const files = filesUnder(node);
    const selected = selection.filter((path) => picked.includes(path)).length;
    const isEmpty = isDir && files.length === 0;
    const isOpen = !isDir || expanded.includes(node.path) || search.trim().length > 0;

    return (
      <div key={node.path} className="tnode">
        <div
          className={
            "trow2" +
            (active && node.entry && active === node.entry.abs ? " is-open" : "") +
            (isEmpty ? " trow2--empty" : "")
          }
          title={isEmpty ? "这个目录里没有文件（可能只有空壳子目录），所以没什么可勾的" : node.path}
          style={{ paddingLeft: 6 + depth * 14 }}
          // 点文件名打开它；点目录行就是展开 / 收起
          onClick={() => {
            if (isDir) {
              onToggleExpand(node.path);
            } else if (node.entry) {
              onOpen(node.entry);
            }
          }}
          onContextMenu={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onContextMenu(node, event.clientX, event.clientY);
          }}
        >
          {isDir && node.children.length > 0 ? (
            <Button
              className="tcaret"
              icon={isOpen ? caretDownIcon : caretRightIcon}
              title={isOpen ? "收起" : "展开"}
              onClick={(event) => {
                event.stopPropagation();
                onToggleExpand(node.path);
              }}
            />
          ) : (
            <span className="tcaret tcaret--leaf" />
          )}
          {/* 勾选框：目录要显示"部分选中"，所以用 Ring UI Checkbox 的 indeterminate */}
          <Tooltip
            title={
              isDir
                ? "整棵子树一起选（" + files.length + " 个文件" + (isEmpty ? "，这是个空目录" : "") + "）"
                : "选它"
            }
            onClick={(event) => event.stopPropagation()}
          >
            <Checkbox
              containerClassName="tcheck"
              checked={selection.length > 0 && selected === selection.length}
              indeterminate={selected > 0 && selected < selection.length}
              onChange={() => (isDir ? onToggleNode(node) : onToggleFile(node.path))}
            />
          </Tooltip>
          {isEmpty && (
            <Tooltip title="里面一个文件都没有（但空目录本身可以打包）">
              <Tag>空</Tag>
            </Tooltip>
          )}
          <span className={"tname" + (node.entry?.external ? " tname--ext" : "")} title={node.path}>
            {node.name}
          </span>
          {node.entry?.external && <Tag tagType={Tag.Type.WARNING}>外部</Tag>}
          <span className="tsize">
            {isDir && node.children.length > 0 ? formatBytes(bytesUnder(node)) : formatBytes(node.entry?.bytes ?? 0)}
          </span>
        </div>
        {isOpen && node.children.map((child) => renderNode(child, depth + 1))}
      </div>
    );
  };

  return (
    <section className="pane pane--tree">
      <header className="pbar">
        <span className="ptitle">游戏目录</span>
        <span className="psub">勾 {picked.length} 项 · {formatBytes(pickedBytes) || "0 B"}</span>
        <span className="ptools">
          <Button onClick={onRefresh}>刷新</Button>
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
      </div>
    </section>
  );
}
