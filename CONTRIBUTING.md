# 贡献指南

感谢愿意搭把手。这个项目的约定尽量少而明确，看完这一页就够动手了。

## 提之前

- **缺陷**：用 [Bug report](https://github.com/SuperQuail/SC2Miyin-Launcher/issues/new?template=bug_report.yml)，
  填上版本号与出问题的包名。导入类问题十有八九看 `data/library.json` 就能定位。
- **功能建议**：用 [Feature request](https://github.com/SuperQuail/SC2Miyin-Launcher/issues/new?template=feature_request.yml)，
  说清楚场景比给方案更有用。
- **不确定算不算 bug**：去讨论区聊。

## 分支与 PR

```text
feat/xxx ─┐
fix/xxx  ─┼─> dev ─> main ─> release ─> tag
chore/xx ─┘
```

- **不要直接往 `dev` / `main` / `release` 推**，三个分支都开了保护，走 PR。
- 分支名：`feat/<简短描述>`、`fix/<简短描述>`、`chore/<简短描述>`。
- **PR 标题必须是 Conventional Commits**（CI 会拦）：

  ```text
  feat(core): 支持从注册表发现安装路径
  fix(install): 修正 CustomCampaigns 目录不存在时的崩溃
  ```

  `type` 只能是 `feat` `fix` `chore` `refactor` `docs` `test` `build` `ci` `perf`，
  主题行不超过 80 字符。
- **目标分支要选对**：特性分支 → `dev`；`dev` → `main`；`main` → `release`。
  CI 会校验流向，跳级或倒流都会被拒。

## 动手前必读

[AGENTS.md](AGENTS.md) 是本仓库的项目约定，**改代码前请完整读一遍**。
里面记了架构、包格式、以及一堆踩过的坑（比如为什么 release 构建必须带
`--features custom-protocol`、为什么代理要显式 `no_proxy()`）。

## 提交前自测

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm -C ui build          # 动了前端才需要
```

四条都得过。PR 里请把**实际输出**贴上来，没跑过就写没跑过 ——
「应该没问题」这种话帮不上忙。

## 编码约定（摘要）

- 格式化一律 `rustfmt`，lint 零告警；需要豁免时 `#[allow(...)]` 并就近写理由。
- 标识符、日志级别、crate 名用**英文**；面向用户的文案、注释、提交信息用**中文**。
- 用户环境导致的错误返回 `Result`，**不要** `unwrap()` / `panic!`。
- 涉及文件系统的测试一律用 `tempfile`，**不要**碰真实的星际争霸 II 目录。
- 修 bug 要顺手补一个能复现它的测试。
- 改动解析逻辑时同步改 `docs/package-format.md`。

## 不要提交的东西

- `reference/`（参考仓库）、`target/`、`node_modules/`、`dist/`
- 任何暴雪版权文件（`.SC2Map` / `.SC2Mod` / `.SC2Bank` 等）——
  测试样例放 `testdata/`（已忽略）

## 许可证

提交即表示同意以 [MIT](LICENSE) 授权你的贡献。