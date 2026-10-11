## 这个 PR 做了什么

**推进类 PR：把 `dev` 合到 `main`**（发 `v0.1.0a7` 的第二步，见 AGENTS §7.3）。

这一程的内容就是刚合进 dev 的那个热修：

- **#27** `fix(core)`：非同盘安装能用了 —— 跨卷搬文件不再用 `fs::rename`（只在本卷有效），
  改成「同卷 rename，跨卷退回复制+删源」，并保证「不留半个目标、删源只在复制成功之后」。
- **#28** `docs(release)`：发行说明模板 + 按模板重排的 `v0.1.0a7.md`
  （这份文件会被 CI 当作 a7 的 Release 正文）。

## 怎么验证的

- [x] CI 在 #27 上全绿（前端构建 / 核心测试 ubuntu+windows / 桌面端 Windows / 标题 / 流向）
- [x] 本机：`cargo fmt --check` 无输出 · `clippy -D warnings` 零告警 · `cargo test` **177 + 6 全过**
- [x] **真跨卷实测**：`MIYIN_CROSS_VOLUME=F:\` 下 `moves_across_volumes` 通过（先是断言裸 rename 失败）

```text
跨卷移动验证通过："C:\Users\ROG\AppData\Local\Temp\.tmpQvjM30\pack" -> "F:\.tmpwbZADx\pack"
test platform::tests::moves_across_volumes ... ok
```

## 影响面

- [x] 改了版本号：`0.1.0-alpha.6 → 0.1.0-alpha.7`（三处一致，见 §15.1）
- [ ] 改面用户文案 / 包格式 / 新增依赖 —— 都没有
- 没有前端改动

## 合并方式

⚠️ **请用 `--merge`，不要 `--squash`** —— 推进类 PR 一 squash，`dev` 与 `main` 的历史就分叉了，
之后每次推进会冲突（§7.3 记着这个坑）。

## 接下来的顺序

1. **#28**（发行说明模板）先合 —— 否则 a7 的 Release 正文是重排前的版本
2. **本 PR** 合入 `main`
3. 再开 `main → release` 并合入
4. 从 `release` 打 tag `v0.1.0a7` 并推送 —— CI 会自动出包并附到 Release

> 按 AGENTS §7.1：**合并由维护者自己点。**
