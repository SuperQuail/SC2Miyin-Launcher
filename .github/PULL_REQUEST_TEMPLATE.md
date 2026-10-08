<!-- 标题请用 Conventional Commits，例如：feat(core): 支持从注册表发现安装路径 -->

## 这个 PR 做了什么

<!-- 一两句话讲清动机与结果。改了解析逻辑就说清了改了什么。 -->

## 怎么验证的

<!-- 把真跑过的命令与结果贴上来。没跑过就写没跑过，别写"应该没问题"。 -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `pnpm -C ui build`（动了前端才需要）

```text
把命令输出贴这里
```

## 影响面

- [ ] 改了面向用户的文案（需要同步 `docs/`）
- [ ] 改了包格式（需要同步 `docs/package-format.md` 与 `MIYIN_FORMAT_VERSION`）
- [ ] 改了版本号（三处必须一致，见 AGENTS.md §15.1）
- [ ] 新增了依赖（需在描述里说明理由与替代方案）

## 截图 / 记录

<!-- 界面改动请附截图；解析类改动请附上用到的那几个真实样本与结果。 -->