#!/usr/bin/env python3
"""门禁：任何一份**原始代码**超过 500 行就告警。

为什么是告警而不是拒绝：行数只是个**信号**，不是判据 —— 有些文件天生就长
（核心测试、命令表）。但一份文件超过 500 行，多半已经在同时干好几件事，
那就是**高耦合**的味道：改它的一个地方要读懂另外九件事。

所以这里只做两件事：把超标的文件按 GitHub 注解打出来（点一下能跳到那一行），
再给一张按行数排的榜。**永远退出 0** —— 拦不拦是这个项目的人来定的。

用法：
    python scripts/check-file-size.py            # 默认 500
    python scripts/check-file-size.py 300        # 换个线，看看离得远不远
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

# 只看**原始代码**：编译产物、依赖、参考仓库一律不算
EXTENSIONS = {".rs", ".ts", ".tsx", ".vue", ".js", ".jsx", ".py", ".ps1", ".mjs"}
SKIP_DIRS = {
    ".git",
    "node_modules",
    "target",
    "dist",
    "reference",
    "assets-staging",
    # 界面方案预览件：本地草稿，不入库，不该占门禁的名额
    "preview",
}

DEFAULT_LIMIT = 500


def repo_root() -> Path:
    """仓库根 —— 以这个脚本所在的位置为准，别信当前目录。"""
    return Path(__file__).resolve().parent.parent


def tracked_files() -> list[Path]:
    """交给 git 列文件：它认识 .gitignore，我们不用自己维护一份跳过清单。"""
    try:
        listed = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=repo_root(),
            check=True,
            capture_output=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        # 没有 git（比如解压出来的源码包）就退化成自己走目录
        return [path for path in repo_root().rglob("*") if path.is_file()]

    return [repo_root() / name for name in listed.decode().split("\0") if name]


def main() -> int:
    limit = int(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_LIMIT
    root = repo_root()

    offenders: list[tuple[int, Path]] = []
    for path in tracked_files():
        if not path.is_file() or path.suffix.lower() not in EXTENSIONS:
            continue
        relative = path.relative_to(root)
        if any(part in SKIP_DIRS for part in relative.parts[:-1]):
            continue
        try:
            lines = len(path.read_text(encoding="utf-8", errors="replace").splitlines())
        except OSError:
            continue
        if lines > limit:
            offenders.append((lines, relative))

    offenders.sort(reverse=True)

    if not offenders:
        print(f"文件行数门禁：没有超过 {limit} 行的原始代码，干净。")
        return 0

    print(f"文件行数门禁：{len(offenders)} 份原始代码超过 {limit} 行 —— 高耦合风险，值得拆。")
    print("（行数只是信号：拆之前先想清楚它到底在干几件事，别为了凑数字而切碎）")
    print()
    for lines, relative in offenders:
        # GitHub 注解：CI 里会出现在 Files changed / 摘要上
        print(f"::warning file={relative.as_posix()}::这个文件 {lines} 行，超过 {limit} 行")
    print()
    print("按行数排：")
    for lines, relative in offenders:
        print(f"  {lines:>5}  {relative.as_posix()}")
    # 告警不拦路
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
