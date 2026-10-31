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
| 当前版本 | **看 `Cargo.toml`** —— 单一事实来源，本文不再抄一份（写法见 §15.1） |
| 许可证 | MIT |
| 曾用名 | HSCL / Hello StarCraft Launcher（历史名称，仅本地目录仍在用） |
| 本地目录 | `D:\Code\Rust\HSCL`（目录名暂未随改名调整，避免破坏现有工作流） |
| 语言 | **Rust**（核心）+ 前端技术栈（见 §3） |
| 目标平台 | **Windows 优先**（星际争霸 II 仅 Windows / macOS 有客户端，本机为 Windows 国服客户端） |
| 状态 | 🚧 早期开发中：战役库 / 补丁 / 导出 / **自动更新**已可用（见 §14、§15） |

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
| GUI | **Tauri 2 + React 18 + TypeScript + Vite** | 见下方理由 |

**GUI 选型理由**：参考项目 scnexus 是 Vue 3 + Electron，采用 Tauri 可复用其 **组件组织 / store 划分 / IPC 契约**经验，同时把 Electron 换成 Rust 后端，安装包体积与内存占用大幅下降。
**组件库**：用户侧用自建控件层（`ui/src/react/ui/`），开发者页用 JetBrains 的 Ring UI（见 §18）。
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
├── crates/miyin-core/         # 领域核心，不含任何 GUI 依赖
│   └── src/
│       ├── error.rs  safety.rs  # 统一错误 / 路径白名单与写盘闸门（§10.5）
│       ├── sc2/                 # 安装发现（注册表 / 手动）+ .build.info 解析
│       ├── campaign/            # 包：metadata / sanitize / package / contents / identify（§14.5-14.8）
│       ├── library/             # **战役库**：索引 / 合成 / 安装 / 激活 / 导出 / 命名 / 已知战役（§14）
│       ├── update/              # 自动更新：version / mirror / net / check / apply（§15）
│       └── saves/              # 存档备份与还原（§14.10）
├── src-tauri/                 # Tauri 2 桌面壳：状态持有 + 命令转发
│   ├── src/lib.rs             # AppState / run() / 命令注册（命令在各子模块，一律带 (async)，§16）
│   ├── src/commands/          # 按域拆开的 #[tauri::command]（campaign / mods / saves / …）
│   ├── src/platform.rs        # 平台实现：开浏览器、资源管理器选中、WebView2 调教、崩溃钩子
│   └── tauri.conf.json + icons/   # icons 由 scripts/prepare-icons.py 生成
├── ui/                        # React 18 前端，两个入口 —— 结构与边界见 docs/frontend.md
├── scripts/                   # prepare-assets.py / prepare-icons.py / check-file-size.py
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
pnpm -C ui build                    # tsc 类型检查 + 两个入口都打包到 ui/dist

# 门禁：任何一份原始代码超过 500 行就告警（不拦路）
python scripts/check-file-size.py
python scripts/check-file-size.py 300   # 换个线看看离得远不远

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
- **文件大小**：单份源码文件（**含内联测试**）不得超过 **800 行** —— 超过就是严重过耦合，必须拆。
  门禁：`python scripts/check-file-size.py 800`（只告警；提交前应为空，或你知道每一份为什么留着）。
  拆分时公共 API 用 `pub use` 原样转发，别顺手改调用方。

---

## 7. Git 规范

### 7.1 分支模型：一律走 PR

| 分支 | 用途 | 谁能合进来 |
| --- | --- | --- |
| `dev` | **日常集成**：所有改动先落这里 | `feat/*` `fix/*` `chore/*` 的 PR |
| `main` | **稳定分支**：随时可发布 | **只接受 `dev` 的 PR** |
| `release` | **发版分支**：tag 从这里打 | **只接受 `main` 的 PR** |

流向固定：`feat/*` → `dev` → `main` → `release` → tag。

> [!IMPORTANT]
> **除非用户明确说明，不得发布 PR。** 改动先待在分支上（推分支没问题），
> 开 PR 要等一句明确的话。这条是让「什么时候把一个改动摆上台面」由人决定。

> **代理不许代替维护者 merge。** PR 只能由维护者自己合 —— 代理负责的是：开 PR、
> 跑门禁、把**真跑过的**验证输出贴进 PR 描述，然后停下等。合不合、什么时候合，由人决定。
> （这条是用户明确定下的规矩：合并是"接收这份改动"的动作，只有人能做。）

> [!IMPORTANT]
> **不要每改一点就往远端推。** 提交先留在本地，等维护者说「推」的时候再**一次性**推上去。
> （2026-10 维护者明确：改完别顺手 push。代理的默认动作是本地提交，不是同步远端。）

> [!IMPORTANT]
> **不要直接往 `dev` / `main` / `release` 推。** 三个分支都开了分支保护，
> 只能通过 PR 合入。要临时绕过时用 `enforce_admins`（只有维护者能做，且别常用）。

推 PR 之后 CI 会拦两件事（见 `.github/workflows/pr.yml`）：

1. **PR 标题必须是 Conventional Commits**（规则见 §7.2，主题行 ≤ 80 字符）
2. **目标分支的流向必须合法** —— 不许跳级（如 `feat/*` → `main`），
   也不许倒流（如 `main` → `dev`）

PR 模板在 `.github/PULL_REQUEST_TEMPLATE.md`，请把**真跑过的**验证命令输出贴进去，
而不是写「应该没问题」。

**为什么改成 PR**：改动合入前得有人或 CI 看一眼。直接在 `main` 上改的话，
CI 红着也能进仓库，出了事只能靠回忆当时干了什么。

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
4. **按流向推 PR**（三个分支都受保护，推不上去）：

   ```bash
   # 1) 发版准备分支 -> dev
   gh pr create --base dev --head chore/release-0.1.0a4 --title "chore(release): 版本升到 0.1.0a4"
   gh pr merge <编号> --squash --delete-branch

   # 2) 逐级推进（这两步用 --merge，**不要**用 --squash）
   gh pr create --base main    --head dev  --title "chore(release): 合并 dev 到 main（0.1.0a4）"
   gh pr merge <编号> --merge
   gh pr create --base release --head main --title "chore(release): 合并 main 到 release（0.1.0a4）"
   gh pr merge <编号> --merge
   ```

5. 从 `release` 打 tag 并推送，`.github/workflows/release.yml` 会自动构建
   Windows 绿色版并附到 Release。版本号里带字母的（`0.1.0a2` / `0.1.0-alpha.2`）
   会**自动标成预发行**：

   ```bash
   git tag -a v0.1.0a4 origin/release -m "0.1.0a4"
   git push origin v0.1.0a4
   ```

⚠️ **推进类 PR 必须用 `--merge`，不能 `--squash`**。踩过的坑：一开始用 squash
把 `dev` 压成单个提交合进 `main`，两边历史就此分叉，之后每次 `dev` → `main`
都冲突。真碰上了就用 `git merge origin/main -s ours` 收口一次（内容以 dev 为准，
但把 main 记为祖先），再走 PR 推上去。

也正因为这样，**不要开「要求线性历史」** —— 它和「长分支互相推进」天生打架。

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

**用途**：只看**产品行为与文件格式** —— 安装发现（注册表键与校验）、CCM 包识别与解压
（`modules/campaign/ccm-process.ts`）、模块划分、IPC 契约与类型定义（`packages/app-shared/src/types/`）。

**规矩**：

1. 可以读、可以学，**不要**把它的代码粘进 Rust 工程（语言不同，照抄只会带来坏设计）。
2. 移植其**算法与常量**时，注释里写 `文件:行号`。
3. 大量复用其**代码**前，先评估 BSD-3-Clause 的署名义务并在 `NOTICE` 里声明。
4. 保持只读。更新：`git -C reference/scnexus pull`（走系统代理，见 §9）。

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

- `.build.info` 构建元数据（含 `Version=` / `Branch=` / CDN 地址）；`.product.db` / `Launcher.db` 是暴雪启动器的二进制库，**勿改**
- `StarCraft II.exe`（根启动器）· `StarCraft II Editor[_x64].exe` · `Support64/SC2Switcher_x64.exe`（版本切换器）
- `Interfaces/` 界面 Mod（`*.SC2Interface`）· `SC2Data/` 游戏资源（config / data / ecache / indices / s2c）
- `Versions/Base<build>/SC2_x64.exe` —— **真正的游戏客户端**（目录名里的数字即构建号）
- `Maps/Campaign/`（官方战役）· `Maps/CustomCampaigns/`（CCM 自制）· `Mods/`
- ⚠️ `Maps` 与 `Mods` **首次使用自制内容时才创建**，不能假设其存在

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

**写盘闸门（2026-10 放开）**：所有写/删操作的目标路径，必须是**星际争霸安装目录之内**的
相对路径。出目录（`..`、绝对路径、盘符）一律拒绝；**安装目录里任意位置允许覆盖** ——
`SC2Data/`、`Versions/` 也一样（决定：自由度给作者，安全靠下面这两条）。

**硬性要求**（少一条都不行）：

1. **动之前先备份**：任何会被覆盖或删除的既有文件，先挪进 `data/backup/<owner>/…` 并记账
   （`Manifest::apply` 负责），切回去时原样还原（`Manifest::remove`）。
2. **先给用户看**：铺盘前出一份预演（`Manifest::preview`）—— 新增 / 覆盖 / 接管 / 删除
   各有几项、一共多少字节。
3. 删除前打印并核对绝对路径；解析符号链接 / junction 后再次校验。
4. 可恢复的破坏性操作提供备份或回收站（`IFileOperation`），**不要**直接 `remove_dir_all`。

> 为什么不收窄到 `Maps` / `Mods` / `Interfaces`：收窄过，代价是复刻战役整包装不进去
> （地图要落在 `Maps/Starcraft Mass Recall/…`），用户看到的是「拒绝往 … 写东西」。
> 闸门改成「目录内随便写」之后，兜底就是上面第 1、2 条。

存档目录 `<documents>/StarCraft II/Banks` 是**用户数据**而不是游戏本体，
按 issue #20 的存档隔离单独处理，不走这里的闸门。

---

## 11. AI 编码代理工作准则

1. **先读后写**：动手前阅读本文件 + 目标模块；不清楚的 API 必须查证（`cargo doc`、crate 源码、官方文档），**禁止臆造函数签名**。
2. **改动闭环**：任何改动完成后，必须实际执行 `cargo fmt` / `cargo clippy` / `cargo test`，并在回复中**如实报告命令与结果**；没跑过就不要说"应该没问题"。
3. **最小改动**：不顺手重构无关代码，不擅自更改公共 API；需要时先说明再动手。
4. **引用可追溯**：参考 `reference/scnexus` 得出结论时，标注 `文件:行号`。
5. **破坏性操作**：删除 / 移动 / 覆盖前，先解析并打印绝对路径确认；不确定时先询问。
5.1 **不许合并 PR** —— merge 只能由维护者自己做（见 §7.1）。
5.2 **不许擅自发 PR** —— 除非用户明确说要发，改动就待在分支上；推分支、跑门禁、贴验证都可以。
6. **不要提交**：`reference/`、构建产物、以及任何游戏版权文件（.SC2Map/.SC2Mod/.SC2Bank 等）——测试用样例请放 `testdata/`（已忽略）。
7. **语言**：与用户交流、写提交信息与文档用**中文**；标识符、日志级别、crate 名称用**英文**。
8. **不确定就说不确定**：把推断与事实分开陈述，不要用自信语气掩盖猜测。

---

## 12. 决策记录与待确认事项

### 12.1 已确认（2026-10-07）

| 决策 | 结论 |
| --- | --- |
| 项目名 | 中文「弥音启动器」/ 英文 **MiYin Launcher**（曾用名 HSCL 仅保留在本地目录名） |
| 技术栈 | **Rust + Tauri 2 + React 18 + TypeScript**（见 §3.1；原本的 Vue 3 已于 2026-10 全部迁完并移除） |
| 版本管理 | Git，`main` 为稳定分支，提交遵循 Conventional Commits（见 §7） |
| 参考实现 | `reference/scnexus`（BSD-3-Clause，gitignored，**只读**，见 §8） |
| 代理 | 系统代理 `http://127.0.0.1:7897`，已配好 Git 全局代理（见 §9） |
| 脚手架 | Rust workspace（`crates/miyin-core`）+ Tauri 2 桌面壳 + React 前端（见 §4） |
| 功能范围 | 首个版本聚焦**战役管理与安装**（扫描 / 预检 / 安装 / 卸载），不含下载站与账号功能 |
| 美术风格 | 只参考 **HMCL 的视觉语言**（Material 3 + **蓝色主色** + 大圆角卡片），**不参考其布局**；看板娘为弥音立绘（见 §13） |
| 战役数据 | **存放在星际争霸安装目录同级的 `miyin_launcher_data/`**（多个启动器实例共用一份），支持同一战役多版本共存与自由切换（见 §14.1） |
| 主菜单范围 | 只列**四大战役**（自由之翼 / 虫群之心 / 虚空之遗 / 诺娃），进化与序章归并到父战役 |

### 12.2 待确认（TODO）

- [x] **许可证**：定为 **MIT**（宽松、便于被广泛集成）。HMCL 是 GPL-3.0、
      scnexus 是 BSD-3-Clause，本项目**没有**复用它们的代码。
- [x] **仓库命名**：`SuperQuail/SC2Miyin-Launcher`。
      本地目录 `HSCL` 暂不改名，避免破坏现有工作流。
- [x] **CI**：`.github/workflows/ci.yml`（核心测试双平台 / 前端构建 / 桌面端整工作区）。
- [x] **启用 / 停用战役（激活）**：已实现，按清单精确回滚（见 §14.11）。
- [x] **游戏运行时探测**：`platform::game_running()`（走 tasklist 认 `SC2.exe` / `SC2_x64.exe`）；
      启用 / 停用前先问它，跑着就拒绝切换。
- [x] **从游戏目录反向导入**：`campaign::collect` 扫 `Maps/CustomCampaigns/*`，
      设置页列出还没进库的，一键收编；导入复用 `library::import`，**不删原目录**。
- [x] **包格式规范**：已写成 `docs/package-format.md`（兼容 CCM 与枢纽标准 + 弥音扩展）。
- [x] 包格式的**打包指南**：`docs/package-authoring.md`（给作者的，带示例与常见坑）。
- [x] `README.md` / `LICENSE` / `CHANGELOG.md` / CI 工作流。
- [x] `CONTRIBUTING.md` 与 Issue / PR 模板（`.github/ISSUE_TEMPLATE/`）。

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

### 14.1 为什么放在游戏安装目录旁边，而不是软件目录

游戏目录里的 `Maps/CustomCampaigns` 只能放「当前这一份」，装第二个版本就得先删第一个。
所以库放在**星际争霸安装目录的同级** `miyin_launcher_data/`（见 `library::data_root`）：

```text
<游戏安装目录的同级>/miyin_launcher_data/
├── library.json               # 索引：每个槽位下有哪些版本、当前启用哪个
├── active.json                # 激活清单：我们往游戏目录放了什么、挪走了什么
├── campaigns/<槽位>/<版本>/    # 各版本的完整内容
└── backup/<槽位>/              # 被挪走的官方文件，切回原版时原样还原
```

**为什么挂在安装目录旁边而不是 exe 同级**：绿色版可以拷好几份、也可以装在不同位置 ——
各自一份 `data/` 就互不相通。挂在安装目录旁边之后，同一台机器上的**所有启动器实例共用一份战役库**。

两条必须保留的兜底：

1. **找不到安装目录**（全新机器第一次启动、注册表里也没有）→ 退回 **exe 同级的 `data/`**，
   程序必须能起来。
2. **旧数据迁移**：新根不存在、而程序旁边还留着旧的 `data/` 时，启动时**搬过去**
   （同盘 rename，跨盘 copy + 删，见 `platform::move_path`）。新根**已经有内容就一律不动** ——
   不覆盖、不合并，只写一行日志（数据是用户的，宁可让他自己看一眼）。
   搬迁失败时**退回旧根继续用** —— 不能因为搬不动就没了库。

### 14.2 五个菜单项：原版战役 + 自制战役

主菜单分**两组**：

| 组 | 内容 | 说明 |
| --- | --- | --- |
| **原版战役** | 自由之翼 / 虫群之心 / 虚空之遗 / 诺娃 | 对官方四部的**改版**（重制、换单位、加关卡） |
| **自制战役** | `CampaignType::Custom` | 独立做的**整部**战役，不依附任何官方战役 |

两者不是一类，所以分成两个选单 —— 界面按 `is_custom()` 分组
（五个菜单项的固定顺序见 `CampaignType::MENU`）。

顺序由 `CampaignType::MAIN` / `MENU` 固定，**不要**改成按名字排序
（那样虚空之遗会排到诺娃后面）。

#### 术语：改版 vs 版本

玩家**基于官方四大战役**做的修改（重制、换单位、加关卡）叫**改版**：
界面上「原版战役」以外的条目一律写「改版」，不要再叫「版本」。
自制战役（独立整部）导入的多个条目、模组自身的版本号、启动器版本仍叫「版本」。
代码里的 `Variant` / `version` 字段不改名 —— 这条只约束用户可见文案。

#### 落盘位置也不一样

```text
原版战役（含改版）   Maps/Campaign[/子目录]/…      子目录见 §14.5
自制战役            Maps/CustomCampaigns/<名字>/…  SC2 给自制内容留的位置
```

规则写在 `library::compose::Placement`：

- `Placement::of(slot_slug, variant_id, target_sub)` 由槽位推出落点
- `payload_target_path(target, placement)` 展开成最终路径

**搞混的后果**：自制战役被塞进 `Maps/Campaign` 会污染官方目录，
而且游戏压根不会把它当自制战役列出来。
`activation::allowed_target` 已把 `Maps/CustomCampaigns` 加进白名单。

#### 版本管理

自制战役与官方改版一样，**每个版本都带 `version`**，冲突时按 §14.6 处理
（覆盖更新 / 重命名后导入）。包内声明 `campaign=custom` 即归入自制战役。

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

另一个真实样本（**AI盟友虫群之心**）：包内顶层就是 `evolution/…`，没有 `swarm/` 前缀。
`evolution/` 是**相对虫群之心战役目录**的结构，必须**保留这一层**、落到
`Maps/Campaign/swarm/evolution/…` —— 拍平到 `swarm/` 根，游戏在「进化」里就找不到这些地图。
`metadata.txt` 直接放在 `evolution/` 里的包同理（内容根就是 evolution）。
普通分类目录（`maps/`）仍然拍平，别把这条规则扩大化。

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

- **容器不挑**：zip 原生读；7z / rar / tar / … 统一交给内嵌 7-Zip（见下），
  不再调用系统自带的 `tar` / `unrar`。见 `campaign/contents.rs`。
- **内嵌 7-Zip**：7-Zip 26.05 的 `7z.exe` / `7z.dll` / `7zFM.exe` / `7zG.exe` 放在
  `src-tauri/assets/7zip/`，构建时 `include_bytes!` 进 exe，运行时解到
  `%LOCALAPPDATA%\net.miyin.launcher\tools\7zip-2605\`，并通过 `MIYIN_7Z` 让 core 优先用
  内嵌的 7z（用户不用装 7-Zip 也能解 7z/rar）。许可证与来源见 `docs/third-party.md`。
- **解压工具**：战役页「工具 ▾」与设置页「可选工具」都有「7-Zip 解压工具」入口，点开是
  **内嵌的 7-Zip 文件管理器窗口**：`src-tauri/src/sevenzip.rs` 用 `SetParent` 把 `7zFM.exe`
  嵌到主窗口**顶栏下方**；WebView2 的 DirectComposition 会盖住普通子窗口，所以用
  `SetWindowRgn` 把 `WRY_WEBVIEW` 裁到只剩顶栏 —— 我们的顶栏仍由页面渲染，顶栏里会出现
  「退出 7-Zip」按钮（`close_sevenzip` 给 7zFM 发 WM_CLOSE）。压缩/解压/测试都用 7-Zip
  自己的界面，不另写一套。
- **解压只走内置 7-Zip**：`UNPACKERS` 只剩内嵌的 `7z.exe`，不再调用系统 `tar` / `unrar`；
  加密包也用 7z 的 `-p`。
- **星际枢纽托管检测**：读 `%APPDATA%\@scnexus\app-main\SCNexusStorage` 的
  `store-profile.json` / `store-campaign.json` / `store-campaign-active.json`，
  扫 `<LIBRARY_ROOT>/*/metadata.json` 与 store 点名的 CCM 目录；卡片标「来自于：星际枢纽」，
  一键转托管（确认框；「删除枢纽副本」默认不勾，active 时禁止删）。实现见
  `crates/miyin-core/src/hub/scnexus.rs`；元数据 schema 照 `reference/scnexus/packages/standard`。
- **导出两种格式**：弥音包（完整保真）与星际枢纽兼容包；后者先 `preview` 列出会丢失的
  内容（补丁绑定、封面、说明、勾选配置、自制战役等），不静默丢。见 `library/export/`。
- **导入三层**：包内元数据（**以数据为准**）→ 证据链自动识别（**尽力而为**）→ 用户手动指定。
- **手动指定永远可用，且优先级最高**：`import_package` 传了 slot 就按传的来。
  界面上始终要有"导入到哪部战役"的选择，默认填自动识别的结果。
- 界面按 `ImportPreview.source` 措辞，**不许把"自动识别"说成确定的**。

### 14.9 打包发布

- **release 必须带 `--features custom-protocol`**（已写进 `src-tauri/Cargo.toml` 的
  `[features]`）。不带的话 Tauri 不会内嵌 `ui/dist`，程序会去连 devUrl，
  用户机器上没有 dev server -> 白屏 + ERR_CONNECTION_REFUSED。
- 绿色版：exe 独立一个文件夹即可；战役库在**游戏安装目录同级**的 `miyin_launcher_data/`。
- 出包前先 `pnpm -C ui build`，否则内嵌的是旧前端。
- **三个 `dist` 不是一回事**（踩过混淆）：
  - `ui/dist`：Vite 前端产物（`tauri.conf.json` 的 `frontendDist`），构建时内嵌进 exe；
  - `target/release/`：`cargo build --release` 的产物，真正的 exe 在这里；
  - 仓库根的 `dist/`：**本地投放目录**（`.gitignore` 里的 `dist/`），
    **只放启动器的单独 exe**：`dist/SC2Miyin-Launcher.exe`（由 `target/release/miyin-launcher.exe` 改名/复制而来）。
    **不放 zip、CLI、预览图、临时包或其它文件。** exe 名不能带空格 —— wry 把 exe 名传给
    WebView2，带空格会让 msedgewebview2 启动即崩。CI 发布物（两个 zip）由
    `.github/workflows/release.yml` 生成，与本地 `dist/` 不是一回事。
    它**不会自动更新** —— 要测最新版时重新构建后复制 exe 进来。

#### 14.9.1 绿色版首次在没有 WebView2 的机器上启动

Windows 10/11 一般自带 WebView2 运行时，但**重装过系统、精简版、某些企业镜像上没有**。
没有它，Tauri 连窗口都建不出来，用户看到的就是「双击一闪就没」。

现在启动器会**自己补这个环境**（`src-tauri/src/webview2_install.rs`，在 `run()` 里跑在
建窗口之前）：

- **检测**：`tauri::webview_version()`（Tauri 官方探测），读不到再查注册表
  `HKLM|HKCU\SOFTWARE[\WOW6432Node]\Microsoft\EdgeUpdate\Clients\{F3017226-…}\pv` 兜底。
  检测「不在了」才动手，已经装好的一步都不多做。
- **安装源**：微软官方 Evergreen 引导程序
  `https://go.microsoft.com/fwlink/p/?LinkId=2124703`（302 到 `MicrosoftEdgeWebview2Setup.exe`）。
  走系统代理设置（环境变量 → Windows 系统代理 → 直连，与 §15.2 同一套），**不写死任何代理地址**。
- **进度界面是原生 Win32**（因为这时还没有 WebView2，画不了 Tauri 窗口）：
  STATIC 说明 + 进度条 + 取消按钮，文案逐段走
  「正在检测 → 正在下载安装包（xx%，xx MB/xx MB）→ 正在安装（可能弹 UAC，请点「是」）→
  安装完成，正在启动启动器」。下载/安装在后台线程，UI 线程只跑消息循环。
  原生窗口建不出来时退回 MessageBox（同样说清在做什么 + 手动链接）。
- **安装**：先直接 `/silent /install`（再试 `--silent /install`），需要时用
  `ShellExecuteW("runas")` 提权；完成后轮询检测，进程退出但检测仍失败会明确说「装是跑了，但系统里还是没有」。
- **失败文案**：原因 + 手动下载链接 + 三条能动手的排查（UAC 点「是」、杀软放行 /
  以管理员身份运行、装完重开）。

> 调试开关：`MIYIN_WEBVIEW2_DEMO=1` 会在**已经装了**运行时的机器上强制再走一遍这个流程，
> 只跳过「真的跑安装程序」那一步，用来验界面与文案。别在给用户的说明里提它。

#### 14.9.2 单实例与按需提权

**启动器永远以普通权限启动，启动时不弹 UAC。** WebView2 profile、`config.json`/`network.json`、
临时文件都留在用户目录（`%LOCALAPPDATA%`/`%APPDATA%`/`%TEMP%`），不迁到数据根 ——
否则 C 盘安装的用户连“看界面”都要提权。数据根仍然只放战役库（游戏安装目录同级）。

- **单实例**：命名内核对象 `Local\net.miyin.launcher.single`（`CreateMutexW`/`CreateEventW`，
  不写文件、不依赖目录权限）。第二个实例会聚焦已有「弥音启动器」窗口；找不到才提示后退出。
- **按需提权**：写游戏目录 / 库根被拒（错误文本以 `[PERMISSION]` 开头）时，
  UI 调 `request_privileged_worker()` 用 `ShellExecuteW(runas)` 拉起**同一个 exe** 的
  `--privileged-worker` 模式（UAC 只弹一次，助手驻留复用），再用
  `privileged_op(op,args)` 重放这一次操作；**UI 不重启、状态不丢**。
  用户拒绝 UAC 时显示“需要管理员权限”对话框，可再次提权或保持现状，其余功能照常可用。
- **助手范围**：只代理写游戏目录/库根的操作（activate/deactivate、import/update/delete、
  补丁、模组铺盘与同步、merge_apply）；存档、导出、config/network 写留在 UI 进程。
  游戏/编辑器始终由 UI（普通权限）启动。
- **安全**：命名管道随机名 + 仅当前用户/SYSTEM 的 ACL + nonce 握手 + 客户端镜像路径/PID 校验
  + 服务端路径白名单；`--privileged-worker` 不加载 Tauri/WebView2、不抢单实例锁。
- **托盘**：进程活着时右下角始终有托盘图标（左键显示主界面，右键「显示主界面 / 退出启动器」）；
  窗口异常消失时用户仍能直接退出，不必去任务管理器。
- **起不来就退出**：setup 里发现没有有效 HWND 的 webview 窗口（WebView2 无法启动）→ 返回错误，
  `run()` 收尾弹原生错误框后 `exit(1)`；不允许「窗口没了、进程还在」。
- **低完整性检测**：启动时查进程令牌完整性（`platform::process_integrity_level`）；低于 Medium
  （exe 放在低完整性目录/沙箱工作区）直接弹框说明“解压到普通文件夹”，不继续启动 ——
  这种进程写不了 `%LOCALAPPDATA%` 的 WebView2 profile，硬起只会得到“窗口建不出来”。

### 14.10 存档管理（issue #20）

实现在 `crates/miyin-core/src/saves/`：**看 / 备份 / 还原** 游戏**真正读**的存档目录 ——
不是单个 `Banks`，而是 `Documents\StarCraft II\Accounts/<账号>/<地域>/Banks`
（本机核实，2026-10；可能有不止一个账号）。`Accounts` 结构完全不存在时才退回
`Documents\StarCraft II\Banks`（老路径，通常为空、游戏不读）。

- **一组 = 所有账号的 Banks 一起**（见 `SavesRoots`）：快照 / 备份 / 还原 / 切换都对
  全部根整体操作，组里按 `Accounts/<账号>/<地域>/Banks/…` 的层级存，切回时原样铺回。
  不按账号拆组 —— 同一部战役的进度本来就散在各账号里。
- 存档是**用户数据**，不走游戏目录那套写盘闸门，但同样两条：**动之前先备份**、
  **先给人看**（`snapshot()` 就是给人看的那一份）。
- **还原前自动把现在这份另存为 `还原前-<时间戳>`** —— 点错了还能退回来。
- 还原会**先清空再铺**：不清的话备份里没有的旧文件会剩下来，两份混在一起。
- 备份目录不适用的场景（还没玩过）返回 `missing`，**不建空目录** ——
  还原一个空备份只会让人以为"存档回来了"。
- `state.json` 读不出来时**不再静默当「关着」**：`Isolation.warning` 带一条人话给界面，
  坏文件在下次写盘前改名成 `state.json.corrupt-<时间戳>` 留证。

按版本自动隔离（每个战役组合一套 Banks）已经做了，但**默认关着** ——
用户在设置页主动打开才生效（打开那一刻把当前这份收成「原版」）。

> [!WARNING]
> **存档管理目前是 WIP**：组与战役的对应关系还没修好，启用存档隔离**可能导致存档损坏**。
> 存档页与设置页的隔离卡必须保留醒目警示条与「开启前二次确认」；修好之前，
> 任何文案都不许暗示「这个功能已经能放心用」。

### 14.11 切换的安全底线

启用 / 停用**只操作清单里记过的文件**，绝不递归删除官方目录：

1. 换版本时按**安装清单**撤掉该槽位**所有**已铺的东西（不只看 `slot.active`）：
   删掉我们放的文件、还原被挪走的官方文件。
2. 铺新版**失败就回滚**：清掉本次半装的文件、启用状态清空，返回「已回滚到原版」。
3. 目标位置若已有同名文件（多半是官方的），先挪进 `data/backup/` 并记账。
4. 复制新版本的文件并逐条记账，最后写 `installed.json`。
5. 存档组跟着「当前启用的是谁」一起切 —— 所有会 `activate` 的路径共用同一个 core 函数。

因此最坏情况是「多留了几个文件」，而不是「官方战役没了」。
这部分由 `crates/miyin-core/src/library/tests/` 的端到端测试覆盖。

### 14.12 模组挂载：配置与实际铺盘是两件事

`ModEntry` / `LibraryMod` 上有两个布尔，**不要混用**：

| 字段 | 含义 | 谁来决定 |
| --- | --- | --- |
| `mounted` | **用户配置**里挂着它（勾选框状态） | `Variant::mounted_mods`，`None` = 没配过 = 全挂 |
| `in_game` | **真的铺进 `<游戏>/Mods/`** | 安装清单 `installed.json`（按 owner 记账） |

`variant_mods()` / `all_mods()` **不许再按 `slot.active` 门控 `mounted`** —— 曾经这么干过，
后果是：没启用的版本一律显示未挂载，用户勾了下次读回来又变未勾；启动拦截读到全 false，
把「配置里全挂着」误判成「一个都没挂」，自制战役直接被拦住启动（issue 反馈）。

**挂载由启动器自己负责，不要指望用户手动点。**

- 启动引导（`AppState::bootstrap`）会调 `activation::sync_mods()`：按激活清单 + 各槽位
  当前选中的版本，把 `Mods/` 下的东西补齐 / 撤掉；幂等，已经一致就跳过；目标归别人或
  被用户手动放的文件占着就跳过并告警（**不抢**）。`launch_game` 之前也会 sync 一次。
- `sync_mods` 只动 `Mods/` 前缀（`Manifest::apply_scoped`），地图落点与补丁归
  `activate` / 补丁那套管，互不越界。

**不许给「进游戏」加「至少启用一个模组」之类的门槛。** 配了哪些模组是用户自己的事、
铺不铺是启动器的责任；拦下来只会让自制战役根本打不开（issue 反馈）。
自制战役页（`CustomCampaignPanel`）**只读展示**「已勾选的模组」（启动器会同步的那些），不提供勾选；
导入对话框也不再问「要不要挂载」。勾选/启停只留在「模组」页（供排查问题），
且有醒目提示「模组由启动器管理，非开发者请勿手动调整」。

界面上「已勾选」（`mounted`）与「已生效」（`in_game`）是两个状态；`in_game` 为 false
时不要写成「已启用」。

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

### 15.3 界面约定

更新这件事**必须让用户看得见**，不能只是个后台行为：

| 位置 | 做什么 |
| --- | --- |
| 顶栏右侧 | 有新版时显示**带下载图标的角标**（图标 + 版本号，轻微呼吸动画），点它跳设置页 |
| 启动时 | **自动检查一次**，不需要用户手点；失败也只在日志里记一笔，不弹红字 |
| 有新版时 | 启动即弹**更新公告**：标题 + 渲染后的 Release 正文，勾「不再提示这个版本」后只留角标 |
| 设置页「更新」 | 版本卡 + 下载按钮 + **进度条** + 安装按钮 |
| 更新日志 | **内嵌终端**：等宽字体、深色底、自动滚到底，把"刚才做了什么"逐行写出来 |
| 下载完成后 | 弹**居中遮罩对话框**，说明需要重启才能安装，给「稍后」与「立即重启并安装」 |

终端的内容来自后端的 `Reporter`（见 `miyin-core/src/update/mod.rs`）：
命令把它接到 Tauri 事件 `update://log` / `update://progress` 上，
前端在 `ui/src/react/state/useUpdate.ts` 里统一订阅并缓存，顶栏与设置页共用同一份状态。

两条容易踩的坑：

1. 对话框的 `.sheet` 系列样式**放全局**（`ui/src/styles/base.css`），不要写在某个组件自己的样式里 ——
   否则别的组件用同名类时完全没有样式（弹窗退化成堆在页面末尾的普通方块），踩过一次。
2. `bindUpdateEvents()` **不要加 `isDesktop` 判断** —— 浏览器演示模式的桥接
   同样提供这两个订阅，加了判断演示模式就永远收不到日志。
3. **Release 正文的 Markdown 必须先净化再插进 DOM**（`ui/src/api/markdown.ts`：
   marked 解析 → DOMPurify 过滤）。正文说到底是从网络来的数据，
   就算仓库是自己的，也不该让一段正文有本事往页面里塞 `<script>` 或 `onerror=`。
4. 公告里的链接**不能在 WebView 里跳转**（那会把启动器自己导航走），
   一律拦下来交给系统浏览器（`api.openUrl`）。
5. 公告要显示图片的话，CSP 的 `img-src` 得放行 `https:`（已放）。
   代价是正文里的图片地址能被对方看到你的 IP —— 只显示自家仓库的内容，可以接受。

### 15.4 底线

- **网络失败不算错误**：包成 `UpdateCheck::error` 返回，界面显示"检查失败"即可。
- **不确定就不动**：版本号认不出、摘要对不上、包里没有 exe、zip 条目越界 —— 一律拒绝。
- **更新只换程序本体**：`data/`（战役库、补丁、配置）一个字节都不碰。
- Windows 上覆盖不了正在运行的 exe，所以走 `.cmd` 重试脚本。

---

## 16. 命令层：一律 `#[tauri::command(async)]`

**踩过的坑**：Tauri 2 里**同步命令跑在主线程**上。启动器的命令动辄几秒到几分钟 —— 读整个压缩包、
拷贝几个 GB、等用户选文件 —— 同步写法会把主线程占死，窗口收不到消息泵，用户看到的就是**「未响应」**。

所以 `src-tauri/src/lib.rs` 里的命令**全部**写成：

```rust
#[tauri::command(async)]
fn import_package(...) -> Result<...> { ... }
```

`(async)` 让 Tauri 把同步函数体丢到线程池 —— 一个词的事，不用改写函数体。
特别长的活（下载、查更新）再加一层 `async fn` + `tauri::async_runtime::spawn_blocking`。

**自查**：`src-tauri/src/commands/` 里不应该出现不带 `(async)` 的 `#[tauri::command]`。
新增命令写在对应域的 `commands/*.rs`，并在 `lib.rs` 的 `generate_handler` 里注册（命令名是 IPC 契约，改名等于改协议）。

---

## 17. 界面改动：先渲染，再定方案

**涉及前端 GUI 的重构或新做，不许直接写代码。** 顺序固定：

1. **先渲染** —— 用真实的设计令牌和组件做出能看的版本（不是线框图），
   在浏览器演示模式（`pnpm -C ui dev`）里真的跑起来
2. **有不确定性的设计，至少出三个版本**，每个都截图交付预览
3. **用户选一个，或者打回重做** —— 选定之后才动真格的代码

「不确定性」指的是：布局怎么摆、信息密度多少、交互走哪条路、视觉重心在哪。
机械改动（改文案、补图标、修对齐）不需要走这套。

**为什么**：界面方案是唯一没法靠测试对齐的东西 —— 代码错了测试会红，
界面丑了只有人看得出来。先渲染再选，比写完再推倒重来便宜得多。

预览产物不入库（`ui/preview.*` 已在 `.gitignore` 里），截图随当次交付给用户。

---

## 18. 前端：全 React（Vue 已于 2026-10 迁完并移除）

> [!IMPORTANT]
> **用户侧与开发者页现在都是 React 18，Vue 已从依赖里移除**
> （`ui/package.json` 里没有 `vue` / `vue-tsc` / `@vitejs/plugin-vue` / `@vue/test-utils`，
> `pnpm-lock.yaml` 已同步；`ui/src` 下**没有 .vue 文件**）。
>
> - **不再写 Vue。** 任何新界面都是 React。
> - **两个入口都是 React**（见下方路径）；迁移期的"整块替换 / 不混写两套框架"那套规矩已成历史。
> - 控件来源：用户侧用自建控件层 `ui/src/react/ui/`（见 §18.1），开发者页用 Ring UI。
> - 前端全部说明（结构 / 边界 / 构建坑 / 调试开关 / 历史）见 [`docs/frontend.md`](docs/frontend.md)。

**两个入口仍然分开**（原因不再是框架不同，而是**包体积**：Ring UI 不能进用户侧的包）。

```text
ui/index.html → src/main.tsx     → React 18   用户侧（战役 / 模组 / 设置 / 开发者房间）
ui/ide.html   → src/ide/main.tsx → React 18   开发者页（IDE 工作台 + #gallery 控件展台）
```

**硬规矩**（越过这条，构建和样式会一起出鬼）：

1. **两个入口不共享组件**：用户侧只准 import `ui/src/react/**`、`ui/src/api/**`；
   开发者页只准 import `ui/src/ide/**` 与 `ui/src/api/**`。
2. 两边共享的只能是**框架无关的纯 TS**（`ui/src/api/*`：桥接 / 类型 / markdown / 美术映射），
   那里**不许 import `react`**。
3. 跨页跳转是**换入口**（整页加载），不是路由跳转 —— 别假装它们在一个 SPA 里。
4. 别为了"顺手"把某个 Ring UI 组件搬进用户侧。要那个控件就在开发者页里用
   （开发者房间的第二页是 `iframe` 嵌 `ide.html#gallery`，见 §18.1）。

**Ring UI 的三个坑**（都踩过）：

| 坑 | 说明 |
| --- | --- |
| 用 `@jetbrains/ring-ui-built`，**不是** `@jetbrains/ring-ui` | 后者是源码包，`.js` 里带裸 JSX，rolldown 不转 `node_modules` |
| 必须手动引 `@jetbrains/ring-ui-built/components/style.css` | 不带样式就等于没装（控件渲染成原生，标签还会叠字） |
| `icon` 要**组件类型**，不是 `<svg/>` 元素 | 传元素会炸在 `Element type is invalid` |

React 固定在 **18**（ring-ui 的 peer 是 16/17/18；19 删了 `findDOMNode`）。

**构建是两次，顺序不能乱**：

```bash
pnpm -C ui build     # = 清空 dist -> 用户侧（vite.config.ts）-> 开发者页（vite.ide.config.ts）
```

- **两个入口必须分开构建**（放一起，rolldown 会把公共块塞进 `index.html` 预加载 → 用户侧白屏）。
- 两次构建都是 `emptyOutDir: false`，**清空只在 build 脚本开头做一次**（否则单跑一次 `vite build` 会把 `ide.html` 抹掉）。

> 这两条与 tsconfig 的 `jsx`、`<style scoped>` 搬过来变全局 —— 现场记录都在 [`docs/frontend.md`](docs/frontend.md)。

**别在开发者页用 Tauri 的 `onCloseRequested`**：那是**按窗口**注册的，不是按页面 ——
打开过一次开发者页，之后回到启动器点 X 也关不掉窗（Tauri 从此不再自己关，交给已经没了的
JS 回调）。拦"打包没完别走"要用页面内的按钮 + `beforeunload`。踩过，整个启动器关不掉。

**它的 `Tabs` 可以用**（2026-10 实测更正）：`children` 就是标签本身，**不要求配面板**；
配 `autoCollapse`，放不下的标签会折成「更多」。导入路径是
`@jetbrains/ring-ui-built/components/tabs/tabs`（没有包级 index，具名导出 `Tabs` / `Tab` /
`CustomItem`）。文档标签页见 `ui/src/ide/workbench/parts/DocTabs.tsx`。

自排容器时注意两点：`.ring-tabs-titles` 默认有 16px 的 `margin-bottom`，要归零；
`Tabs` 会额外渲染一个装选中面板的空 `div`（不传 children 就是空盒子，无害）。

---

### 18.1 控件必须先上开发者房间

**规矩：一个控件只有在开发者房间（设置页 →「开发者模式」→「进入开发者房间」）里
展示过，才能拿到生产界面用。**

- 新控件（或现有控件的新变体）先在房间里排一行 —— 每个变体、每个状态（禁用、出错、整行…）
  各来一份，确认**外观与手感**之后再上生产。
- 房间里只放**控件与通用组件**，不放业务页面。业务页面的样式应当来自控件本身。
- 房间的「待迁移」列表是**唯一**允许存在"没控件化的东西"的地方。迁完一个划掉一个。
- **不许**再往那个列表里加新东西：新界面要么用已有控件，要么先在房间里做出新控件。
- 为什么：控件是这个项目里唯一"错了没人替你发现"的东西 —— 测试能证明逻辑对，
  证明不了它长得好不好看、挪到别处是不是还成立。把它摆在一个专门的地方看，
  比在生产页面里逐个试便宜得多。

> 房间在**用户侧入口**里（`ui/src/react/views/room/`），已按两套视觉语言**分成两页**：
> 第一页用户侧控件（`react/ui/`），第二页开发者工具侧（`iframe` 嵌 `ide.html#gallery`）。

### 18.2 右键菜单与快捷键：一律不露浏览器味儿

**WebView 的原生右键菜单（另存为 / 检查 / 重新加载）与这个应用格格不入，
而且会把用户导向一个跟它无关的世界。所以整个窗口一律拦掉。**

- 用户侧：`ui/src/react/app/useShellEffects.ts` 在 window 上一律 `preventDefault()`；
  组件自己的菜单靠 `ui/src/react/lib/contextMenu.ts` 的时间戳优先。
  **输入框里给剪切/复制/粘贴/全选** —— 这是拦掉的代价，不能连粘贴路径都做不到。
- 开发者页：空白处右键也要兜住（`ui/src/ide/workbench/` 里的空白菜单），自己的菜单靠
  `event.defaultPrevented` 判定。
- 新写右键菜单时：**在自己的 `onContextMenu` 里先 `preventDefault()`**，
  否则它会和兜底菜单叠在一起。

**快捷键也是同一件事**（2026-10 补）：用户不该在这个应用里感觉到"我在用浏览器"。三层：

| 层 | 在哪 | 管什么 |
| --- | --- | --- |
| **WebView2 硬开关** | `src-tauri/src/lib.rs` 的 `tame_webview` | `SetAreBrowserAcceleratorKeysEnabled(false)` —— **唯一**能彻底摘掉浏览器加速键（Ctrl+F/P/U/R、F5、Ctrl+±/0）的地方。右键菜单**不在这里管**（页面已经拦干净，重复设一遍是冗余）。Tauri 2.11 没暴露 wry 的对应 builder，只能顺着 `with_webview` 拿 `ICoreWebView2Controller` 自己关 |
| **页面兜底** | `ui/src/api/webview.ts` 的 `installWebviewGuards()` | 两个入口都装：拦刷新/打印/缩放/中键/拖放导航；**浏览器演示模式也生效**。开发者页用 `allow` 放行 `Ctrl+F`（编辑器的查找）与开发者工具 |
| **要用的快捷键** | 原生菜单 accelerators 或页面自己的 keydown | 别依赖浏览器默认 —— 想用哪个就自己实现哪个 |

- **别拦过头**：`Ctrl+C/V/Z/A/S`、方向键、输入法组合键都不能碰（`webview.ts` 里有测试钉着）。
- 拦不住的只有 OS 级组合（Alt+Tab、Win 键）—— 那些本来也不该拦。
