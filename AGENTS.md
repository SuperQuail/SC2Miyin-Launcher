# AGENTS.md — 弥音启动器 (MiYin Launcher)

> 本文件是本仓库的**项目约定**，面向 AI 编码代理与人类协作者。
> **动手改代码前请先完整阅读本文档**；文档与代码冲突时，以代码为准并顺手修正文档。

---

## 1. 项目标识

| 项 | 值 |
| --- | --- |
| 项目名 | **SC2Miyin Launcher**（仓库 / 发行物名） |
| 中文名 | 弥音启动器 |
| 英文名 | MiYin Launcher |
| 组织 | [SuperQuail](https://github.com/SuperQuail) |
| 仓库 | https://github.com/SuperQuail/SC2Miyin-Launcher |
| 当前版本 | **`0.1.0a2`**（对外写法；`Cargo.toml` 里是 `0.1.0-alpha.2`，见 §15.1） |
| 许可证 | MIT |
| 曾用名 | HSCL / Hello StarCraft Launcher（历史名称，仅本地目录仍在用） |
| 本地目录 | `D:\Code\Rust\HSCL`（目录名暂未随改名调整，避免破坏现有工作流） |
| 语言 | **Rust**（核心）+ 前端技术栈（见 §3） |
| 目标平台 | **Windows 优先**（星际争霸 II 仅 Windows / macOS 有客户端，本机为 Windows 国服客户端） |
| 状态 | 🚧 早期开发中：**战役扫描 / 预检 / 安装 / 卸载**与桌面界面已可运行（见 §12.1） |

---

## 2. 项目定位

**一句话**：用 Rust 编写的《星际争霸 II》**战役与 Mod 管理器 / 启动器**，产品形态借鉴 **HMCL（Hello Minecraft! Launcher）** 的"启动器即管理中心"思路。

### 2.1 目标能力（规划）

- **游戏发现与配置**：自动从注册表发现 SC2 安装位置，支持手动指定；校验安装完整性并读取版本号。
- **战役管理**：扫描、安装、启用/禁用、卸载自制战役；兼容 **CCM（Custom Campaign Manager）格式**与「星际枢纽」标准格式。
- **Mod / 自定义内容管理**：`Mods`、`Maps`、`Interfaces` 的安装与启停，依赖关系提示。
- **存档管理**：`Documents\StarCraft II\Banks`（战役存档）与 `ArcadeBanks`（大厅存档）的备份、还原、导入导出。
- **多配置 / 目录隔离**：借鉴 HMCL 的"版本隔离"思路，为不同战役组合提供互不污染的运行环境。
- **启动链控制**：按需启动客户端 / 编辑器 / 切换器，可传自定义启动参数。
- **资源下载**：可配置镜像与代理、断点续传、哈希校验。
- **诊断**：读取游戏日志、崩溃信息，给出可读的问题定位。

### 2.2 明确的非目标

- ❌ 不提供、不分发游戏本体或任何暴雪版权资源。
- ❌ 不实现账号体系绕过、不逆向游戏授权校验。
- ❌ 不内置盗版地图/战役资源站。

---

## 3. 技术栈

### 3.1 推荐基线（✅ 已定，如需变更请同步修改本节）

| 层 | 选型 | 说明 |
| --- | --- | --- |
| 语言 | Rust `1.94+`，**edition 2024** | 本机已装 `cargo 1.94.0` / `rustc 1.94.0` |
| 异步运行时 | `tokio` | 下载、进程管理、并发扫描 |
| 错误处理 | `thiserror`（领域错误）+ `anyhow`（应用/CLI 边界） | 库层不吞错，边界层聚合上下文 |
| 日志 | `tracing` + `tracing-subscriber` | 结构化日志，落盘到应用数据目录 |
| 序列化 | `serde` + `serde_json` / `toml` | 配置、元数据 |
| Windows 集成 | `windows` 或 `winreg`、`sysinfo` | 注册表、进程探测 |
| 归档 | `zip`、`flate2`、`tar` | 按 CCM 实际打包格式补充 |
| HTTP | `reqwest`（rustls 优先） | 下载、更新检查 |
| 哈希 | `sha2`、`blake3` | 完整性校验 |
| GUI | **Tauri 2 + Vue 3 + TypeScript + Vite** | 见下方理由 |

**GUI 选型理由**：参考项目 scnexus 是 Vue 3 + Electron，采用 Tauri 可直接复用其 **Vue 组件组织 / Pinia store 划分 / IPC 契约**经验，同时把 Electron 换成 Rust 后端，安装包体积与内存占用大幅下降。
**备选**：`egui` / `iced`（纯 Rust、无 Web 工具链，适合追求极简依赖；代价是 UI 表现力与生态）。

### 3.2 依赖原则

- 优先标准库与成熟 crate；引入新依赖前说明理由与替代方案。
- 禁止引入已停止维护、或与项目许可证不兼容的依赖。
- **提交 `Cargo.lock`**（本项目是应用程序，不是库）。

---

## 4. 仓库结构（当前实现）

```text
HSCL/                          # 仓库根（目录名待后续统一为 miyin-launcher）
├── AGENTS.md                  # 本文件
├── Cargo.toml                 # workspace 根
├── crates/
│   └── miyin-core/            # 领域核心：不含任何 GUI 依赖
│       └── src/
│           ├── error.rs       # 统一错误类型（错误信息直接面向用户，用中文）
│           ├── safety.rs      # 路径白名单与规范化校验（所有写操作的闸门）
│           ├── sc2/           # 安装发现（注册表 / 手动）+ .build.info 解析
│           ├── campaign/      # 战役包领域（与游戏目录耦合的那部分）
│           │   ├── metadata.rs    # CCM metadata.txt / 标准 metadata.json 解析
│           │   ├── sanitize.rs    # 目录名安全化（防目录穿越）
│           │   ├── package.rs     # zip 预检：格式识别、zip-slip、体积上限、解压
│           │   ├── installer.rs   # 直接装进游戏目录（旧路径，保留）
│           │   └── scanner.rs     # 目录扫描与核对
│           └── library/       # **战役库**：多版本共存与切换（见 §14）
│               ├── mod.rs          # 索引模型、槽位、封面与路径规则
│               ├── store.rs        # 导入 / 删除版本
│               └── activation.rs   # 启用 / 停用（按清单精确回滚）
├── src-tauri/                 # Tauri 2 桌面壳：只做状态持有与命令转发
│   ├── src/lib.rs             # 全部 #[tauri::command] 都在这里
│   ├── tauri.conf.json
│   └── icons/                 # 由 scripts/prepare-icons.py 生成
├── ui/                        # Vue 3 + TypeScript 前端
│   ├── public/                # 美术资产（见 §13，含 NOTICE.md）
│   └── src/
│       ├── api/               # 类型定义 / 后端桥接（含浏览器演示模式）/ 美术映射
│       ├── components/        # SlotCard（战役卡片）、VariantCard（版本卡片）
│       ├── composables/       # useLauncher：全局状态与动作
│       ├── views/             # CampaignsView（战役列表）、SlotMenuView（战役菜单）、SettingsView
│       └── styles/            # tokens.css（设计令牌）、base.css
├── scripts/                   # prepare-assets.py / prepare-icons.py
├── docs/                      # 设计文档、格式说明
└── reference/                 # ⚠️ 只读参考仓库，已 gitignore，禁止提交
```

**依赖方向（不可违反）**：`miyin-core` ← `src-tauri` ← `ui`。
核心层**不得**依赖 GUI、不得弹窗、不得自己去读配置目录（路径由调用方传入）。

---

## 5. 常用命令

```bash
# Rust：构建与质量门禁（提交前必过）
cargo fmt --all                     # 格式化
cargo fmt --all -- --check          # 校验格式（CI 用）
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace              # 领域逻辑测试都在 miyin-core

# 前端
pnpm -C ui install
pnpm -C ui dev                      # 浏览器演示模式：无需桌面壳，自带示例数据
pnpm -C ui build                    # vue-tsc 类型检查 + 打包到 ui/dist

# 桌面版
#
# ⚠️ release 构建**必须**带 --features custom-protocol**，否则不会把 ui/dist 内嵌进二进制，
#   程序启动后会去连 devUrl（http://localhost:5183），在没有 dev server 的机器上
#   就是一片 ERR_CONNECTION_REFUSED。这个坑踩过一次。
pnpm -C ui build
cargo build --release -p miyin-launcher --features custom-protocol
# 产物：target/release/miyin-launcher.exe（绿色版，数据写 exe 同级的 data/）

# 调试运行（debug）才需要 dev server：
pnpm -C ui dev            # 另开一个终端
cargo run -p miyin-launcher

# 素材再生成（需要 Pillow）
python scripts/prepare-assets.py     # 原始素材 -> ui/public/
python scripts/prepare-icons.py      # 立绘 -> src-tauri/icons/
```

> 调试运行（debug）时 Tauri 会去连 `devUrl`（`http://localhost:5183`），
> 因此需要先在另一个终端执行 `pnpm -C ui dev`；release 构建则使用打包进二进制的前端。

---

## 6. 编码规范

- **格式**：一律 `rustfmt`（行宽 100，见 `.editorconfig`）；不要手工对齐、不要提交未格式化代码。
- **Lint**：`clippy -D warnings` 必须零告警；确实需要豁免时用 `#[allow(...)]` 并**就近写理由**。
- **命名**：遵循 Rust 惯例，标识符用英文；面向用户的文案用中文（后续走 i18n）。
- **文档注释**：对外可见的 `pub` 项必须有 `///` 注释；模块顶部写职责说明。
- **错误处理**：
  - 可恢复错误返回 `Result`，**禁止**用 `panic!`/`unwrap()` 处理用户环境问题（路径不存在、权限不足、文件损坏等）。
  - `unwrap()` / `expect()` 仅允许出现在测试、或已在上一行证明不可能失败的位置，并写明原因。
- **unsafe**：默认禁止。确需使用时必须附 `// SAFETY:` 说明前置条件。
- **路径处理**：统一用 `std::path`；注意 Windows 长路径（>260）与中文路径；不要用字符串拼接路径分隔符。
- **平台分支**：与平台相关的实现集中到独立模块（如 `platform/win.rs`），不要散落在业务逻辑里。
- **测试**：
  - 单元测试放同文件 `#[cfg(test)] mod tests`；集成测试放 `tests/`。
  - 涉及文件系统的一律用 `tempfile`，**禁止**在真实 SC2 安装目录里做测试。
  - 修复 Bug 时必须补一个能复现该 Bug 的测试。

---

## 7. Git 规范

### 7.1 分支模型

| 分支 | 用途 | CI |
| --- | --- | --- |
| `dev` | **日常开发**：所有特性与修复先合到这里 | 必跑 |
| `main` | **稳定分支**：只接受从 `dev` 合并的提交，随时可发布 | 必跑 |
| `release` | **发版分支**：只接受从 `main` 合并的提交，tag 从这里打 | 必跑 |
| `feat/<简短描述>` | 新功能，合回 `dev` | 跑 |
| `fix/<简短描述>` | 缺陷修复，合回 `dev` | 跑 |
| `chore/<简短描述>` | 构建、依赖、文档等杂项 | 跑 |

流向固定：`feat/*` → `dev` → `main` → `release` → tag。
**不要跳级合并，也不要直接往 `main` / `release` 推。**

### 7.2 提交信息 — Conventional Commits

格式：`<type>(<scope>): <subject>`（与参考项目 scnexus 的提交风格保持一致）

- `type`：`feat` | `fix` | `chore` | `refactor` | `docs` | `test` | `build` | `ci` | `perf`
- `scope`：模块名，如 `core`、`install`、`ui`、`ci`、`deps`
- 示例：
  - `feat(core): 从注册表发现星际争霸 II 安装路径`
  - `fix(install): 修正 CustomCampaigns 目录不存在时的崩溃`
  - `chore(deps): 升级 tauri 至 2.x`

**要求**：一个提交只做一件事；**禁止**把格式化改动与功能改动混在同一提交里；主题行不超过 72 字符。

### 7.3 发布

版本号遵循 SemVer；预发行用 **紧凑写法** `0.1.0a2`（内部是 `0.1.0-alpha.2`，见 §15.1）。

发版流程：

1. 改版本号，**三处必须一致**：
   `Cargo.toml` 的 `[workspace.package] version`、`ui/package.json`、
   `src-tauri/tauri.conf.json`。
2. 写发行说明 `docs/release-notes/v<版本>.md` —— CI 会把它作为 Release 正文，
   **文件名必须与 tag 完全对应**（`v0.1.0a2` 对 `v0.1.0a2.md`）。
3. 更新 `CHANGELOG.md`。
4. 从 `main` 合到 `release`，打 tag 并推送：

   ```bash
   git checkout release && git merge main && git push origin release
   git tag v0.1.0a2 && git push origin v0.1.0a2
   ```

5. `.github/workflows/release.yml` 自动构建 Windows 绿色版并附到 Release。
   版本号里带字母的（`0.1.0a2` / `0.1.0-alpha.2`）会**自动标成预发行**。

⚠️ **改完版本号务必确认文件仍是 UTF-8**：PowerShell 的 `Set-Content` 默认按 ANSI 写盘，
会把中文写成非法字节 —— `cargo` 会直接报 `path was not valid utf-8`。
用编辑器以 UTF-8 保存，或用 Python 脚本改（踩过一次）。

### 7.4 仓库卫生

- ❌ 不提交：`target/`、`node_modules/`、构建产物、`reference/`、本地配置（详见 `.gitignore`）。
- ✅ 提交：`Cargo.lock`、`.gitattributes`、`.editorconfig`、CI 配置。
- 行尾统一 LF（由 `.gitattributes` 保证），避免 Windows/Unix 贡献者产生噪声 diff。

---

## 8. 参考仓库 `reference/`

| 项 | 值 |
| --- | --- |
| 本地路径 | `reference/scnexus` |
| 上游 | https://github.com/MengLuoRJ/scnexus |
| 项目名 | 星际枢纽 / SCNexus（TypeScript + Electron + Vue 3） |
| 许可证 | BSD 3-Clause |
| Git 状态 | **已加入 `.gitignore`（`/reference/`），永远不会被提交** |

**用途**：仅作为**产品行为与文件格式的参考**，重点参考其：

- SC2 安装发现逻辑（注册表键、路径校验）
- CCM 自制战役的识别与解压流程（`packages/app-main/src/modules/campaign/ccm-process.ts`）
- 模块划分（`modules/{profile,campaign,customize,workshop,setting,common}`）
- IPC 契约与类型定义（`packages/app-shared/src/types/`）

**规矩**：

1. 可以读、可以学，**不要**把它的代码直接粘进 Rust 工程（语言不同，照抄只会带来坏设计）。
2. 移植其**算法与常量**时，在注释里写明来源文件与行号，便于人类复核。
3. 若将来确实需要大量复用其**代码**，需评估 BSD-3-Clause 的署名义务并在 `NOTICE` 中声明。
4. 保持只读：不要在 `reference/scnexus` 里改代码、提交、或安装依赖。

**更新参考仓库**：

```bash
git -C reference/scnexus pull        # 走系统代理，见 §9
git -C reference/scnexus log --oneline -20
```

---

## 9. 网络与代理

- 本机系统代理：**`http://127.0.0.1:7897`**（Clash 系，端口已在监听）。
- 已写入 Git 全局配置（`http.proxy` / `https.proxy`），因此 `git clone` / `git pull` 会**自动走代理**，一般无需额外设置。
- **crates.io 目前为直连**。若 `cargo fetch` 缓慢或超时：

  ```powershell
  $env:HTTPS_PROXY = "http://127.0.0.1:7897"
  $env:HTTP_PROXY  = "http://127.0.0.1:7897"
  cargo build
  ```
- ⚠️ **仓库内禁止硬编码代理地址**；代理属于用户配置项，应通过设置界面 / 配置文件注入。

---

## 10. 星际争霸 II 领域知识（本机已核实，2026-10）

> 以下路径与结构均在开发机（**国服 / 网易代理客户端**）上实际验证过，可直接作为实现依据。标注"推断"的条目需实测后再采信。

### 10.1 本机环境

| 项 | 值 |
| --- | --- |
| 安装根目录 | `D:\Game\BLZ\StarCraft II` |
| 版本 | `5.0.16.97579` |
| 分支 / 区域 | `cn`（网易 CDN：`blzdist-s2.necdn.leihuo.netease.com`） |
| 用户文档目录 | `C:\Users\<user>\Documents\StarCraft II` |

### 10.2 安装目录结构

```text
StarCraft II/
├── .build.info               # 构建元数据（管道分隔表，含 Version=5.0.16.97579、Branch=cn、CDN 地址）
├── .product.db / Launcher.db # 暴雪启动器数据库（二进制，勿改）
├── StarCraft II.exe          # 根启动器
├── StarCraft II Editor.exe   # 编辑器（32 位）
├── StarCraft II Editor_x64.exe
├── Support/                  # 32 位运行时
├── Support64/
│   └── SC2Switcher_x64.exe   # 版本切换器（启动器常用入口）
│   └── SC2Editor_x64.exe
├── Interfaces/               # 界面 Mod：Pro_2020 / Split_1v1 / Streamlined / WCS_3.0 等 .SC2Interface
├── SC2Data/                  # 游戏资源：config / data / ecache / indices / s2c
├── Versions/
│   └── Base<build>/          # 每个构建号一个目录，如 Base97579
│       ├── SC2.exe
│       └── SC2_x64.exe       # 真正的游戏客户端
├── Maps/                     # ⚠️ 首次使用自制内容时才创建
│   ├── Campaign/             # 官方战役地图
│   └── CustomCampaigns/      # CCM 自制战役根目录
└── Mods/                     # ⚠️ 同上，按需创建
```

### 10.3 关键结论

1. **安装发现**：注册表 `HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\StarCraft II` → `InstallLocation`（本机值 `D:\Game\BLZ\StarCraft II`，`DisplayName` 为 `星际争霸II`）。
   注意这是 **32 位注册表视图**，64 位进程需访问 `WOW6432Node` 或使用 `KEY_WOW64_32KEY`。
2. **安装校验**：以 `<root>\StarCraft II.exe` 是否存在为准（沿用 scnexus 的判定）。
3. **版本读取**：优先解析 `.build.info` 中的 `Version` 字段；`Versions\Base<build>` 目录名中的数字即构建号（97579）。
4. **启动链**（推断，需实测确认）：`StarCraft II.exe` → `Support64\SC2Switcher_x64.exe` → `Versions\Base<build>\SC2_x64.exe`。启动器侧建议以 `SC2Switcher_x64.exe` 作为可配置入口。
5. **`Maps` 与 `Mods` 目录可能不存在**：不能假设其存在，写入前必须按需创建（scnexus 亦有 `initCCMDirectory()` 这种专门逻辑）。
6. **用户文档目录** `Documents\StarCraft II` 实测包含：`Accounts`、`Maps`、`Screenshots`、`GameLogs`、`UserLogs`、`EditorLogs`、`EditorBackup`、`Interfaces`、`ImageUploads`、`Variables.txt`、`ExecuteInfo.txt`、`EditorVariables.txt`，以及形如 `<账号名>_<id>@<区域>.lnk` 的**账号快捷方式**。
   注意：**`Banks` / `ArcadeBanks` 仅在产生存档后出现**，代码里要按"可能缺失"处理。

### 10.4 战役地图路径（参考 scnexus 的命名习惯）

| 资料片 | 相对路径 |
| --- | --- |
| 自由之翼 WoL | `Maps\Campaign` |
| 虫群之心 HotS | `Maps\Campaign\swarm` |
| 虫群之心 · 进化 | `Maps\Campaign\swarm\evolution` |
| 虚空之遗 LotV | `Maps\Campaign\void` |
| 虚空之遗 · 序章 | `Maps\Campaign\voidprologue` |
| 诺娃隐秘行动 NCO | `Maps\Campaign\nova` |
| CCM 自制战役 | `Maps\CustomCampaigns` |

### 10.5 文件写入安全（务必遵守）

采用**路径白名单**策略（参考 scnexus 的 `PATH_WHITE_LIST`）：所有写/删操作的目标路径，必须校验落在以下白名单根内，且解析后（`canonicalize`）不得越界：

```
<game_root>/Maps
<game_root>/Mods
<game_root>/Interfaces
<game_root>/Maps/CustomCampaigns
<game_root>/Maps/Campaign/**           # 官方战役目录：默认只读，改前必须显式确认
<documents>/StarCraft II/Banks         # 存档操作
```

**硬性要求**：删除前打印并核对绝对路径；解析符号链接/junction 后再次校验；对可恢复的破坏性操作提供备份或回收站（`IFileOperation`）而非直接 `remove_dir_all`。

---

## 11. AI 编码代理工作准则

1. **先读后写**：动手前阅读本文件 + 目标模块；不清楚的 API 必须查证（`cargo doc`、crate 源码、官方文档），**禁止臆造函数签名**。
2. **改动闭环**：任何改动完成后，必须实际执行 `cargo fmt` / `cargo clippy` / `cargo test`，并在回复中**如实报告命令与结果**；没跑过就不要说"应该没问题"。
3. **最小改动**：不顺手重构无关代码，不擅自更改公共 API；需要时先说明再动手。
4. **引用可追溯**：参考 `reference/scnexus` 得出结论时，标注 `文件:行号`。
5. **破坏性操作**：删除 / 移动 / 覆盖前，先解析并打印绝对路径确认；不确定时先询问。
6. **不要提交**：`reference/`、构建产物、以及任何游戏版权文件（.SC2Map/.SC2Mod/.SC2Bank 等）——测试用样例请放 `testdata/`（已忽略）。
7. **语言**：与用户交流、写提交信息与文档用**中文**；标识符、日志级别、crate 名称用**英文**。
8. **不确定就说不确定**：把推断与事实分开陈述，不要用自信语气掩盖猜测。

---

## 12. 决策记录与待确认事项

### 12.1 已确认（2026-10-07）

| 决策 | 结论 |
| --- | --- |
| 项目名 | 中文「弥音启动器」/ 英文 **MiYin Launcher**（曾用名 HSCL 仅保留在本地目录名） |
| 技术栈 | **Rust + Tauri 2 + Vue 3 + TypeScript**（见 §3.1） |
| 版本管理 | Git，`main` 为稳定分支，提交遵循 Conventional Commits（见 §7） |
| 参考实现 | `reference/scnexus`（BSD-3-Clause，gitignored，**只读**，见 §8） |
| 代理 | 系统代理 `http://127.0.0.1:7897`，已配好 Git 全局代理（见 §9） |
| 脚手架 | Rust workspace（`crates/miyin-core`）+ Tauri 2 桌面壳 + Vue 3 前端（见 §4） |
| 功能范围 | 首个版本聚焦**战役管理与安装**（扫描 / 预检 / 安装 / 卸载），不含下载站与账号功能 |
| 美术风格 | 只参考 **HMCL 的视觉语言**（Material 3 + **蓝色主色** + 大圆角卡片），**不参考其布局**；看板娘为弥音立绘（见 §13） |
| 战役数据 | **存放在软件同级的 data 目录**（绿色版），支持同一战役多版本共存与自由切换（见 §14） |
| 主菜单范围 | 只列**四大战役**（自由之翼 / 虫群之心 / 虚空之遗 / 诺娃），进化与序章归并到父战役 |

### 12.2 待确认（TODO）

- [x] **许可证**：定为 **MIT**（宽松、便于被广泛集成）。HMCL 是 GPL-3.0、
      scnexus 是 BSD-3-Clause，本项目**没有**复用它们的代码。
- [x] **仓库命名**：`SuperQuail/SC2Miyin-Launcher`。
      本地目录 `HSCL` 暂不改名，避免破坏现有工作流。
- [x] **CI**：`.github/workflows/ci.yml`（核心测试双平台 / 前端构建 / 桌面端整工作区）。
- [x] **启用 / 停用战役（激活）**：已实现，按清单精确回滚（见 §14.11）。
- [ ] **游戏运行时探测**：识别 SC2 进程是否在运行，避免切换时文件被占用。
- [ ] **从游戏目录反向导入**：把已经装在 `Maps/CustomCampaigns` 里的旧战役收进库
      （`campaign::scanner` 已经能扫描这类目录，缺的是收编流程）。
- [x] **包格式规范**：已写成 `docs/package-format.md`（兼容 CCM 与枢纽标准 + 弥音扩展）。
- [ ] 包格式的**可视化说明**：给包作者一份带示例的打包指南（放 `docs/`）。
- [x] `README.md` / `LICENSE` / `CHANGELOG.md` / CI 工作流。
- [ ] `CONTRIBUTING.md` 与 Issue / PR 模板（欢迎外部贡献前补齐）。

---

## 13. 美术资产与合规

### 13.1 视觉风格

界面**只参考 HMCL 的视觉语言**（Material 3 色彩体系、**蓝色主色**、大圆角卡片、柔和阴影、模糊背景），
**不参考它的布局**。设计令牌集中在 `ui/src/styles/tokens.css`：改风格先改令牌，不要在组件里散写颜色。

### 13.2 素材来源与授权（重要）

| 素材 | 位置 | 授权状况 |
| --- | --- | --- |
| 战役 key art / 主 Logo / 背景 | `ui/public/campaigns/`、`ui/public/backdrop.jpg` | **Blizzard 版权素材**，仅作标识性使用 |
| 弥音立绘 | `ui/public/miyin/` | 项目作者提供 |

- 完整说明与再生成方法见 `ui/public/NOTICE.md`。
- **原始素材不入库**：`assets-staging/` 已加入 `.gitignore`，只有压缩后的产出会被提交。
- ⚠️ **公开发布前必须复核**：把 Blizzard 素材打包进公开仓库存在权利风险。
  若无法接受，替换 `ui/public/campaigns/*` 后重跑 `scripts/prepare-assets.py` 即可；
  界面在缺少素材时会退回渐变背景（`art.ts` 已做缺省处理）。

### 13.3 立绘使用位置

| 立绘 | 用途 |
| --- | --- |
| `miyin/wink.png` | 首页看板（封面看板娘） |
| `miyin/chibi.png` | 顶栏品牌头像、安装对话框 |
| `miyin/portrait.png` | 「关于」立绘，并用于生成应用图标 |
| `miyin/cry.png` | 空状态插画 |

立绘自带白底，界面统一用 `mask-image: radial-gradient(...)` 把方形边缘化开；
**不要**把立绘直接放在深色背景上而不加遮罩。

---

## 14. 战役库（多版本共存）

### 14.1 为什么放在软件目录而不是游戏目录

游戏目录里的 `Maps/CustomCampaigns` 只能放「当前这一份」，装第二个版本就得先删第一个。
所以库放在**软件可执行文件同级的 `data/` 目录**（绿色版，随软件走，见 `library::default_root`）：

```text
<启动器目录>/data/
├── library.json               # 索引：每个槽位下有哪些版本、当前启用哪个
├── active.json                # 激活清单：我们往游戏目录放了什么、挪走了什么
├── campaigns/<槽位>/<版本>/    # 各版本的完整内容
└── backup/<槽位>/              # 被挪走的官方文件，切回原版时原样还原
```

### 14.2 四大战役与归并

主菜单**只列四大战役**：自由之翼 → 虫群之心 → 虚空之遗 → 诺娃隐秘行动。
顺序由 `CampaignType::MAIN` 固定，**不要**改成按名字排序（那样虚空之遗会排到诺娃后面）。

「虫群之心 · 进化」与「虚空之遗 · 序章」**不作为独立条目**：
`CampaignType::parent()` 把它们归并到父战役。包内声明 `campaign=HOTSEVO` 的包
导入到「虫群之心」槽位，但 `Variant::target_sub` 记为 `swarm/evolution`，
启用时地图落到正确位置 —— **目标子目录属于版本，而不是槽位**，这是关键设计。

### 14.3 交互约定

- 战役列表是**卡片**；点进某个战役进入它**自己的菜单页**（不是弹窗），里面同样是**卡片**。
- 每个战役菜单里第一张卡永远是「原版战役」，其后是导入的玩家版本。
- **导入入口只有一处**：战役列表页的「＋ 导入战役包」。
  导入时按包内声明的资料片**自动判断归属**（`CampaignType::main_slot()`），
  认不出来才让用户选；导入成功后直接进入该战役的菜单页，能立刻看到新版本。
- 包可以自带封面图，规则见 `docs/package-format.md` §5；
  没有就用该战役的官方美术（`ui/src/api/art.ts` 的 `slotArt()`）。

### 14.4 包格式规范

**`docs/package-format.md` 是包格式的唯一权威**（v2：兼容 CCM 与枢纽标准，扩展放在 `miyin` 命名空间）。
改解析逻辑时必须同步改那份文档；格式语义变化要提升 `miyin.format`，
并同步 `miyin-core` 里的 `MIYIN_FORMAT_VERSION`。

### 14.5 载荷：地图与模组既可能是文件也可能是目录树

**这是本项目最容易踩的坑**：真实包里 `.SC2Map` / `.SC2Mod` 有两种载体 ——

- 单文件 MPQ：`paiur01.SC2Map`（一个 1 MB 的文件，CCM 平铺包就是这样）
- 解开目录树：`paiur01.SC2Map/Base.SC2Data/...`（镜像包就是这样）

`Payload` 用 `expanded` 区分，落盘时保持原形态。取"载荷根"的办法是
**路径里第一个以 `.SC2Map`/`.SC2Mod` 结尾的组件**（见 `payload_root`）。

**包内相对目录的含义取决于内容根站在哪里**（两个真实样本教我们的）：

- 内容根**已经落在**官方战役目录里（`X/swarm/metadata.txt`）→ 包内相对路径是该目录**之内**
  的结构，原样保留（`evolution/y.SC2Map` → `swarm/evolution/y.SC2Map`）
- 内容根是包根、顶层是**官方目录名**（`voidprologue/`）→ 当绝对路径用
- 内容根是包根、顶层是**任意分类目录**（`maps/`）→ 只取文件名，否则游戏扫不到

判错的后果：进化地图被摆到虫心主目录、序章地图被摆进虚空之遗主目录，游戏都找不到。

另一个坑：包内可能是**游戏目录镜像**（顶层有 `Maps/` 或 `Mods/`）。
这时内容根必须是**包根**，否则 metadata 所在的那一层会把同包的其它目录切掉 ——
真实案例：metadata 在 `Maps/Campaign/void/`，内容却还包含 `Maps/Campaign/voidprologue/`。
见 `has_mirror_root`。

### 14.6 补丁与版本

- **补丁 = 覆盖层**，不是战役。判定：元数据写明 `kind=patch`，或者
  **包里只有模组、一张地图都没有**（现实补丁几乎都是这样，见 `PATCH_INFERRED`）。
- **依赖**：`requires` 非空 = 完全补丁，可自动匹配战役；为空 = 只能手动指定（`PATCH_UNBOUND`）。
- **版本**：每个版本都有 `version` 与 `registration_id`（注册 ID，缺失时启动器生成）。
- **导入冲突**：往同一战役导同一个 ID（或同名）的版本时，`conflict_for` 会给出
  `Conflict`（含新旧版本对比 `VersionRelation`），界面据此让用户选
  **覆盖更新**（`ImportMode::Overwrite`，沿用原目录名，补丁绑定不受影响）
  还是 **重命名后导入**（`ImportMode::Rename`，两者并存）。

`compare_versions` 是宽松比较（抽数字段按数值比），认不出来返回 `None` —— **不要瞎猜版本新旧**。

### 14.7 非 CCM 包的归属判定

**大量玩家包根本不是 CCM 包**（没元数据、没目录结构，就是塞满地图的 zip）。
判定链在 `crates/miyin-core/src/campaign/identify.rs`，按可靠度从高到低：

1. 元数据 `campaign` 字段
2. 包内镜像路径 `Maps/Campaign/void/…`
3. **地图内部依赖声明** `Void Story (Campaign)` —— 读 `DocumentHeader`
4. 地图文件名前缀 `p*`（启发式，需要明显多数）

**第 3 层是精确证据，也是这个功能的价值所在**：SC2 的 `.SC2Map` 是 MPQ 归档，
完整解析成本高，但 `DocumentHeader` 是个 zlib 流 —— 展开形态直接读文件，
单文件形态扫流解压即可。实测 7 个有地图的样本里 5 个 8/8 命中、1 个 3/8，
**地图取任意文件名也能判对**。

拿不到时必须**干净地落到第 4 层**，绝不能把"扫不到"理解成"这包没有依赖"。

四层都认不出来就返回 `None`，由界面请用户指定。**不要加"猜一个"的兜底。**

`CampaignEvidence::is_exact()` 用来区分"精确证据"与"启发式"，
界面上要如实标注，别把猜的说成确定的。

### 14.8 任意打包形式与导入三层

- **容器不挑**：zip 原生读；7z / rar / tar / … 交给系统自带的 `tar`
  （Windows 10 1803+ 的 bsdtar/libarchive，一个工具覆盖一大片），
  找不到再试 7-Zip、UnRAR。见 `campaign/contents.rs`。
- **导入三层**：包内元数据（**以数据为准**）→ 证据链自动识别（**尽力而为**）→ 用户手动指定。
- **手动指定永远可用，且优先级最高**：`import_package` 传了 slot 就按传的来。
  界面上始终要有"导入到哪部战役"的选择，默认填自动识别的结果。
- 界面按 `ImportPreview.source` 措辞，**不许把"自动识别"说成确定的**。

### 14.9 打包发布

- **release 必须带 `--features custom-protocol`**（已写进 `src-tauri/Cargo.toml` 的
  `[features]`）。不带的话 Tauri 不会内嵌 `ui/dist`，程序会去连 devUrl，
  用户机器上没有 dev server -> 白屏 + ERR_CONNECTION_REFUSED。
- 绿色版：exe 与 `data/` 同级，整个文件夹拷走即可。
- 出包前先 `pnpm -C ui build`，否则内嵌的是旧前端。

### 14.11 切换的安全底线

启用 / 停用**只操作清单里记过的文件**，绝不递归删除官方目录：

1. `deactivate` 上一个版本：删掉我们放进去的文件、还原被挪走的官方文件。
2. 目标位置若已有同名文件（多半是官方的），先挪进 `data/backup/` 并记账。
3. 复制新版本的文件并逐条记账，最后写 `active.json`。

因此最坏情况是「多留了几个文件」，而不是「官方战役没了」。
这部分由 `crates/miyin-core/src/library/tests.rs` 的端到端测试覆盖。

---

## 15. 自动更新

实现在 `crates/miyin-core/src/update/`，四层分工：

| 模块 | 职责 |
| --- | --- |
| `version` | 版本比较（两种写法都认、且等价） |
| `mirror` | GitHub 镜像前缀与 URL 改写 |
| `net` | 代理自动选择 + **并发竞速**下载 |
| `check` | 查 Releases、挑最新、校验摘要 |
| `apply` | 解开更新包、生成替换脚本 |

### 15.1 版本号：内外两种写法

Cargo **只接受 semver**（`0.1.0a2` 会直接报 `unexpected character 'a' after patch version number`），
所以：

- `Cargo.toml` / `package.json` / `tauri.conf.json` 写 `0.1.0-alpha.2`
- **界面、git tag、发行说明**统一写 `0.1.0a2`

由 `update::version::compact()` 转换，`update::current_version()` 返回的是**对外写法**。
**单一事实来源永远是 `Cargo.toml`**，不要去别处手改版本号。

版本比较器必须让两种写法**等价**（`0.1.0a2 == 0.1.0-alpha.2`），
且 `a10 > a9` —— 否则 `a1`/`a2` 会被当成同一版，**永远检测不到更新**。

### 15.2 网络：三层代理 + 镜像竞速

代理按 **环境变量 → Windows 系统代理 → 直连** 自动挑；设置在 `data/network.json`。

两条**必须记住**的坑：

1. **reqwest 默认会自己读环境变量代理**。构建客户端时若不显式 `no_proxy()`，
   `proxy = None`（"直连" / "关掉代理" / "限流后改直连"）会被 `HTTPS_PROXY`
   悄悄拉回代理，这几条路全是**假的**。
2. **GitHub API 额度按出口 IP 算**。国内代理多是共享 IP，额度常被用光（实测撞过 403）。
   所以 `check()` 在代理被限流时会**自动改直连重试一次**，成功则标注"直连（代理被限流）"。

镜像竞速只用在**资产下载**上（镜像基本不代理 `api.github.com`）：
展开成「直连 + 6 个镜像」，并发开跑，谁先下完谁赢，其余看到赢家就立刻放弃。

### 15.3 底线

- **网络失败不算错误**：包成 `UpdateCheck::error` 返回，界面显示"检查失败"即可。
- **不确定就不动**：版本号认不出、摘要对不上、包里没有 exe、zip 条目越界 —— 一律拒绝。
- **更新只换程序本体**：`data/`（战役库、补丁、配置）一个字节都不碰。
- Windows 上覆盖不了正在运行的 exe，所以走 `.cmd` 重试脚本。
