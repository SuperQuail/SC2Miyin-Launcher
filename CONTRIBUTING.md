# 贡献指南

先说一句：**这个项目有一条流水线，改动必须走完它再提 PR。**

完整约定在 [AGENTS.md](AGENTS.md) —— 动手前请读一遍，尤其是：

- §6 编码规范（rustfmt / clippy 零告警 / 测试用 tempfile）
- §7 Git 规范（分支流向、Conventional Commits、发版）
- §10.5 文件写入安全（写盘闸门 + 先备份 + 先给用户看）
- §16 命令层一律 `#[tauri::command(async)]`
- §17 界面改动：先渲染，再定方案
- §18 双前端：用户侧 Vue，开发者侧 React（**不许互相 import**）

## 提交前必须跑

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

pnpm -C ui build
```

**没跑过就不要说"应该没问题"** —— PR 描述里请贴真跑过的输出。

## 分支与 PR

`feat/*` `fix/*` `chore/*` `docs/*` → `dev` → `main` → `release` → tag。

三个长期分支都开了保护，**只能走 PR**。PR 标题必须是 Conventional Commits：

```
feat(core): 从注册表发现星际争霸 II 安装路径
fix(install): 修正 CustomCampaigns 目录不存在时的崩溃
```

## 几条容易踩的坑

- **别提交游戏版权文件**（`.SC2Map` / `.SC2Mod` / `.SC2Bank` …）。测试样例放 `testdata/`，已忽略。
- **不要动 `reference/`**（scnexus 参考仓库，只读、已忽略）。
- **写盘相关改动**要想清楚三件事：会不会覆盖用户的文件、出不出游戏目录、出事能不能还原。
- 前端改动涉及布局/交互的，**先出渲染**再写实现（AGENTS.md §17）。

## 报告问题

用 [Issue 模板](.github/ISSUE_TEMPLATE) 提。带上：启动器版本、星际争霸 II 版本、复现步骤、
以及 `data/logs` 里对应的日志片段。
