import type { CommitRecord } from "../api";
import { isDirNode, selectionUnder, type TreeNode } from "../lib/tree";
import type { ContextMenuItem } from "./parts/ContextMenu";

export type NodeMenuActions = {
  open: (node: TreeNode) => void;
  toggleNode: (node: TreeNode) => void;
  toggleExpand: (path: string) => void;
  copyPath: (node: TreeNode) => void;
  reveal: (node: TreeNode) => void;
  setCover: (abs: string) => void;
  setDoc: (abs: string) => void;
  close: () => void;
};

/** 文件树节点的右键菜单：打开 / 勾选 / 展开 / 复制路径 / 设为封面或说明书。 */
export function nodeMenuItems(
  node: TreeNode,
  expanded: string[],
  picked: string[],
  actions: NodeMenuActions,
): ContextMenuItem[] {
  const items: ContextMenuItem[] = [];
  const entry = node.entry;

  if (!isDirNode(node) && entry) {
    items.push({
      label: "打开",
      onSelect: () => {
        actions.open(node);
        actions.close();
      },
    });
  }
  // 整棵子树一起选 ←→ 一起撤（目录用半选态告诉人现在是哪一种）
  const every = selectionUnder(node).every((path) => picked.includes(path));
  items.push({
    label: every ? "取消勾选（含子目录）" : "勾选（含子目录）",
    onSelect: () => {
      actions.toggleNode(node);
      actions.close();
    },
  });
  if (isDirNode(node)) {
    items.push({
      label: expanded.includes(node.path) ? "收起" : "展开",
      onSelect: () => {
        actions.toggleExpand(node.path);
        actions.close();
      },
    });
  }
  items.push({
    label: "复制路径",
    onSelect: () => {
      actions.copyPath(node);
      actions.close();
    },
  });
  items.push({
    label: "在资源管理器里打开",
    onSelect: () => {
      actions.reveal(node);
      actions.close();
    },
  });
  if (/\.(png|jpg|jpeg|webp|gif)$/i.test(node.path) && entry?.abs) {
    items.push({
      label: "设为封面图",
      onSelect: () => {
        actions.setCover(entry.abs);
        actions.close();
      },
    });
  }
  if (/\.pdf$/i.test(node.path) && entry?.abs) {
    items.push({
      label: "设为说明书",
      onSelect: () => {
        actions.setDoc(entry.abs);
        actions.close();
      },
    });
  }
  return items;
}

export type CommitMenuActions = {
  apply: (item: CommitRecord) => void;
  copyMessage: (item: CommitRecord) => void;
  forget: (item: CommitRecord) => void;
  close: () => void;
};

/** 提交记录的右键菜单。 */
export function commitMenuItems(item: CommitRecord, actions: CommitMenuActions): ContextMenuItem[] {
  return [
    {
      label: "应用这一版（勾选 + 包信息）",
      onSelect: () => {
        actions.apply(item);
        actions.close();
      },
    },
    {
      label: "复制说明",
      onSelect: () => {
        actions.copyMessage(item);
        actions.close();
      },
    },
    {
      label: "删掉这条记录",
      danger: true,
      onSelect: () => {
        actions.forget(item);
        actions.close();
      },
    },
  ];
}

/** 空白处右键的兜底菜单 —— 浏览器原生菜单一律不露（AGENTS §18.2）。 */
export function bareMenuItems(close: () => void): ContextMenuItem[] {
  return [
    { label: "关闭菜单", onSelect: close },
    { label: "刷新页面", onSelect: () => location.reload() },
    {
      label: "返回启动器",
      onSelect: () => {
        location.href = "/index.html";
      },
    },
  ];
}
