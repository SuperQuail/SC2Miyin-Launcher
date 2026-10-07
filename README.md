# SC2Miyin Launcher · 弥音启动器

> 用 Rust 写的《星际争霸 II》**战役与补丁管理器**。产品形态借鉴 [HMCL](https://github.com/HMCL-dev/HMCL) 的「启动器即管理中心」思路。

[![CI](https://github.com/SuperQuail/SC2Miyin-Launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/SuperQuail/SC2Miyin-Launcher/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/SuperQuail/SC2Miyin-Launcher?include_prereleases&label=release)](https://github.com/SuperQuail/SC2Miyin-Launcher/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## 它解决什么问题

玩家自制战役的现实是：**同一个战役有七八个版本、几十个补丁**，装第二个就得删第一个；
补丁要手动覆盖到 `Mods/`，想撤销只能重装；包格式五花八门，一大半根本不是 CCM 包。

| 痛点 | 这个启动器的做法 |
| --- | --- |
| 版本互相覆盖 | **战役库**：同一战役多版本共存，随时切换，库在软件目录里不污染游戏 |
| 补丁装了就撤不掉 | **分层合成**：补丁是覆盖层，不复制本体，按优先级叠加，关掉即撤销 |
| 包格式看不懂 | **任意打包形式**：zip / 7z / rar / tar… 直接导入，不用转格式 |
| 不知道是哪个战役的包 | **归属判定链**：元数据 → 包内路径 → 地图内依赖声明 → 地图文件名，认不出就问你 |

## 特性

- 🎯 **自动识别战役归属** —— 优先读包内元数据；没有元数据时读地图内部的依赖声明
  （暴雪自己的依赖系统，`.SC2Map` 里的 `DocumentHeader`）；再退回地图名前缀。
  **每一层都会告诉你是凭什么判的**，认不出来绝不瞎猜，会请你选。
- 📦 **兼容任意压缩格式** —— zip 原生读，7z / rar / tar / cpio 交给系统自带的 `tar`
  （Windows 10 1803+ 的 bsdtar/libarchive），找不到再试 7-Zip、UnRAR。
- 🧩 **补丁系统** —— 覆盖层 + 优先级 + 快捷开关。声明了依赖的补丁可自动匹配，
  没声明的只能手动挂（这是现实里的常态）。
- 💾 **省空间且可恢复** —— 补丁全程只存一份；铺进游戏的文件全部记账，撤下即还原。
  最坏情况是「多留几个文件」，而不是「官方战役没了」。
- 📤 **导出 CCM 兼容包** —— 我们自己的数据全放在 `Miyin/` 子目录里，
  CCM 读不到也不冲突；可以把补丁一起带走。
- ✏️ **元数据可编辑** —— 名称 / 作者 / 注册 ID / 描述都能改，只改启动器记录，不动包内容。
- 🎨 界面参考 HMCL 的视觉语言（Material 3 + 蓝色主色 + 大圆角卡片）。

## 快速开始

从 [Releases](https://github.com/SuperQuail/SC2Miyin-Launcher/releases) 下载
`SC2Miyin-Launcher-<版本>-win64.zip`，解压后双击 `SC2Miyin Launcher.exe`。

- **免安装**，绿色版：数据写在 exe 同级的 `data/`，整个文件夹拷走就换了台机器。
- 首次启动会自动从注册表找星际争霸 II；找不到就在「设置」里手动指定游戏根目录。
- **升级只替换 exe**，`data/` 留着，导入过的战役不会丢。
- 直接把压缩包**拖进窗口**也能导入。

## 从源码构建

`@bash
# 前端
pnpm -C ui install
pnpm -C ui build

# 桌面版（release 必须带 custom-protocol，否则不会内嵌前端）
cargo build --release -p miyin-launcher --features custom-protocol
# 产物：target/release/SC2Miyin Launcher.exe
`@

调试运行需要先起 dev server（debug 版连 `http://localhost:5183`）：

`@bash
pnpm -C ui dev            # 另开一个终端
cargo run -p miyin-launcher
`@

只跑前端（浏览器演示模式，自带示例数据，不需要桌面壳）：

`@bash
pnpm -C ui dev
`@

## 质量门禁

`@bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pnpm -C ui build            # 含 vue-tsc 类型检查
`@

CI 在 `dev` / `main` / `release` 三个分支上跑，见 [.github/workflows/ci.yml](.github/workflows/ci.yml)。

## 分支模型

| 分支 | 用途 |
| --- | --- |
| `dev` | 日常开发，CI 必须全绿 |
| `main` | 稳定分支，随时可发布 |
| `release` | 发版分支，只接受从 `main` 合并的提交 |
| `feat/*` `fix/*` | 特性与修复，合回 `dev` |

打 tag 触发发行：`git tag v0.1.0-alpha.1 && git push origin v0.1.0-alpha.1`

## 项目结构

`@text
crates/miyin-core/     领域核心：不含任何 GUI 依赖
  sc2/                 安装发现（注册表 / .build.info）
  campaign/            包格式：元数据、预检、归属判定、任意打包形式
  library/             战役库：多版本、补丁合成、启用回滚、导出
src-tauri/             桌面壳：只做状态持有与命令转发
ui/                    Vue 3 + TypeScript 前端
docs/                  格式规范与发行说明
`@

**依赖方向不可违反**：`miyin-core` ← `src-tauri` ← `ui`。
核心层不依赖 GUI、不弹窗、不自己去读配置目录。

## 文档

- [战役包 / 补丁包格式规范](docs/package-format.md) —— 兼容 CCM 与枢纽标准，
  扩展放在 `miyin` 命名空间
- [发行说明](docs/release-notes/)
- [AGENTS.md](AGENTS.md) —— 项目约定（含 SC2 领域知识与踩过的坑）

## 授权与合规

代码以 [MIT](LICENSE) 授权。

⚠️ **仓库内不含任何游戏版权资源**。`ui/public/campaigns/` 下的战役 key art 与
主 Logo 是 Blizzard 版权素材，仅作**标识性使用**；弥音立绘由项目作者提供。
详见 [ui/public/NOTICE.md](ui/public/NOTICE.md)。公开发布前请自行复核这部分的权利风险。

本项目不提供、不分发游戏本体，不绕过任何授权校验，也不内置盗版资源站。
