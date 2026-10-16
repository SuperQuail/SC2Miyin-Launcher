import type { DevEntry } from "../api";

/** 树：目录节点没有 entry，文件节点有。 */
export type TreeNode = { name: string; path: string; entry?: DevEntry; children: TreeNode[] };

export function buildTree(entries: DevEntry[]): TreeNode[] {
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

/** 这是不是目录节点（没有 entry 的中间节点也算目录）。 */
export function isDirNode(node: TreeNode): boolean {
  return !node.entry || node.entry.is_dir;
}

/**
 * 勾一个节点 = 勾住**它自己 + 底下所有东西**（文件和目录都算）。
 *
 * 目录也算进去是为了**空目录**：作者可能就是要留一个空壳子，
 * 只搬文件的话那种目录根本进不了包。
 */
export function selectionUnder(node: TreeNode, out: string[] = []): string[] {
  out.push(node.path);
  node.children.forEach((child) => selectionUnder(child, out));
  return out;
}

/** 这个节点底下的全部文件（不含目录）—— 提交历史只记文件。 */
export function filesUnder(node: TreeNode, out: string[] = []): string[] {
  if (node.entry && !node.entry.is_dir) out.push(node.path);
  node.children.forEach((child) => filesUnder(child, out));
  return out;
}

export function bytesUnder(node: TreeNode): number {
  if (node.entry && !node.entry.is_dir) return node.entry.bytes;
  return node.children.reduce((sum, child) => sum + bytesUnder(child), 0);
}
