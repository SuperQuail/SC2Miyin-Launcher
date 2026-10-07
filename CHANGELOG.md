# 更新日志

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)，提交信息遵循
[Conventional Commits](https://www.conventionalcommits.org/zh-hans/)。

发行说明的正文放在 `docs/release-notes/<tag>.md`，打 tag 时由 CI 直接引用。

## [未发布]

## [0.1.0-alpha.1] - 2026-10-07

第一个能用的版本。核心链路（导入 → 识别 → 启用 → 打补丁 → 导出）跑通并有测试覆盖。

### 新增

- **战役库**：同一战役多版本共存、随时切换；库在软件同级 `data/`，不污染游戏目录
- **任意打包形式**：zip / 7z / rar / tar / gz / bz2 / xz / zst / cab / iso 直接导入
  （借系统自带的 bsdtar/libarchive，零新依赖）
- **归属判定链**：元数据 → 包内镜像路径 → 地图内依赖声明 → 地图文件名；
  每层都记录依据，认不出来就让用户选，绝不瞎猜
- **补丁系统**：分层合成 + 优先级 + 快捷开关 + 依赖自动匹配 + 单独导出 + 元数据编辑
- **导出 CCM 兼容包**：额外数据放 `Miyin/` 子目录，CCM 读不到也不冲突，可带补丁
- **导入三层**：包内元数据（以数据为准）→ 证据链识别（尽力而为）→ 用户手动指定
- **版本与更新**：注册 ID、宽松版本比较、冲突时选覆盖更新或重命名后导入
- **拖拽导入**：压缩包拖进窗口任意位置即可
- **包格式规范**：[docs/package-format.md](package-format.md)

### 修复

- 包内相对目录的落点：进化地图与序章地图之前会被摆到父战役主目录，游戏找不到
- 补丁判定顺序：补丁包之前会先被问一遍「属于哪部战役」
- 空包（0 地图 0 模组）之前显示「可安装」，现在正确判为不可安装
- release 构建缺 `custom-protocol`，导致 exe 不内嵌前端、启动后白屏

### 已知限制

- 非 zip 格式依赖系统自带的 `tar.exe`
- 导出时「解开的目录树」形态的地图，CCM 可能读不了
- 地图内依赖声明的扫描是尽力而为，拿不到时会干净地下落到下一层

[未发布]: https://github.com/SuperQuail/SC2Miyin-Launcher/compare/v0.1.0-alpha.1...dev
[0.1.0-alpha.1]: https://github.com/SuperQuail/SC2Miyin-Launcher/releases/tag/v0.1.0-alpha.1
