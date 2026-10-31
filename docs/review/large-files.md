# 单文件 >800 行：耦合体检与拆分方案

> **2026-10-29 更新：本报告列出的 >800 行文件已全部拆分完成。** 拆分后全仓源码
> （含测试）最大文件 791 行（`library/install.rs`），其余 ≤789；`python scripts/check-file-size.py 800`
> 输出「没有超过 800 行的原始代码，干净」。目录模块化后的结构见 CHANGELOG「未发布 / 解耦」。
> 下文保留原始体检记录（作为拆分依据与历史证据）。
>
> **扫描口径**：维护者的判据是「单代码文件 > 800 行 = 严重过耦合」。本文按这个线重新扫全仓**源码**
> （`*.rs` / `*.ts` / `*.tsx` / `*.css`），排除 `target/`、`node_modules/`、`ui/dist`、`reference/`、
> `assets-staging/`、`ui/src/preview/`（本地设计草稿，gitignore）与生成物。
>
> **测量快照**：2026-10-29 16:1x（本地），HEAD `51e71e4`，工作树有 6 个未提交改动
> （`crates/miyin-core/src/library/{activation,install,mod}.rs`、`src-tauri/src/lib.rs`、
> `ui/src/react/views/mods/{ModsView.tsx,mods.css}`）。
> 其中 `library/mod.rs`、`library/install.rs`、`src-tauri/src/lib.rs`、`library/tests.rs`、
> `ui/src/api/{bridge,types}.ts` **正在被别人编辑**，行号会漂 ±几十行。本文判断的是**结构**，
> 不是精确坐标；引用行号时以「快照」为准。
> **复测（16:2x）**：`src-tauri/src/lib.rs` 2581、`library/mod.rs` 1521 —— **>700 的名单与顺序均未变**，只是 ±8 行漂移。
>
> **不改任何代码。** 本文只出清单、证据与方案。

---

## 0. 结论速览

| 结论 | 内容 |
| --- | --- |
| 仓库总量 | 227 份源码 / ≈5.34 万行（`ui/src` ≈2.66 万，`crates/miyin-core/src` ≈2.19 万） |
| >800 行 | **10 份**（门禁脚本只报出 9 份 —— `ui/src/ide/ide.css` 是门禁盲区，见 §1.3） |
| 700–800 逼近线 | **1 份**（`crates/miyin-core/src/campaign/metadata.rs` 789） |
| 600–700 观察名单 | 3 份（`library/install.rs` 694、`dev/export.rs` 649、`library/store.rs` 612） |
| 真过耦合 | **6 份**：`campaign/package.rs`、`library/mod.rs`、`library/mods.rs`、`saves.rs`、`ui/src/api/bridge.ts`、`campaigns/SlotMenu.tsx` |
| 结构性合理 | **4 份**：`library/tests.rs`（测试聚合）、`ui/src/api/types.ts`（纯类型）、`ui/src/ide/ide.css`（样式）、`src-tauri/src/lib.rs`（机械命令表，见 §3.3 的保留意见） |
| 先拆谁 | **① `campaign/package.rs`（`inspect()` 一个函数 454 行、6 个阶段）→ ② `ui/src/api/bridge.ts`（96% 是演示数据）→ ③ `library/mod.rs`（胖 `mod.rs` 的尾巴）** |
| 别动 | `ui/src/api/types.ts`、`ui/src/ide/ide.css` —— 行数大但没有「职责耦合」这回事 |

一条总的判断：**行数在 Rust 侧主要说明「测试和业务挤在一个文件里」，在前端侧主要说明「演示数据挤在桥接层里」。**
真正需要靠拆分解决的「一个函数干六件事」只有两处：`campaign/package.rs:636-1089` 的 `inspect()`，
和 `ui/src/react/views/campaigns/SlotMenu.tsx:28-810` 的 `SlotMenu()`。

---

## 1. 方法与口径

### 1.1 用了哪两个尺子

```bash
# ① 仓库自带门禁（只认 git ls-files，认 .gitignore；不含 .css）
python scripts/check-file-size.py 800

# ② 自己按目录统计（绕开 git，按扩展名白名单走目录；补上门禁漏掉的 .css）
#    只统计 .rs / .ts / .tsx / .css；跳过 target、node_modules、dist、reference、
#    assets-staging、preview、.vite
```

两把尺子对得上（见 §6 附录）：800 线以上两份口径都报同样的 9 份，差异只在 `ide.css`。

### 1.2 类别怎么分

| 类别 | 判据 | 本期命中 |
| --- | --- | --- |
| **测试** | 文件整体在 `#[cfg(test)] mod tests` 下，或文件名就是 `tests.rs` / `*.test.tsx` | `library/tests.rs` |
| **类型** | 只有 `interface` / `type` 声明与常量，零运行时逻辑、零 import | `api/types.ts` |
| **样式** | 纯样式表 | `ide/ide.css` |
| **桥接** | 主体是「把 A 的调用转成 B 的调用」：命令表、IPC 适配、演示后端 | `src-tauri/src/lib.rs`、`api/bridge.ts` |
| **业务** | 有领域判断（规则、状态机、格式解析、磁盘操作） | 其余 5 份 |

> 注意：类别不等于结论。业务文件里也有一半行数是测试（`saves.rs` 55%、`package.rs` 27%），
> 而「桥接」文件里也有真逻辑（见 §3.3）。下面每个文件都分开算。

### 1.3 先记一笔门禁的盲区

`scripts/check-file-size.py:23` 的扩展名白名单是：

```
{".rs", ".ts", ".tsx", ".vue", ".js", ".jsx", ".py", ".ps1", ".mjs"}
```

**没有 `.css`**。所以 `ui/src/ide/ide.css`（811 行）永远不会出现在门禁输出里 —— 它不是被「豁免」，
是**没被看见**。本期 >800 的真实数量因此是 10 而不是门禁报的 9。这不影响功能，但门禁的「信号」少了一档。
（本文只报告，不改脚本 —— 任务边界是不动代码。）

### 1.4 按目录统计（源码口径，227 份 / ≈53,413 行）

| 范围 | 文件数 | 行数 | 占比 |
| --- | ---: | ---: | ---: |
| `ui/src` | 172 | ≈26,631 | 50% |
| `crates/miyin-core/src` | 40 | ≈21,876 | 41% |
| `src-tauri/src` | 2 | ≈2,579 | 5% |
| `crates/miyin-core/examples` | 5 | 729 | 1.4% |
| `crates/miyin-cli/src` | 2 | 619 | 1.2% |
| `crates/miyin-cli/tests` | 1 | 509 | 1.0% |
| `crates/miyin-core/tests` | 1 | 381 | 0.7% |
| `ui/*.config.ts` + `src-tauri/build.rs` | 4 | 89 | 0.2% |

值得注意的密度：`crates/miyin-core/src/library/` 一个目录 **13 份 / ≈10,505 行**，
占核心层总行数的一半；`ui/src/react/views/campaigns/` 18 份 / ≈3,966 行，
是前端最重的一个页面目录。**>800 的 10 份里有 4 份落在 `library/`。**

---

## 2. 表 1：>800 行清单（按行数降序）

| # | 文件 | 行数 | 类别 | 生产 / 测试 | 一句话性质 |
| ---: | --- | ---: | --- | ---: | --- |
| 1 | `crates/miyin-core/src/library/tests.rs` | **3190** | 测试 | 0 / 3190 | 全仓唯一的「战役库端到端测试总集」，69 个 `#[test]` |
| 2 | `crates/miyin-core/src/saves.rs` | **2598** | 业务 | 1180 / 1418 | 存档备份 + 存档隔离 + 目标解析 + Banks 扫描，4 个概念一个文件 |
| 3 | `src-tauri/src/lib.rs` | **2573** | 桥接 | 2573 / 0 | **106 条 IPC 命令**全在这一个文件，外加少量平台实现 |
| 4 | `crates/miyin-core/src/campaign/package.rs` | **1688** | 业务 | 1227 / 461 | 包预检；`inspect()` 一个函数 454 行，内含 6 个阶段 |
| 5 | `crates/miyin-core/src/library/mods.rs` | **1647** | 业务 | 1555 / 92 | 独立模组库：索引 + 导入流水线 + 铺盘 + 导出 + 比对 |
| 6 | `crates/miyin-core/src/library/mod.rs` | **1522** | 业务 | 1522 / 0 | 胖 `mod.rs`：数据模型 + Library 索引 + 数据根迁移 + 版本冲突 |
| 7 | `ui/src/api/bridge.ts` | **1507** | 桥接 | 1507 / 0 | 桌面桥只占 123 行，**≈1350 行是浏览器演示后端** |
| 8 | `ui/src/api/types.ts` | **965** | 类型 | 965 / 0 | 69 个 `interface`/`type`，零运行时逻辑；`LauncherApi` 契约 239 行 |
| 9 | `ui/src/ide/ide.css` | **811** | 样式 | 811 / 0 | 开发者页样式，130 个选择器、12 个区段；**门禁扫不到** |
| 10 | `ui/src/react/views/campaigns/SlotMenu.tsx` | **810** | 业务（UI） | 810 / 0 | `SlotMenu()` 一个组件函数 782 行、25 个内部 handler |

排序说明：第 2、3、4、5、6 名的行数会随别人正在进行的改动漂移，**相邻名次之间差距不大**，
不要把「谁是第二名」当成结论。



---

## 3. 表 2：逐个文件为什么算过耦合

下面每份都给四样东西：**职责分组（带行号）**、**耦合证据**、**拆分方案**、**判断**。

---

### 3.1 `crates/miyin-core/src/library/tests.rs` — 3190 行（测试）

**结论：合理（测试聚合）。** 它没有生产逻辑，全部内容在 `#[cfg(test)] mod tests` 之下；
被 `crates/miyin-core/src/library/mod.rs:63-64` 以 `#[cfg(test)] mod tests;` 载入，
编译进测试二进制，不进发布产物。**但它确实过大了**，大到新增一个战役库测试时要先想「放哪一段」。

**它承担了几件事（按行号）**：

| 区段 | 行号 | 内容 | 大致行数 |
| --- | --- | --- | ---: |
| 夹具 | 13-92 | `MARKER`、`build_zip`、`Fixture`、`fixture`、`package`、`slot` | 80 |
| 导入 / 启用 / 切换 / 回滚 | 93-430 | 版本共存、切回原版、官方地图备份还原、恶意槽位拒绝、资料片落点 | 338 |
| 版本比较 / 冲突 / 覆盖与重命名 | 431-627 | `compare_versions`、`conflict_for`、Rename vs Overwrite、元数据编辑 | 197 |
| 补丁 | 628-1049 | 叠加、优先级、导出回环、补丁元数据编辑 | 422 |
| 主地图解析 | 1050-1108 | `resolve_main_map` 不猜、自然序排序 | 59 |
| 模组 | 1109-1806 | 分组、导入去重、兄弟互斥、目录形态、包内取模组 | 698 |
| 冲突 / 接管 | 1807-1972 | 不同 owner 不互相覆盖、冲突告警、只接管一条路径 | 166 |
| 归属判定 / 镜像布局 | 1973-2295 | 自制战役落点、显式槽位优先、根级入口图、游戏根镜像 | 323 |
| 预演 / 写盘闸门 | 2296-2417 | `preview` 声明会动什么、游戏目录内可写 | 122 |
| 数据根 / 迁移 / 半成品 | 2418-2744 | 新根、迁移、不覆盖已有新根、半个 root 不被信任、遗留恢复 | 327 |
| 合并（多实例） | 2745-2941 | 合并计划、应用、冲突、遗留目录回收 | 197 |
| 激活冲突 | 2942-3190 | 三条冲突判定（含平台路径比较） | 249 |

**耦合证据**：文件头只 import 4 处（`tests.rs:3-10`），但它 import 的是
`crate::library::{ImportMode, Library, MapEntry, VariantChanges, VersionRelation, activate,
compare_versions, compose, conflict_for, conflicts, import, patch, remove_variant,
resolve_main_map, update_variant}` —— **15 个库 API**，再加 `campaign::package` 与
`sc2::{DiscoverySource, Installation}`。也就是说它依赖的是整个 `library` 的公开面：
`library/` 里任何一次签名改动都会先在这里报错。这是**测试该有的耦合**（端到端测试本来就跨模块），
问题是它把 12 个域的用例挤在一个文件里，改一处要滚 3000 行。

**拆分方案**（纯搬运，零语义变化）：

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `tests/mod.rs` + `tests/fixtures.rs` | 120 | 夹具：`build_zip` / `Fixture` / `fixture` / `package` / `slot` / `seed_library` / `conflict_package` |
| `tests/import_activate.rs` | ≈540 | 93-430 + 431-627 |
| `tests/mods.rs` | ≈700 | 1109-1806 |
| `tests/packages.rs` | ≈640 | 628-1049 + 1973-2295 + 预演段 2296-2417 |
| `tests/data_root.rs` | ≈520 | 2418-2941 |
| `tests/conflicts.rs` | ≈250 | 1807-1972 + 2942-3190 |

路径从 `src/library/tests.rs` 改成 `src/library/tests/mod.rs`（`mod tests;` 不用改）。
**顺序**：先搬夹具，再按区段搬用例，每搬一段跑一次 `cargo test -p miyin-core`。
**风险**：低 —— 只有测试可见性（夹具需要 `pub(crate)` 或 `use super::*`）；
`git blame` 会断一次。**收益**：新增测试有明确落点；单次 `cargo test` 的报错定位更短。

**注意**：迁移类测试（2418-2941）依赖真实临时目录与 `tempfile`，按 AGENTS §6 的约定
（涉及文件系统一律 `tempfile`）本来就成立，拆文件不影响。

---

### 3.2 `crates/miyin-core/src/saves.rs` — 2598 行（业务；生产 1180 / 测试 1418）

**结论：真过耦合（生产部分），但拆的收益是「读得懂」，不是「改得动」。**

**为什么算过耦合**：生产段的 1180 行里，四个概念**互不调用**，只是都读写一份 `Isolation` 状态：

| 职责 | 行号 | 内容 | 行数 |
| --- | --- | --- | ---: |
| ① 备份 / 还原引擎 | 76-232 | `snapshot`（76-107，给人看的那一份）、`backup`（137-155）、`backups`（156-197）、`restore`（198-232，还原前自动另存） | ≈157 |
| ② 文件搬运 | 233-344 | `group_dir` / `group_files` / `move_save_file` / `move_save_to_key` | ≈112 |
| ③ 存档隔离状态机 | 345-418 | `Isolation`、`ORIGINAL`、`state_file`、`isolation`、`write_isolation`、`set_isolation` | ≈74 |
| ④ profile 切换与绑定 | 419-569 | `save_active`、`save_into`、`switch`、`assign`、`profile_for`、`reconcile_active_binding`、`bind_unbound_active` | ≈151 |
| ⑤ 组标识 / 命名 | 570-628 | `CUSTOM_PREFIX`、`is_custom_key`、`identity_of`、`isolation_key` | ≈59 |
| ⑥ 目标解析 | 629-933 | `SaveTarget`、`picked_variants`、`named_targets`、`group_label`、`targets`、`ensure_for_slot`、`original_group` | ≈305 |
| ⑦ 扫描 / 清单 | 934-1181 | `SaveEntry`、`BankEntry`、`SaveInventory`、`SKIP_DIRS`、`scan`、`scan_saves_dir`、`scan_banks_dir`、`save_stem` | ≈248 |
| ⑧ 测试 | 1182-2598 | 46 个 `#[test]` | 1418 |

**耦合证据**：
- import 面**很窄**：`saves.rs:22-29` 只有 6 行 —— `std`、`serde`、`crate::error::Result`、
  `crate::library::{Library, LibraryIndex, Variant}`、`crate::safety`。
- 反向引用也只有 **1 个**：`src-tauri/src/lib.rs`（`use miyin_core::saves as library_saves;`），
  加前端 `ui/src/api/types.ts` 里的 `Save*` 类型镜像。
- 但**内部领域概念有 7 个**（上表①-⑦），而 `scan`（1011-1087）与 `restore`（198-232）
  之间没有任何调用关系 —— 它们只是「都跟存档有关」。

**拆分方案**：把 `saves.rs` 改成目录

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `saves/mod.rs` | ≈80 | 类型 re-export + `pub use` 转发（**先立转发层，再搬**） |
| `saves/backup.rs` | ≈330 | ①+②（备份、还原、搬运） |
| `saves/isolation.rs` | ≈290 | ③+④+⑤（开关、profile、键） |
| `saves/targets.rs` | ≈310 | ⑥ |
| `saves/scan.rs` | ≈250 | ⑦ |
| `saves/tests/mod.rs` + 4 个子文件 | ≈1400 | 按 backup/isolation/targets/scan 分 |

**顺序**：`scan.rs`（纯只读、无副作用）→ `targets.rs`（纯计算）→ `backup.rs` → `isolation.rs`（最后动状态机）。
**风险**：**本清单里风险最高的一份** —— 它操作的是用户存档（AGENTS §14.10：
「存档是用户数据」）。但反面是它有 1418 行测试护着（46 个用例），
`restore` 的「先清空再铺」「还原前自动另存」都有用例（`saves.rs:1182+`）。
所以「有测试」和「用户数据」两件事互相抵消：**可以拆，但要一段一跑 `cargo test`**。

**判断**：真过耦合。但**优先级放在后面**（见 §5）：调用面窄意味着拆分解决的是阅读成本，
而收益最小的恰是「没人天天改」的存档模块。

---

### 3.3 `src-tauri/src/lib.rs` — 2573 行（桥接；0 测试）

**结论：真过耦合（按职责），但拆起来最机械、也最容易被漏掉一处。**

**它承担了几件事（按行号）**：

| # | 命令组 | 行号 | 代表命令 |
| ---: | --- | --- | --- |
| 1 | 状态与配置 | 29-239 | `AppState`（41-59 / impl 85-199）、`load_config`（200-208）、`open_in_browser`（209-230） |
| 2 | 安装发现 | 240-316 | `detect_installation`、`set_installation` |
| 3 | **存档（12 条）** | 317-499 | `save_isolation`、`switch_save_profile`、`list_save_backups`、`restore_saves`、`banks_root` |
| 4 | 启用预演 / 冲突 | 500-537 | `preview_activation`、`activation_conflicts` |
| 5 | **开发者页（9 条）** | 538-692 | `dev_scan`、`dev_export`、`dev_history`、`dev_commit`、`dev_diff` |
| 6 | **战役库 / 导入 / 变体（15 条）** | 693-937 | `list_slots`、`inspect_package`、`prepare_import`、`import_package`、`activate_variant`、`set_mounted_mods` |
| 7 | 编辑器与工具 | 938-1133 | `open_map_in_editor`、`list_tools`、`install_tool`、窗口三连（1026-1058） |
| 8 | **独立模组（13 条）** | 1134-1430 | `import_mod`、`edit_mod`、`export_mods`、`list_library_mods` |
| 9 | 文档 / 变体元数据 | 1431-1473 | `variant_doc`、`read_doc`、`update_variant` |
| 10 | **网络与更新（6 条）** | 1474-1659 | `network_settings`、`check_update`、`download_update`、`apply_update` |
| 11 | 导出 / 补丁绑定（13 条） | 1660-1832 | `export_variant`、`bind_patch`、`configure_patch`、`preview_composition` |
| 12 | 删除 / 启动 / 路径揭示 / 封面 | 1833-2010 | `delete_variant`、`launch_game`、`reveal_in_file_manager`、`file_to_data_url` |
| 13 | 平台实现 | 2011-2101 | `base64_encode`（2037-2076，手写 40 行）、`tame_webview`（2077-2101，WebView2 直操作） |
| 14 | 数据合并 / 崩溃 | 2102-2277 | `merge_apply`、`append_crash_log`、`install_crash_hook`（2182-2219） |
| 15 | 启动装配 | 2278-2573 | `run()`（2278-2465，含 `generate_handler!` 大表）、`prepared_map`（2541-2565） |

**耦合证据**：
- `#[tauri::command(async)]` 共 **106 条，且 106 条全带 `(async)`** —— AGENTS §16 的规矩没被破坏。
- import 15 行（`lib.rs:14-27`），从 `miyin_core` 拉进 6 个模块
  （`campaign::metadata`、`campaign::package`、`library`、`library::mods`、`saves`、`sc2`、`tools`、`update::*`）。
- 反向引用只有 `src-tauri/src/main.rs`（挂 `run()`），但**真正的调用方是前端**：
  `ui/src/api/bridge.ts:64-186` 的 `desktop` 对象有 90 个方法，一个一个 `invoke(...)` 打到这 106 条命令。
  所以它是「前端 × 核心层」的**唯一接口表**，不是普通模块。
- 它并不纯：`file_to_data_url`（2011-2034）、`base64_encode`（2037-2062）、
  `reveal_in_file_manager`（1903-1949）、`tame_webview`（2077-2101）、`install_crash_hook`（2182-2219）、
  `update_variant_from_package`（2504-2539，含「点明 vs 按身份找」的领域判断）、
  `prepared_map`（2541-2565，先 activate 再解析地图路径）都是**真实现**，不是转发。

**拆分方案**：

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `src-tauri/src/lib.rs` | ≈220 | `AppState`、`run()`、`generate_handler!`、`resolve_site_url`、启动失败兜底 |
| `commands/installation.rs` | ≈120 | 组 2 + 4 |
| `commands/saves.rs` | ≈220 | 组 3 |
| `commands/dev.rs` | ≈170 | 组 5 |
| `commands/library.rs` | ≈280 | 组 6 + 9 |
| `commands/mods.rs` | ≈300 | 组 8 |
| `commands/tools.rs` | ≈200 | 组 7 |
| `commands/update.rs` | ≈190 | 组 10 |
| `commands/patches.rs` | ≈190 | 组 11 |
| `commands/launch.rs` | ≈180 | 组 12 + `launch_game_with_map`、`prepared_map` |
| `platform/base64.rs` | ≈50 | `base64_encode` + `file_to_data_url`（**建议下沉到 `miyin-core` 或独立 crate**） |
| `platform/webview.rs` | ≈40 | `tame_webview` |
| `platform/crash.rs` | ≈110 | 崩溃日志 + 钩子 |

**顺序与风险**：
1. **先搬平台实现**（第 13、14 组）—— 它们跟命令注册无关，零风险。
2. 再按组搬命令。**关键约束**：`run()` 里的 `tauri::generate_handler![...]` 列表必须同步搬，
   漏一条是**编译期错误**（不算运行时风险，这点很好）；但**命令名不能改** ——
   它是 IPC 契约，前端 `invoke("...")` 的字符串在 `bridge.ts` 里。改名即破坏。
3. `#[tauri::command(async)]` 的 `(async)` 不能掉 —— 掉了命令回主线程，界面「未响应」（AGENTS §16）。

**判断**：真过耦合（15 个领域一个文件）。但它有一个反直觉的性质：
**这是「平」的耦合** —— 106 条命令之间互不调用，搬走一条不影响另一条。
所以风险低、收益也只是「找命令不用滚 2500 行」。**优先级放在中后段**（见 §5）。

---

### 3.4 `crates/miyin-core/src/campaign/package.rs` — 1688 行（业务；生产 1227 / 测试 461）

**结论：真过耦合，且是本清单里「单个函数最长」的一处 —— 优先拆它。**

**为什么算过耦合**：一个 1688 行的文件里，其实塞了**两样东西**：

1. **一个 454 行的单函数** `inspect()`（`package.rs:636-1089`）—— 它按顺序干六件事，
   文件里的注释已经把这六步标出来了（这就是最好的证据）：

| 阶段 | 行号（注释处） | 干什么 |
| --- | --- | --- |
| 打开容器 | 637-683 | zip 原生读、其它交系统解压器（`contents` 模块）；先把清单拷出来 |
| **安全闸门** | 684-719 | 条目数 / 总字节 / 单图字节上限、zip-slip（`safe_entry_path` 在 609-635） |
| **元数据定位** | 720-880 | 找 `metadata.txt` / `patch.txt` / `metadata.json`，忽略子目录里的、版本比启动器新就拒绝 |
| **镜像布局** | 881-889 | 包内直接是游戏目录样子（顶层有 `Maps/`、`Mods/`）时内容根取包根 |
| **载荷** | 890-908 | 地图/模组是单文件还是目录树；「只有模组没地图」判为补丁 |
| **归属判定** | 909-1019 | 四层证据链（元数据 → 镜像路径 → 地图内依赖 → 文件名启发式） |
| **命名** | 1020-1089 | 目录名兜底（空名/保留名/全非法字符 → 退回压缩包名） |

2. **一套「包内路径语义」工具包**（`package.rs:232-635`，14 个函数 ≈404 行）：
   `entry_path`、`collect_map_declarations`、`looks_like_game_category`、`canonical_game_dir`、
   `has_mirror_root`、`count_payloads`、`collect_payloads`、`payload_root`（423-441）、
   `is_under`（442-471）、`mirrors_game_root`（472-494）、`game_relative`（495-521）、
   `payload_target`（522-576）、`is_known_campaign_dir`、`known_campaign_prefix`、`safe_entry_path`（609-635）。

   这套路径规则是 AGENTS §14.5 里「最容易踩的坑」的代码化（「包内相对目录的含义取决于内容根站在哪里」），
   它跟 `inspect()` 的流程无关，是**可独立测试的纯函数**。

**耦合证据**：
- import 8 行（32 行文件头之后）：`crate::campaign::{contents, identify, metadata, sanitize, CampaignFormat, HealthIssue}`、
  `crate::error::Result`、`std`、`serde` —— **依赖了 campaign 里几乎所有兄弟模块**。
- 被 **14 个文件**引用：`library/{compose, export, mods, patch, store, compat, mod}`、
  `dev/export`、`src-tauri/src/lib.rs`、`miyin-cli`、2 个 examples、`library/tests.rs`。
- 内部领域概念 **5 个**：容器格式（zip/7z/rar）、路径安全、payload 形态、CCM/弥音元数据、归属识别。
- 测试 461 行（1229-1688 两段：`mod tests` 1229-1397、`mod payload_tests` 1398-1688）——
  这是**拆分的安全网**，也是它「敢先动」的理由。

**拆分方案**：改成目录

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `package/mod.rs` | ≈90 | 类型 re-export + `inspect`/`extract_to` 的入口声明 |
| `package/types.rs` | ≈200 | `MAX_*` 常量、`Payload`、`PayloadTarget`、`Entry`、`PackageInspection`、`unusable` |
| `package/paths.rs` | ≈420 | 232-635 全部（纯函数，无 IO） |
| `package/inspect.rs` | ≈460 | 636-1089，按阶段提函数（见下） |
| `package/extract.rs` | ≈140 | `extract_to`（1090-1174）+ 小工具 |
| `package/tests/` | ≈460 | 按 paths / inspect / payload 分三个文件 |

`inspect()` 内部再提成 6 个私有函数（建议名即阶段名）：
`open_archive`（637-683）、`gate`（684-719）、`locate_metadata`（720-880）、
`resolve_content_root`（881-908）、`detect_campaign`（909-1019）、`decide_dir_name`（1020-1089）。
**只提函数、不改顺序** —— 注释里写死了隐含依赖：`package.rs:894-897`
「启发式：只有模组、一张地图都没有 → 判为补丁。**必须排在归属判定之前**」，
`package.rs:881-889` 镜像布局也必须在载荷之前。

**顺序**：`paths.rs`（纯函数，先抽，`payload_tests` 直接跟着走）→ `extract.rs` → `types.rs` → 最后处理 `inspect()`。
**风险**：中低 —— 有 461 行测试；唯一要小心的是上面三处**顺序依赖**，以及
`inspect()` 里对 `archive` 的借用时序（`package.rs:655`「先把清单拷出来，后面读内容还要可变借用 archive」）。

**判断**：**真过耦合，最该先拆**。理由：单个函数 454 行是清单里最长的业务函数，
而它的拆分是「提函数 + 分文件」，不改任何判断逻辑。

---

### 3.5 `crates/miyin-core/src/library/mods.rs` — 1647 行（业务；生产 1555 / 测试 92）

**结论：真过耦合（5 个职责），其中「导出」与「比对」两块最独立，可先拆。**

**它承担了几件事（按行号）**：

| # | 职责 | 行号 | 行数 |
| ---: | --- | --- | ---: |
| 1 | **铺盘同步** `sync` / `placed_name` | 29-187 | ≈159 |
| 2 | 类型群 | 188-370 | `ModKind`、`ModSource`、`StandaloneMod`、`ModPreview`、`ModImport{Mode,Action}`、`ModChanges`、`ModIndex` ≈183 |
| 3 | **索引读写 / 扫描** | 371-513 | `mods_root`、`list`、`save`、`get`、`scan_dir`、`fingerprint_of`、`effective_id` ≈143 |
| 4 | **暂存与定位** | 514-719 | `StagedMod`、`Staged`、`single_child_dir/file`、`locate_mods`、`stage` ≈206 |
| 5 | **预览 / 导入** | 720-1022 | `preview`、`import`、`import_one`（870-998，129 行）、`unique_version`、`copy_tree` ≈303 |
| 6 | 战役记录 | 1048-1124 | `find_campaign`、`remember_campaign`、`short_hash` |
| 7 | 更新 / 启停 / 删除 | 1125-1198 | `update`、`set_enabled`、`remove` |
| 8 | **导出** | 1199-1355 | `export`（57 行）、`export_many`（99 行） |
| 9 | **比对** | 1356-1533 | `ModFileDiff`、`ModComparison`、`ModSemanticDiff`、`list_files`、`compare`（100 行）、`same_bytes` |
| 10 | 补记战役 | 1534-1556 | `repoint_campaign` |
| 11 | 测试 | 1556-1647 | 只有 1 个 `#[test]`，92 行 |

**耦合证据**：
- import 5 行：`std`、`serde`、`sha2::{Digest, Sha256}`、`crate::error`、`crate::safety`。
- 被 **7 个文件**引用：`library/{mod,store,tests}.rs`、`src-tauri/src/lib.rs`、
  `examples/{try-mod-import,verify-mod-deploy}.rs`、`tests/packages.rs`。
- 内部领域概念 **5 个**：模组**身份**（文件夹粒度、`fingerprint_of` 505-513、`effective_id` 514-528）、
  **导入流水线**（stage → import_one）、**铺盘**（sync，走 `install` 引擎）、
  **导出**（打 zip、写 CCM metadata）、**比对**（按文件/按语义）。
- 测试覆盖的信号很弱：**1555 行生产代码只有 92 行内联测试**。
  真正的模组测试在 `library/tests.rs:1109-1806`（≈700 行），跨文件验证 —— 见 §3.1。

**拆分方案**：

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `mods/mod.rs` | ≈100 | re-export + `mods_root` / `index_path` / 小工具 |
| `mods/types.rs` | ≈200 | 第 2 组全部 + `SyncReport` |
| `mods/index.rs` | ≈150 | 第 3 组（`list`/`save`/`get`/`scan_dir`/`fingerprint_of`/`effective_id`） |
| `mods/import.rs` | ≈520 | 第 4+5 组（`stage`、`locate_mods`、`import`、`import_one`、`copy_tree`） |
| `mods/deploy.rs` | ≈190 | 第 1 组 + `set_enabled` + `placed_name` |
| `mods/export.rs` | ≈170 | 第 8 组 |
| `mods/diff.rs` | ≈190 | 第 9 组 |
| `mods/tests.rs` | ≈92 | 现有内联测试 |

**顺序**：先 `export.rs`（对外 API 只有 `export`/`export_many`，最独立）→ `diff.rs`（纯计算）→
`types.rs` → `deploy.rs` → 最后 `import.rs`（最长、最容易踩 `safety` 校验）。
**风险**：低-中 —— `sync` 走的是 `library/install.rs` 的统一引擎（`install.rs:59-187`），
拆分不能把它绕开；`import_one` 里对「整包进整包出」的目录形态判断要靠 `library/tests.rs` 的模组段兜。

**判断**：真过耦合。拆分价值主要在**导入流水线**（stage 617-719 + import 803-869 + import_one 870-998
共 380 行，跟导出/比对完全无关）。



---

### 3.6 `crates/miyin-core/src/library/mod.rs` — 1522 行（业务，全生产）

**结论：真过耦合 —— 典型「胖 mod.rs」：子模块拆了 10 个，剩下的尾巴没拆干净。**

**它承担了几件事（按行号）**：

| # | 职责 | 行号 | 行数 |
| ---: | --- | --- | ---: |
| 0 | 子模块声明 + `write_atomic` | 41-73 | 11 个子模块（`activation` 49、`compat` 50、`compose` 51、`install` 52、`known` 53、`mods` 54、`naming` 55、`export` 58、`merge` 59、`patch` 60、`store` 61）+ 原子写 41-48 |
| 1 | **数据模型** | 74-260 | `INDEX_VERSION`、`Variant`（78-158，81 行）、`ModOrigin`、`LibraryMod`、`DocInfo`、`ModEntry` |
| 2 | **模组身份 / 分组** | 261-394 | `effective_mounted_mods`、`group_mods`（277-320，44 行）、`ModKey`、`mod_identity`、`normalize_mod_key`、`mod_key_of` |
| 3 | **入口地图解析** | 395-538 | `MapEntry`、`MainMapChoice`（409-455）、`resolve_main_map`（456-526，71 行）、`looks_like_entry_map` |
| 4 | 补丁 / 绑定 / 索引 / 槽位视图 | 539-652 | `CampaignSlot`、`Patch`、`default_priority`、`Binding`、`LibraryIndex`、`SlotView` |
| 5 | **`Library` 索引与查询** | 653-1086 | `Library` 结构（653-656）+ `impl Library`（657-1086）= **20 个方法 430 行** |
| 6 | **数据根解析 / 迁移** | 1087-1306 | `default_root`、`data_root`、`DataRootPlan`、`is_complete`、`partial_root`、`parked_root`、`dir_has_entries`、`is_writable`、`resolve_data_root`（1185-1306，122 行） |
| 7 | **冲突 / 版本** | 1307-1444 | `slot_for`、`Conflict`、`VersionRelation`、`ImportMode`、`compare_versions`、`conflict_for`（1394-1444） |
| 8 | 小工具 | 1445-1522 | `require_slot`、`notice_for`、`unique_suffix`、`now_seconds`、`natural_cmp`（1486-1522，37 行） |

**耦合证据**：
- import 10 行（`library/mod.rs:28-39`）：`campaign::CampaignFormat`、`campaign::metadata::{CampaignType, PackageKind}`、
  `campaign::package::{Payload, PayloadTarget}`、`error`、`safety`、`sc2::Installation`、`serde`、`std`
  —— **它同时引了 `campaign` 的三个兄弟模块**。
- 被 **21 个文件**引用（`library/` 内部 9 个、`dev`、`saves`、`campaign/{collect,contents,package}`、
  `src-tauri`、`miyin-cli`、3 个 examples），是核心层引用面最广的文件。
- 内部领域概念 **4 个**（上表 2、3、6、7），彼此独立：
  「模组身份」不需要「入口地图」，「数据根迁移」跟「版本冲突」毫无关系。
- `library/mod.rs:64` 的 `#[cfg(test)] mod tests;` 把 3190 行的测试挂在这里（§3.1）——
  也就是说**所有测试都从这一个文件进入**，反过来强化了它的中心地位。

**拆分方案**：这里的**顺序很关键** —— 先立转发层，否则 21 个引用方和 `tests.rs` 立刻全红。

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `library/mod.rs` | ≈120 | 只剩 `mod` 声明 + `pub use` 转发 + `write_atomic` |
| `library/model.rs` | ≈330 | 第 1 组 + 第 4 组（所有序列化结构） |
| `library/mod_keys.rs` | ≈140 | 第 2 组（模组身份/分组/键） |
| `library/main_map.rs` | ≈150 | 第 3 组（入口地图解析） |
| `library/index.rs` | ≈450 | 第 5 组（`impl Library`；内部再分 `index_io` 与 `queries`） |
| `library/data_root.rs` | ≈230 | 第 6 组（数据根 + 迁移，AGENTS §14.1 的兜底逻辑） |
| `library/conflict.rs` | ≈150 | 第 7 组 + 第 8 组（`natural_cmp` 可进 `naming.rs`） |

**顺序与风险**：
1. **先加 `pub use` 转发**（`pub use model::*;` 等）—— `library::` 的对外路径一个字节都不变，
   `src-tauri`、`miyin-cli`、`tests.rs` 全部照旧编译。这一步单独提交，风险 0。
2. 再逐块搬。每搬一块，`cargo test -p miyin-core`。
3. **风险点**：`Library` 的 7 个路径方法（`root`、`index_path`、`campaigns_dir`、`patches_dir`、
   `patch_dir`、`backup_dir`、`slot_dir`，657-700 附近）与其他方法共用 `self.root`，
   拆文件时**方法和字段必须同文件**（用 `impl` 块分文件是允许的，但字段定义只能在一处）。
4. `natural_cmp`（1486-1522）是通用工具，可顺手挪 `library/naming.rs`（已有 160 行）——
   但那是**行为等价搬迁**，别顺手改名。

**判断**：真过耦合。**优先级高**（§5 排第 3）：它引用了 21 个地方，是所有库操作的入口，
拆干净之后 §3.5 的 `mods.rs` 拆分才有地方放。

---

### 3.7 `ui/src/api/bridge.ts` — 1507 行（桥接；≈96% 是演示后端）

**结论：真过耦合，但「耦合」的是演示数据 —— 这是本期**性价比最高**的一次拆分。**

**它承担了几件事（按行号）**：

| # | 职责 | 行号 | 行数 |
| ---: | --- | --- | ---: |
| 1 | **桌面桥**：90 个方法，一行一个 `invoke(...)` | 64-186（`desktop`） | ≈123 |
| 2 | `isDesktop` 判定 | 60-61 | 2 |
| 3 | **演示夹具：安装 / 库根** | 187-210 | `demoInstallation`、`DEMO_LIBRARY_ROOT` |
| 4 | **演示夹具：补丁 / 预检 / 变体** | 211-319 | `DemoPatch`、`demopatches`、`demoInspection`、`variant()` |
| 5 | **演示夹具：槽位与版本** | 320-398 | `demoslots` |
| 6 | **演示夹具：模组 / 地图** | 399-504 | `demoLibraryMods`、`demoGameMods`、`demoMaps`、`demoMods` |
| 7 | **演示夹具：合并** | 505-543 | `demoMergeLocal/Target`、`demoMerged`、`demoMergeStatus` |
| 8 | **演示夹具：存档（最重的一块）** | 544-849 | `delay`、`demoSaveGroups`、`demoSavesState`、`demoBanks`、`demoFilesOf`、`demoBanksOf`、`demoInstalledIn`、`demoSaveInventory`（652-712）、`demoIsolationKey`、`demoEnsureSaves`、`demoTargets`、`demoMoveFile` |
| 9 | **演示实现**：72 个方法（可交互的假后端） | 853-1476（`demo`） | ≈624 |
| 10 | 后端选择 | 1477 | 1 |
| 11 | 一个**例外导出**：`ActivationConflict` + `activationConflicts` | 1480-1507 | 28 |

**耦合证据**：
- import 只有 3 个源（`bridge.ts:8-9` + `types`）：`@tauri-apps/api/core`、`@tauri-apps/api/event`、
  `./types`；但那个 `import type {...}` 块列了 **43 个类型**（`bridge.ts:11-57`）——
  它同时握着**整个 IPC 契约**。
- 被 **25 个文件**引用：`react/state/*`（8 个 hook）、`react/app/*`、`react/components/*`、
  `react/views/*`（campaigns / custom / install / mods / settings / update）、`ide/workbench/Workbench.tsx`。
  即**前端每个功能都要 import 它**，而其中 96% 的行跟这些功能无关。
- 内部领域概念 **6 个**（上表 3-9）：安装、战役库、模组、合并、存档、更新 —— 各自一整套演示夹具。
- 反证：桌面桥本身只有 123 行、90 个方法，**真正「桥接」的内容很少**；
  这一层并不难，难的是它跟 1385 行演示数据共处一文件。

**拆分方案**：

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `api/bridge.ts` | ≈160 | `isDesktop` + `desktop`（64-186）+ `api` 选择 + `activationConflicts`（保留原导出路径） |
| `api/demo/fixtures/installation.ts` | ≈60 | 安装、库根 |
| `api/demo/fixtures/library.ts` | ≈140 | patches、inspection、variant、demoslots |
| `api/demo/fixtures/mods.ts` | ≈120 | 模组 / 地图 |
| `api/demo/fixtures/merge.ts` | ≈60 | 合并夹具 |
| `api/demo/fixtures/saves.ts` | ≈260 | 544-849 这一整块 |
| `api/demo/api.ts` | ≈640 | `853-1476` 的 `demo` 对象，按域再拆 4-6 份（可后置） |
| `api/demo/index.ts` | ≈30 | 汇总成 `LauncherApi` |

**顺序**：先搬夹具（`saves.ts` 最重、也最独立）→ 再按域切 `demo` 实现对象 →
最后 `bridge.ts` 只剩 160 行。
**风险**：低，但有两点必须保住：
1. **演示夹具之间有共享可变状态**（`demoInstalled` 629-634、`demoGroupFiles` 597、`demoFilesOf` 598-610、
   `demoBanks` 578-587）—— 拆模块时要保持**模块级单例**，不能变成每个引用方一份副本，
   否则演示模式的「可交互」会失真（AGENTS §15.3 的演示模式是截图验收的依据）。
2. `api` 的选择（`isDesktop ? desktop : demo`）必须**只出现在一个地方**，
   否则 `bindUpdateEvents()` 那种「不要加 `isDesktop` 判断」的坑会回来（AGENTS §15.3 第 2 条）。

**判断**：真过耦合，**优先拆**（§5 排第 2）。它是全前端引用最多的文件（25 个），
而 96% 的行是「浏览器演示模式」专用数据 —— 读它的人 99% 的时候只需要那 123 行桌面桥。

---

### 3.8 `ui/src/api/types.ts` — 965 行（类型）

**结论：合理（纯类型）。不建议动。**

**证据**：
- **零 import**（`ui/src/api/types.ts:1` 起就是 doc 注释 + 类型），零运行时逻辑；
  69 个 `interface` / `type` 声明，全部是 `miyin-core` 序列化结构的镜像（文件头原文：
  「与 Rust 侧 `miyin-core` 的序列化结构一一对应（均为 snake_case）」）。
- 最长的单块是 `LauncherApi`（726-965，239 行）—— 那是 **IPC 契约**，是一张清单，不是逻辑。
- 被 **46 个文件**引用（去掉 3 个 gitignored 的 `ui/src/preview/*` 草稿后是 **43 个**），
  是前端引用面第二广的文件。

**为什么不算过耦合**：接口声明之间**没有控制流**。改 `Variant` 不会让 `SaveInventory` 变复杂；
拆成 `types/{installation,library,mods,saves,update}.ts` 只是把 69 条声明分到 5 个盒子，
`import` 行数反而增加（43 个引用方要改路径）。**行数是「契约表面」的大小，不是复杂度。**

**真要减行只有一个办法**：按域给前端各 hook 提供**窄接口**，让 `useSaves` 不 import 整个 `LauncherApi`。
但那是接口设计议题，不是「大文件」议题 —— 且当前规模（965 行 / 43 引用）不构成阻塞。

**判断**：**合理，不动。**

---

### 3.9 `ui/src/ide/ide.css` — 811 行（样式）

**结论：合理（样式表）。不建议动。顺带修门禁的盲区（另开任务）。**

**证据**：
- 12 个区段，注释已分好（`ide.css:24` 工具栏、窗口按钮、主体、面板头/内容/状态栏、
  文档标签页、文本/二进制/图片、文件树、包信息表单、日志、右键菜单、离开拦截、控件展台 #gallery）；
- 130 个顶层选择器（`.tb`、`.ide`、`.pane` …）；
- **只有 1 个引用方**：`ui/src/ide/main.tsx:9`（另有两处出现在 `gallery/demos.tsx:17`、
  `gallery/sections.tsx:175` 的字符串/示例里）。开发者页独立入口，跟用户侧零共享（AGENTS §18 第 2 条）。
- 它**不在门禁的统计范围**：`scripts/check-file-size.py:23` 的扩展名白名单没有 `.css`。

**为什么不算过耦合**：样式的「职责」就是选择器；`ide.css` 里没有一条规则会因为另一条规则变复杂。
真正的风险是**全局作用域打架**（AGENTS §18 的坑：「Vue 的 `<style scoped>` 搬过来变全局」），
那跟行数无关，跟命名有关 —— 它已经用 `.tb` / `.ide` 这种短前缀避让了。

**什么时候再拆**：当某个区段自己超过 200 行（目前最大的是文件树 / 日志），
或用户侧要复用其中某个块时。届时按区段拆 `ide/{toolbar,panes,tabs,forms,menu}.css`，
但要注意 Vite 的 CSS 顺序（`main.tsx` 的 import 顺序决定层叠）。

**判断**：合理。**唯一动作建议**：把 `.css` 加进门禁白名单（一行改动，另开任务，本期不动代码）。

---

### 3.10 `ui/src/react/views/campaigns/SlotMenu.tsx` — 810 行（业务 UI）

**结论：真过耦合 —— 一个组件函数 782 行、25 个 handler、6 个界面区块。**

**它承担了几件事（按行号）**：

| # | 职责 | 行号 | 证据 |
| ---: | --- | --- | --- |
| 1 | **状态容器** | 40-60 | 16 个 `useState`（版本、补丁、编辑、导出、删除确认、更新对话框…） |
| 2 | 版本选择 / 启用 / 启动 | 97-129 | `pickVariant`(98)、`apply`(111)、`applyAndPlay`(116) |
| 3 | 删除确认 | 121-128 + JSX 787-806 | `confirmDrop`(122) + 一个 `Sheet` |
| 4 | **补丁管理**（9 个 handler） | 181-256 | `togglePatch`(183)、`unbindPatch`(187)、`bindPatchItem`(191)、`setPriority`(195)、`autoBind`(202)、`importPatch`(218)、`exportPatchItem`(231)、`openPatchEdit`(245)、`savePatchEdit`(256) |
| 5 | **元数据编辑**（含依赖模组） | 275-322 + JSX 685-764 | `openEdit`(277)、`toggleRequired`(297)、`saveEdit`(304) |
| 6 | **导出** | 323-350 | `doExport`(325) |
| 7 | 拖拽排序 | 66-96 | `order`(67)、`guard`(70) |
| 8 | 剪贴板 / 右键菜单 | 130-180 | `showVariantMenu`(136)、`copyVariantPath`(175) |
| 9 | JSX：头条 banner | 355-377 | key art 底图 + 开始游戏 |
| 10 | JSX：版本卡片（拖拽） | 379-435 | 原版固定卡 + 玩家版本 |
| 11 | JSX：自制战役面板 | 437-460 | 地图 + 挂载模组 |
| 12 | JSX：**补丁面板** | 462-548 | 绑定的补丁列表 + 优先级 + 自动挂载 |
| 13 | JSX：底部操作条 | 550-623 | |
| 14 | JSX：编辑补丁 `Sheet` | 625-684 | |
| 15 | JSX：编辑元数据 `Sheet` + 依赖模组列表 | 685-764 | |
| 16 | JSX：文档查看 / 铺盘预演 / 删除确认 | 766-806 | `DocViewer`、`ActivationPreviewDialog`、删除 `Sheet` |

文件里已经有 `/* ---------------- 补丁 ---------------- */`（181）、`/* ---- 编辑元数据 ---- */`（275）、
`/* ---------------- 导出 ---------------- */`（323）三段手工分隔 —— **作者自己已经意识到这是三块**。

**耦合证据**：
- import **15 个**（`SlotMenu.tsx:1-17`）：`react`、`api/art`、`api/bridge`、`api/errors`、`api/types`、
  `lib/clipboard`、`ui`（控件层）、`components/{DocViewer, VariantCard}`、
  同目录 `{CustomCampaignPanel, ImportDialog, ActivationPreviewDialog, SortableCard}`、
  `./host`、`./slot-menu.css`。
- 反向只有 **1 个**：`ui/src/react/views/campaigns/CampaignsView.tsx:4` `import { SlotMenu } from "./SlotMenu"`。
  也就是说**对外接口很窄，内部是一锅** —— 典型的「一个组件包打一页」。
- 内部领域概念 **5 个**：版本选取、补丁绑定、元数据编辑、依赖模组、导出。

**拆分方案**（受 AGENTS §17 约束：先渲染、截图确认布局不变，再动代码）：

| 拆成 | 建议行数 | 放什么 |
| --- | ---: | --- |
| `campaigns/SlotMenu.tsx` | ≈240 | 状态容器 + 布局骨架（355-435、550-623）+ `pickVariant`/`apply`/`applyAndPlay`/拖拽 |
| `campaigns/PatchPanel.tsx` | ≈220 | 补丁面板（462-548）+ 编辑补丁 `Sheet`（625-684）+ 9 个补丁 handler |
| `campaigns/VariantEditSheet.tsx` | ≈200 | 编辑元数据（685-764）+ `openEdit`/`saveEdit` |
| `campaigns/ModDepsEditor.tsx` | ≈80 | 依赖模组列表（736-762）+ `toggleRequired` |
| `campaigns/DeleteVariantSheet.tsx` | ≈40 | 787-806 + `confirmDrop` |

`host.ts`（同目录）已经是现成的「把外壳的动作递进来」模式 —— 拆分照着它做，
子组件从 `host` 拿 `notify` / `pendingActivation` / `launch`，不要各自再加 prop 透传。

**顺序**：`DeleteVariantSheet`（最小、最独立）→ `ModDepsEditor` → `VariantEditSheet` → `PatchPanel`
（最大，且 `patchForm` 的 5 个字段与 `openPatchEdit` 成对出现）。每拆一步在浏览器演示模式里
（`pnpm -C ui dev`）看一眼布局。
**风险**：中 —— 这是**纯 UI 重构**，没有测试护（该目录只有 `ActivationConflictDialog.test.tsx`），
所以 AGENTS §17 的「先渲染再定方案」在这里是硬约束：**拆之前截图，拆之后对比**；
另外 16 个 `useState` 搬进子组件时要注意「删除确认 / 编辑弹窗」的状态仍然属于父组件（一次只开一个）。

**判断**：真过耦合。**优先级中**（§5 排第 5）：收益明显（一个 782 行函数没人愿意读），
但风险与验证成本比 Rust 侧高，且必须先走 §17 的渲染流程。



## 4. 700–800 逼近线

严格落在 700–800 之间、**下一批会撞线**的只有 1 份：

| 文件 | 行数 | 生产 / 测试 | 判断 |
| --- | ---: | ---: | --- |
| `crates/miyin-core/src/campaign/metadata.rs` | **789** | 602 / 187（15 个 `#[test]`） | **合理（同族解析器），可拆但不必现在拆** |

**为什么它还算合理**：文件干的事是「两种元数据格式的解析」——
`CcmMetadata`（46-89 结构 / 90-172 impl，`metadata.txt` 的 `键=值`）、
`StandardMetadata`（173-218 / 219-329，`metadata.json`）、
`MiyinExtensions` / `PatchExtensions`（338-384，`miyin` 扩展命名空间）、
`PackageKind`（395-416）、`CampaignType`（417-603，五个资料片的判定 + 打表）。
**全是同一件事的不同侧面**（AGENTS §14.4：`docs/package-format.md` 是唯一权威），
彼此调用密集，拆开的收益低于 `package.rs`。
**如果哪天要拆**：`metadata/{ccm.rs, standard.rs, campaign_type.rs}`，
按 AGENTS §14.4 的规矩，改解析必须同步 `docs/package-format.md` 与 `MIYIN_FORMAT_VERSION` ——
所以**拆它要连带改文档**，这也是「先别拆」的理由。

### 4.1 600–700 观察名单（顺带看一眼，暂时不用管）

| 文件 | 行数 | 生产 / 测试 | 判断 |
| --- | ---: | ---: | --- |
| `crates/miyin-core/src/library/install.rs` | 694 | 694 / 0 | **合理**：统一安装引擎，`Manifest` 的 impl 一块就 385 行（175-560），是单一职责。它自己就是别的文件「该拆到哪」的样板 |
| `crates/miyin-core/src/dev/export.rs` | 649 | 273 / 376 | **合理**：生产只有 273 行，56% 是测试（275-649） |
| `crates/miyin-core/src/library/store.rs` | 612 | 612 / 0 | 可留意：`import()`（20-175，156 行单函数）与 `remove_variant` / `update_variant` 是两个流向；未过线，先不拆 |

---

## 5. 优先级：先拆哪个最值

排序依据四条：**① 最长单函数 / 最杂的职责数 → ② 引用面（改动影响多少人）→ ③ 有没有测试护着（敢不敢动）→ ④ 是不是用户数据/契约（拆错的代价）**。

| 优先级 | 文件 | 先拆什么 | 为什么排这里 | 风险 |
| :---: | --- | --- | --- | --- |
| **P0** | `crates/miyin-core/src/campaign/package.rs` | 抽 `paths.rs`（404 行纯函数）→ `extract.rs` → 把 `inspect()` 454 行按 6 阶段提成私有函数 | **单个业务函数最长**（636-1089），且每个阶段都有现成注释划界；461 行测试做安全网；14 个引用方但只通过 `run`/`inspect` 两个入口 | 低-中（三处顺序依赖不能动：`package.rs:881-889`、`894-897`） |
| **P1** | `ui/src/api/bridge.ts` | 把演示夹具搬进 `api/demo/fixtures/*`，`bridge.ts` 只留 123 行桌面桥 | **25 个文件 import 它，其中 96% 的行用不上**；纯数据搬迁，零业务风险 | 低（保住夹具的模块级单例、`isDesktop` 只在一处判） |
| **P2** | `crates/miyin-core/src/library/mod.rs` | 先加 `pub use` 转发层（风险 0 的一步）→ 再搬 `model.rs` / `main_map.rs` / `data_root.rs` / `conflict.rs` | 21 个引用方的入口、4 个互不相关的领域；**它是「胖 mod.rs」的收尾**，拆完 `mods.rs` 才有落点 | 中（21 个引用方；`impl Library` 的字段与方法必须同文件） |
| **P3** | `crates/miyin-core/src/library/mods.rs` | `export.rs` + `diff.rs`（各 ≈170-190 行，最独立）→ `deploy.rs` → `import.rs` | 1555 行生产代码只有 92 行内联测试，是「最不敢改」的一份；但导出/比对两块跟导入完全无关，拆它们不碰风险区 | 中（`import_one` 的目录形态判断靠 `tests.rs:1109-1806` 兜） |
| **P4** | `crates/miyin-core/src/saves.rs` | `scan.rs` → `targets.rs` → `backup.rs` → 最后 `isolation.rs` | 生产 1180 行里 7 个职责（4 个独立概念），但**调用面只有 1 个**、1418 行测试护着 → 收益是阅读、不是改动 | **高**（用户存档数据） |
| **P5** | `ui/src/react/views/campaigns/SlotMenu.tsx` | 删版本 Sheet → 依赖模组 → 编辑元数据 → 补丁面板 | 782 行单组件 + 25 handler；但**先要按 AGENTS §17 截图确认布局**，验证成本最高 | 中（无测试；纯 UI） |
| **P6** | `src-tauri/src/lib.rs` | 先搬平台实现（`base64`/`webview`/`crash`），再按 15 组搬命令 | 106 条命令同一文件；搬迁机械、漏注册是编译错。但**它不阻塞任何人**：命令之间互不调用 | 低（唯一硬约束：命令名是 IPC 契约，不能改名） |
| **P7** | `crates/miyin-core/src/library/tests.rs` | 夹具先出，再按 6 个域搬 | 纯测试搬运、零功能风险；收益是「新测试有落点」 | 极低 |
| **不动** | `ui/src/api/types.ts`、`ui/src/ide/ide.css` | —— | 类型与样式的行数不代表复杂度；拆了只增加 import 噪音 | —— |

**如果只能做一件事**：拆 `package.rs` 的 `inspect()`（P0）。
一个 454 行的函数里塞了「安全闸门 → 元数据 → 镜像布局 → 载荷 → 归属判定 → 命名」六个阶段，
是全仓唯一「读一个函数要记住六个前提」的地方。

**如果只能做两件事**：加上 `bridge.ts`（P1）—— 它影响的是**每天都要读它的人**（前端 25 个引用方），
而修起来最安全（只搬演示数据）。

### 5.1 不建议做的事

- **不要为了凑行数切文件**。`saves.rs` 的测试占 55%、`package.rs` 占 27%、`metadata.rs` 占 24% ——
  把测试搬走能立刻让「行数」变小，但那不解决任何耦合。
- **不要顺手改公共 API 名**。`library/mod.rs` 有 21 个引用方、`src-tauri` 的 106 条命令名写在前端
  `invoke("...")` 字符串里。拆分只搬文件、不搬名字（AGENTS §11.3「不擅自更改公共 API」）。
- **不要同时动 `library/mod.rs` 和 `library/mods.rs`**。两者互相引用，
  必须先把 `mod.rs` 的转发层立起来（P2 第 1 步），再动 `mods.rs`。

---

## 6. 附录：门禁原始输出（`python scripts/check-file-size.py 800`）

```
文件行数门禁：9 份原始代码超过 800 行 —— 高耦合风险，值得拆。
（行数只是信号：拆之前先想清楚它到底在干几件事，别为了凑数字而切碎）

按行数排：
   3190  crates/miyin-core/src/library/tests.rs
   2598  crates/miyin-core/src/saves.rs
   2573  src-tauri/src/lib.rs
   1688  crates/miyin-core/src/campaign/package.rs
   1647  crates/miyin-core/src/library/mods.rs
   1522  crates/miyin-core/src/library/mod.rs
   1507  ui/src/api/bridge.ts
    965  ui/src/api/types.ts
    810  ui/src/react/views/campaigns/SlotMenu.tsx
```

与本文口径的差异：**只差 `ui/src/ide/ide.css`（811 行）** —— 脚本的 `EXTENSIONS`（`scripts/check-file-size.py:23`）不含 `.css`。
另外脚本走 `git ls-files`，未跟踪文件不会出现（本期没有未跟踪的源码大文件，已核对）。

（同一脚本换 500 线会报 16 份，多出 `library/install.rs` 694、`dev/export.rs` 649、
`library/store.rs` 612、`compose.rs` 595、`update/check.rs` 587、`InstallView.tsx` 572、
`library/export.rs` 555、`campaign/contents.rs` 543、`ImportDialog.tsx` 539、`miyin-cli/tests/cli.rs` 509
—— 这些都在本文 §4.1 的判断范围内，不必逐个拆。）

---

## 7. 一句话收口

**>800 行的 10 份里，只有 2 份是「一个函数干六件事」的真问题（`package.rs` 的 `inspect()`、
`SlotMenu.tsx` 的组件），3 份是「多个职责同住」（`saves.rs` / `mods.rs` / `library/mod.rs`），
1 份是「演示数据挤进桥接层」（`bridge.ts`），剩下 4 份（测试聚合 / 纯类型 / 样式 / 命令表）
行数大但没有耦合可拆。**
先拆 P0 的 `inspect()`，再拆 P1 的 `bridge.ts`；其余按 §5 的顺序，一次只动一份，每动一次跑一遍
`cargo test --workspace` 与 `pnpm -C ui build`。





