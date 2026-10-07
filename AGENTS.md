# AGENTS.md — 弥音启动器 (MiYin Launcher)

> 本文件是本仓库的**项目约定**，面向 AI 编码代理与人类协作者。
> **动手改代码前请先完整阅读本文档**；文档与代码冲突时，以代码为准并顺手修正文档。

---

## 1. 项目标识

| 项 | 值 |
| --- | --- |
| 中文名 | 弥音启动器 |
| 英文名 | MiYin Launcher |
| 曾用名 | HSCL / Hello StarCraft Launcher（历史名称，仅本地目录仍在用） |
| 本地目录 | `D:\Code\Rust\HSCL`（目录名暂未随改名调整，避免破坏现有工作流） |
| 语言 | **Rust**（核心）+ 前端技术栈（见 §3） |
| 目标平台 | **Windows 优先**（星际争霸 II 仅 Windows / macOS 有客户端，本机为 Windows 国服客户端） |
| 状态 | 🚧 初始化阶段：仓库刚建立，尚无业务代码 |

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

## 4. 仓库结构（规划）

```text
HSCL/                          # 仓库根（目录名待后续统一为 miyin-launcher）
├── AGENTS.md                  # 本文件
├── README.md                  # 面向用户的项目介绍（待补）
├── LICENSE                    # 待定，见 §10
├── Cargo.toml                 # workspace 根
├── crates/
│   ├── miyin-core/            # 领域核心：SC2 安装发现、战役/Mod 模型、元数据解析
│   ├── miyin-install/         # 安装、卸载、启停、目录隔离
│   ├── miyin-net/             # 下载、镜像、校验、代理
│   ├── miyin-store/           # 配置与本地数据库持久化
│   ├── miyin-cli/             # 可选 CLI，便于无 GUI 调试与自动化测试
│   └── miyin-app/             # Tauri 应用入口（src-tauri）
├── ui/                        # 前端工程（若采用 Tauri）
├── docs/                      # 设计文档、格式说明、调研结论
├── reference/                 # ⚠️ 外部参考仓库，已被 .gitignore 忽略，禁止提交
│   └── scnexus/
└── .github/workflows/         # CI / 发布流水线（待补）
```

**依赖方向（不可违反）**：`*-core` ← `*-install` ← `miyin-app`；核心层**不得**依赖 GUI、不得直接弹窗或读环境变量做交互。

---

## 5. 常用命令

```bash
# 构建 / 运行
cargo build                       # debug 构建
cargo build --release             # 发布构建
cargo run -p miyin-cli -- --help  # 调试用 CLI

# 质量门禁（提交前必过）
cargo fmt --all                   # 格式化
cargo fmt --all -- --check        # 校验格式（CI 用）
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace

# 依赖审计 / 体积
cargo tree -d                     # 重复依赖
cargo bloat --release             # 二进制体积分析（可选）

# 前端（若采用 Tauri）
pnpm install
pnpm tauri dev
pnpm tauri build
```

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

| 分支 | 用途 |
| --- | --- |
| `main` | 稳定分支，随时可发布；**禁止**直接推送，走 PR |
| `dev` | 集成分支（可选，视协作规模启用） |
| `feat/<简短描述>` | 新功能 |
| `fix/<简短描述>` | 缺陷修复 |
| `chore/<简短描述>` | 构建、依赖、文档等杂项 |

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

- 版本号遵循 **SemVer**，打成 `vX.Y.Z` tag。
- 发布由 tag 触发 CI 构建 Windows 安装包（Tauri 的 NSIS/MSI 目标）并附到 GitHub Release。
- 首个公开版本前需补齐：`LICENSE`、`README.md`、`CONTRIBUTING.md`、Issue 模板、CI 工作流。

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

## 12. 待确认事项（TODO）

- [ ] **GUI 框架最终选型**：Tauri 2 + Vue 3（当前推荐基线）vs egui / iced 纯 Rust。
- [ ] **许可证**：HMCL 为 GPL-3.0、参考项目 scnexus 为 BSD-3-Clause；本项目需自行决定（若希望被广泛集成，MIT/Apache-2.0 更宽松）。
- [ ] **仓库 / 目录正式更名**：`HSCL` → `miyin-launcher`（含 crate 名、仓库名、CI 路径）。
- [ ] 首个可运行版本的功能边界（先做"发现 + 启动"还是"战役管理"）。
- [ ] CCM 格式规格文档整理（放 `docs/`）。
