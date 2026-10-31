# 启动 / 挂载全链路审计

> **只读审计**：本文不改任何代码。审计时点 **2026-10-29 16:1x**，工作区当时含 Lead 未提交的修复
> （`git status`：activation.rs / install.rs / mod.rs / tests.rs / lib.rs / ModsView.tsx / mods.css）。
> 结论分两类：**事实**（逐行读过代码，给 文件:行号）与**推断**（未实测，会标注）。行号以写入时的工作区为准，
> core 与 tauri 两侧仍在改动，之后可能漂移几十行 —— 按函数名检索更稳。

---

## 0. 一句话结论

「用配置当实际状态」这条类别里，**最出名的那个洞已经修好了**（`variant_mods` 不再按 `slot.active` 门控，
新增 `in_game`，启动拦截改看配置，`AGENTS §14.12` 也把这条约定写下来了，文档与代码没有漂移）。
但同一个根因还剩**七处**在用（第 5 节逐条列），其中三处会造成**用户数据损坏**
（存档串档 R1、游戏目录半装 R2、清单与磁盘永久分叉 R3）；而新做的「启动时自动挂载」目前**只修 Mods/、失败只进 stderr、
撞车会互相抢**，需要三条护栏才算收口。

---

## 1. 现场：现在到底有几条会写游戏目录的路径

| 入口 | 位置 | activate | 切存档组 | 问「游戏在跑吗」 | 写盘前给用户看 |
| --- | --- | --- | --- | --- | --- |
| 启动游戏（顶栏 / 战役页「启动」） | `lib.rs:1873-1891` | ✗ 只 sync 模组 | ✗ | ✗ | ✗ |
| 启动进这张图（自制战役面板） | `lib.rs:2481-2500` → `prepared_map` `lib.rs:2549-2573` | ✓ | ✗ | ✗ | ✗ |
| 用编辑器打开地图 | `lib.rs:958-1003` | ✓ | ✗ | ✗ | ✗ |
| 启用 / 停用版本 | `lib.rs:832-864` | ✓ | ✓ | ✗ | ✓（自制面板除外，见 R11） |
| 启用 / 停用独立模组 | `lib.rs:1350-1364` | — | — | ✗ | ✓ |
| 启动器启动（bootstrap） | `lib.rs:145-165` | ✗ 只 sync 模组 | ✗ | ✓（`lib.rs:150`） | ✗ 只 stderr |
| 删除版本 | `store.rs:176-215` | 视 `slot.active` 而定 | ✗（`activate(None)` 不切组） | ✗ | — |
| 更新版本（replace_variant） | `store.rs:501-582` | 视 `slot.active` 而定 | ✗ | ✗ | ✓（导入框） |
| CLI | `crates/miyin-cli/src/main.rs:127-133` | — | — | — | — |

**CLI 的结论**：`miyin-cli` 只有 `inspect` / `identify` / `placements` / `compat` 四条（`main.rs:127-133`，
与 `docs/cli.md:9` 一致），**没有任何启动 / 激活命令** —— 它不在这条写盘链上，也没有文档漂移。
代价是脚本化「装好再启动」走不通，见 R15。

---

## 2. 已修（Lead 第一版修复）后的语义

写在这里，免得后面把已修的当没修：

- `variant_mods` / `all_mods`：`mounted` = **用户配置**，`in_game` = **真的铺在 `<游戏>/Mods/` 里**
  （`mod.rs:773-806`、`mod.rs:823-860`、`mod.rs:790-806` 的 `installed_mod_keys` 按安装清单算）。
- 启动拦截：`require_mods_configured` 看配置（`lib.rs:240-262`），两条路径共用
  （`open_map_in_editor` `lib.rs:971`、`prepared_map` `lib.rs:2557`）。
- 启动时自动挂载：`activation::sync_mods`（`activation.rs:134-231`）+ bootstrap 调用（`lib.rs:145-165`）
  + `launch_game` 前再补一次（`lib.rs:1876-1882`）；写盘只落在 `Mods/`（`plan_mods` `activation.rs:184-207`，
  `apply_scoped` `install.rs:421-491`、`remove_scoped` `install.rs:540-...`）；幂等靠 `is_settled`
  （`activation.rs:209-231`）。

---

## 3. bug / 风险表（按严重度）

| # | 严重度 | 现象 | 文件:行号 | 根因 | 影响面 | 建议修法（一句话） |
| --- | --- | --- | --- | --- | --- | --- |
| R1 | **高** | 「启动进这张图」「用编辑器打开」「删除正在启用的版本」都会 `activate`，但**不切存档组** → 玩 B 战役时 Banks 还挂在 A 的组上，下次切组会把 B 的进度**覆盖写进 A 的组** | 唯一会切组的：`lib.rs:843-861`；不切的：`lib.rs:2559`、`lib.rs:974`、`store.rs:199`；覆盖机制：`saves.rs:419-433` → `save_into_dir` `saves.rs:400-416`（先清空再拷） | 存档组切换被绑在「`activate_variant` 这条 Tauri 命令」上，而不是绑在「激活」这件事上 | 开了存档隔离的用户：进度串档 / 被覆盖（只剩 restore 时那份「还原前-时间戳」，`saves.rs:477` 注释）；**不可自动恢复** | 把「激活 + 切存档组」下沉成 core 的一个函数，三条进游戏路径 + 删除/更新共用。 |
| R2 | **高** | `activate` 中途失败（拷不动 / 文件被占 / 磁盘满）后：旧版本已全部撤下、新版本只铺了一部分、`index.active` 还是旧的 → 新 owner 那部分文件**永远撤不掉**；自动挂载还会把半装那版的模组铺回去 | `activation.rs:98-103`（`apply` 成功后才写 index）；`install.rs:480-487`（单条失败只撤这一条）；`activation.rs:59-72`（撤谁只看 `index.active`） | 「成功才更新选中」缺了配对的**回滚** | 游戏目录半旧半新；切回原版清不干净；清单 / 界面 / 磁盘三方不一致 | `apply` 失败时对 `plan.owner` 做 `manifest.remove` 回滚；把「撤上一位」改成按清单算。 |
| R3 | **高** | 安装清单里 `source` 存的是**绝对路径**；数据根一搬（旧 `data/` 迁移、游戏目录整体搬走、换机器同步库）就对不上 → `target_of` 返回 `None` → 打开地图**静默退回库里那份**，而注释明确说库里那份打不开 | 写：`install.rs:476`；比对：`install.rs:346-354`（字符串精确相等）；回退点：`lib.rs:988-991`、`lib.rs:2567-2570`；搬迁场景：`mod.rs:1181` `resolve_data_root` | 清单把「位置」记成了绝对路径，而库根**本来就是可变的**（`AGENTS §14.1` 的迁移逻辑） | 编辑器打开地图失败 / 游戏加载失败；回退无任何提示，用户只会看到「无法打开地图」 | 存相对库根的路径（或 `slot/variant + 库内相对路径`），读取时用当前库根重拼。 |
| R4 | 中高 | 删除 / 更新用 `slot.active` 判「是否装着」：真装着但 `active` 不是它时，删版本直接删库目录，游戏目录里的文件 + 清单记录全留下（幽灵） | `store.rs:191-201`、`store.rs:519`+`573`；对照 `activation.rs:112-125`（清单里有几条就是几部）与测试 `activation.rs:309-357` | `active` 是「最后选中的」，被当成了「装着的」 | 幽灵文件永久残留、清单指向不存在的源、后来别人的 takeover 警告误报 | 判「装着没」统一用清单（`of(owner)` 非空 / `enabled_variants`）。 |
| R5 | 中高 | 游戏运行中仍可启用 / 铺盘：`sc2_running` 命令**没有任何调用方**，而被占用文件的删除失败还被 `let _ = ` 吞掉 | 命令：`lib.rs:499-502`；全仓前端 0 处调用（grep `sc2_running\|sc2Running` 无命中）；吞错：`install.rs:568-570`、`install.rs:520-522`；文档承诺：`AGENTS §12.2` | 安全底线只写在文档和命令里，没接到三条写盘路径上 | 游戏中切战役 → 清单说撤了、文件还在（账实分叉的制造机）；游戏可能半坏 | 所有写盘命令入口统一前置 `game_running()` 并拒绝。 |
| R6 | 中 | 自动挂载只修 `Mods/`：重装游戏 / 用户清过目录后地图仍缺，但启动「成功」、卡片还显示「已启用」 | `activation.rs:184-207`（只取 `Mods/` 前缀）、`install.rs:421`（scope）；对照 `mod.rs:724-727`（「装了没」看清单） | 修复范围刻意收窄，但**没有「不完整」的检测** | 用户点启动 → 游戏进原版 / 地图不存在，且没有任何提示 | sync 时核对地图文件存在性，缺了就返回一条明确 warning（不要静默成功）。 |
| R7 | 中 | 自动挂载 / 独立模组同步的提示只 `eprintln!`；release 没有控制台 → 失败对用户**完全不可见** | `lib.rs:145-165`；`lib.rs:38`（注释：release 没有控制台）；现成的失败出口：`BootStatus` `lib.rs:71-75` + `boot_ready` `lib.rs:2243` | 复用了「后台日志」通道，而不是「用户可见」通道 | 自动挂载静默失败 = 用户以为好了 | `BootStatus` 加 `warnings`，`boot_ready` 顺带返回，前端 toast 一次。 |
| R8 | 中 | 启动时先 `activation::sync_mods` 再 `mods::sync`，两者对同一个 `Mods/X` 都是「接管」（release 对方）→ **每次启动抢一次、整包重拷**，谁生效取决于顺序 | 顺序：`lib.rs:153`、`lib.rs:161`；接管：`install.rs:436-448`；`is_settled` 下次又判未完成：`activation.rs:209-231` | 两个 owner 体系共用同一片 `Mods/` 命名空间，且都用「我来接管」解撞车 | 大模组反复拷写（慢、耗盘、伤 SSD）；`in_game` 抖动；行为不确定 | 自动挂载撞车时**跳过 + 告警**，takeover 只留给用户主动启用那条路。 |
| R9 | 中低 | 清单读坏 / 丢失时当空清单：磁盘上我们铺过的东西再也无人认领，只能手动清 | `install.rs:180-192`（改名 `.corrupt` 后返回空）；连带 `mod.rs:790-806` 的 `in_game`、`activation.rs:112` 的 `enabled_variants` 全按「没装」算 | 账本是唯一真相，没有「从磁盘重建账本」的路 | 残留文件 + 界面说没装；下次启用时它们以「官方文件」身份被备份，越滚越大 | 读坏时给出「账本损坏」的显式警示，或提供一次「扫 `Mods/` 与 `Maps/` 重建账本」的行动。 |
| R10 | 中 | 「用编辑器打开地图」是**隐藏激活**：点一下会把当前槽位已启用的版本撤下、装这一版 | `lib.rs:974`（`activate` 在 `open_map_in_editor` 里） | 为了让编辑器找到地图，顺手把整个战役重装一遍 | 用户只想看一张图，结果游戏目录换了版本；无预览 / 无确认 | 先在清单里查「这一版的地图是否已在位」，在位就不 activate；否则显式提示「会切换」。 |
| R11 | 中低 | 自制战役面板的启用 / 停用**绕过预览与冲突**两道闸（战役页有，这页没有） | `CustomCampaignPanel.tsx:123-134` 直调 `api.activateVariant`；对照 `useLibrary.ts:163-181`（先 `activationConflicts` 再 `previewActivation`） | 面板自己造了一条写盘入口 | 写盘前不给用户看（`AGENTS §10.5` 第 2 条）；撞车时静默接管别人的文件 | 面板改调 `requestActivation`。 |
| R12 | 中低 | `launch_game` 什么也不校验：不 activate、不看 `active`、不检查文件在不在 —— 玩到的是磁盘现状 | `lib.rs:1873-1891` | 「启动游戏」被当成纯 spawn，与库状态解耦 | 卡片显示「已启用 X」但磁盘是原版时，用户以为在玩 X（用户第一反应是「启动器骗我」） | 里面复用 sync + 一次完整性检查，缺了给提示。 |
| R13 | 低 | 合并时用本地记录**整体替换**目标记录，连 `mounted_mods` / `main_map` 一起换 | `merge.rs:197-207`（Local 分支 `*item = record`） | 记录当整体搬运，没区分「内容」与「用户配置」 | 另一台机器的模组勾选被静默覆盖 | 只更新内容相关字段，保留目标库的 `mounted_mods`/`main_map`。 |
| R14 | 低 | 同一张卡片上 `active`（意图）与 `vanilla_enabled`（清单）可以互相矛盾，UI 只读前者 | `mod.rs:754-764`（两个字段并列）；`SlotCard.tsx:15`（只看 `active_name`）；前端全仓不用 `vanilla_enabled` | 两个数据源并列输出，没有裁决规则 | 清单丢了但 `active` 还在 → 卡片显示「已启用：X」，实际游戏目录是原版 | 状态以清单为准，`active` 只表示「上次选的」。 |
| R15 | 低 | CLI 没有启动 / 激活命令；`deactivate` 成了无人调用的公开 API | `main.rs:127-133`、`docs/cli.md:9`；`activation.rs:23`（除测试无调用） | 功能只长在 GUI 侧 | 脚本化「装好再启动」走不通；「全部还原」没有一键出口（R2/R4 的孤儿更没人清） | 要么删掉 `deactivate`，要么接一个「全部还原」入口；CLI 要扩就复用同一套 core 函数。 |
| R16 | 低 | 新增的 `in_game` 前端**一次都没用**（`ui/src` grep 0 命中），勾选框只代表配置 | 字段：`mod.rs:196`、`mod.rs:253`；`useMods.ts:84-108` 仍按 `mounted` 反推 | 修复只做到后端 | 用户勾上后回列表看不出「还没生效」，下次启动才生效——像 bug | 模组行加「待挂载 / 需重启」标记（另一 agent 正在改这个视图，交给它）。 |

---

## 4. 启动时自动挂载的设计方案

### 4.1 三个落点对比

| 方案 | 落点 | 优点 | 代价 |
| --- | --- | --- | --- |
| **A（现在的实现）** | 后端 `AppState::bootstrap` 后台线程，`manage` 之前（`lib.rs:145-165`） | ① 与**所有**前端命令天然串行 —— `AppState` 还没 `manage`，前端一条命令都发不出来，不可能并发写清单（这是它最大的价值，`lib.rs:2243` 的注释也是这个意思）；② 用户零操作；③ 失败不拦启动 | ① 拖慢 `boot_ready`，首帧变慢（首次铺 GB 级模组时最明显）；② 写盘发生在任何 UI 之前，**无法征得同意**；③ 目前告警只进 stderr（R7）；④ 撞车会 takeover（R8） |
| **B** | 前端就绪后，在 `useShellEffects.ts:142-149` 里再挂一个启动期动作（该文件已有 bootstrap / merge / update / tools 四个定时动作，位置现成） | ① 拿得到 `warnings` 直接 toast；② 可不挡首屏；③ 能加「本次跳过」 | ① 和用户点击有**竞态**（用户点得比 sync 早，两条写盘路径并行）；② 多一条前端副作用；③ 浏览器演示模式要假装一遍 |
| **C** | 惰性：只在三条「进游戏」路径里 sync（`launch_game` 已经这么做了，`lib.rs:1876-1882`；`prepared_map` / `open_map_in_editor` 还未接） | ① 启动零开销；② 写盘发生在**意图明确**的那一刻；③ 天然幂等（`is_settled`） | ① 「Mods 被删 / 重装过游戏」要等用户点启动才自愈；② 三条路径都得接，漏一条就有一条不修 |

### 4.2 推荐：A 为主 + B 的「告知」+ C 兜底

**保留 A**（理由就是上表第 ①：唯一能保证「写盘没有任何并发」的落点；B 单独用会引入竞态，C 单独用会让
「重装游戏后模组全无」一直挂着直到用户点启动）。但必须补三条护栏，否则 A 会自己制造新问题：

1. **失败与跳过必须可见**（补 R7）：`sync_mods` 的 `warnings` 不要只 `eprintln!`，塞进已有的 `BootStatus`
   （`lib.rs:71-75`），让 `boot_ready`（`lib.rs:2243`）返回给前端 —— `boot.ts:14-17` 现在是 `Promise<void>`，
   改成收一个数组即可，`BootGate.tsx:19-32` 之后 toast 一次（最多 3 条，其余进日志）。
2. **不抢别人的目标**（补 R8）：`apply_scoped` 在 step 2 发现 `owner_of(target) != plan.owner` 时，
   自动挂载应当**跳过 + 记 warning**，而不是 `release` 对方（`install.rs:436-448`）。takeover 只属于
   「用户主动点启用」那条路，那里有冲突对话框（`ActivationConflictDialog`）。
3. **不全静默成功**（补 R6）：`is_settled` 现在只看 `Mods/` 的记录与存在性（`activation.rs:209-231`），
   顺手加一条「这个 owner 在地图落点上是否还有记录/文件」，缺了就返回一条「上次启用的版本文件不完整，
   请在战役页点一次启用」—— 只提示，不自动铺地图（启动时改 `Maps/` 的风险大于收益）。

### 4.3 五个必答问题

| 问题 | 结论 |
| --- | --- |
| **挂载范围怎么定** | 只 `Mods/`，且只铺「配置里挂着的」（`effective_mounted_mods`，`mod.rs:261`；`compose.rs:269-281` 已经按它过滤）。地图 / 补丁 / Interfaces 一律不自动铺，只检测并提示（见护栏 3）。 |
| **多自制战役共存怎么办** | 盘上不冲突：各自落在 `Maps/CustomCampaigns/<variant>/`（`Placement`，`compose.rs`；槽位只有 `custom` 一个）。唯一会撞的是 **`Mods/` 同名**。规则建议：① `index.active` 那一版优先；② 其余按 (slot, variant) 排序，先到先得；③ 后到的**跳过并告警**，绝不互相 takeover。不要按「配置里谁新」决定——那会让结果随操作历史漂移。 |
| **失败怎么提示** | 启动类失败 → `BootStatus` → `boot_ready` → 前端 toast（护栏 1）；启动游戏前后的失败 → 直接在 `launch_game` 的返回里带 `warnings`（现在这个命令返回 `()`，加一个返回值不算破坏）。**都不拦启动**。 |
| **幂等性** | `is_settled`（`activation.rs:209-231`）的思路是对的：比对「清单里这个 owner 在 `Mods/` 的记录」与「计划」+ 每个目标文件是否存在。再补两点：① 比对按 Windows 规则忽略大小写（复用 `activation.rs:206-212` 的 `same_path`）；② 稳态（没改配置、文件都在）不写盘 —— 所以正常启动的成本只是「每部启用的战役一次 compose + 一次清单比对」，很便宜，A 的「拖慢启动」只在首次/搬运后才真实存在。 |
| **性能** | ① 稳态零写入（见上）；② 一改配置就整包重铺：`apply` 的语义是「计划就是当前状态」，先 `remove_scoped` 再全量拷（`install.rs:428-429`）—— 勾一个模组会让该版**全部模组**重拷一遍，大模组（`Alenger` 那种十几个文件的）能到 GB 级；③ 撞车跳过（护栏 2）能去掉每次启动的重拷；④ `game_running` 时整体跳过（已有，`lib.rs:150`）。如果以后要优化，优先级是 ② > ③。 |

---

## 5. 哪些地方把「配置」和「实际状态」混用了

> 这是本次 bug 的类别，逐个列。**✅ 已修 / ⚠️ 仍在**。

| # | 位置 | 混用了什么 | 状态 |
| --- | --- | --- | --- |
| 1 | `mod.rs:773-806` `variant_mods` | 原来用 `slot.active` 门控 `mounted`（没启用 → 一个都不算挂载） | ✅ 已修：`mounted`=配置，`in_game`=清单 |
| 2 | `mod.rs:823-860` `all_mods` | 同上 | ✅ 已修 |
| 3 | `lib.rs:240-262` 启动拦截 | 原来是「`mounted` 全 false」当「没挂」 | ✅ 已修；小瑕疵：错误文案仍写「打开地图会报错」，实际它只保证**配置**，不保证铺盘（`lib.rs:250-258`） |
| 4 | `activation.rs:59-72` `activate` 撤上一位 | 用 `index.slots[slot].active` 决定撤谁，而不是清单 | ⚠️ 仍在（R2/R4 的根） |
| 5 | `store.rs:191` `remove_variant` / `store.rs:519` `replace_variant` | `slot.active == id` 当「这一版装着」 | ⚠️ 仍在（R4） |
| 6 | `mods.rs:69-90` `mods::sync` 的 `why_not` | `index.slots.active` 当「这个战役启用中」 | ⚠️ 仍在（判断对象是独立模组的来源战役，影响面小，但同一条错法） |
| 7 | `mod.rs:754-764` `SlotView` | `active`（意图）与 `vanilla_enabled`（清单）并列输出且无裁决，UI 只读前者（`SlotCard.tsx:15`） | ⚠️ 仍在（R14） |
| 8 | `install.rs:346-354` + `install.rs:476` | 清单里的「实际位置」存成了绝对路径；失配后 `lib.rs:988-991 / 2567-2570` 把**库里的假设位置**当兜底，而那个位置已知打不开 | ⚠️ 仍在（R3） |
| 9 | `install.rs:180-192` | 账本读坏 → 当「什么都没装」，把「账本」当成了「磁盘」 | ⚠️ 仍在（R9） |
| 10 | `useMods.ts:84-108`、`CustomCampaignPanel.tsx:137-153` | 勾选框状态用 `mounted` 反推（语义已随后端修复变正确），但界面没有「配置了 / 生效了」两态的表达（`in_game` 未被消费） | ⚠️ 半修（R16） |
| 11 | `CustomCampaignPanel.tsx:111`+`185` | `mountedCount === 0` 拦启动 | ✅ 随修复变正确（现在统计的是配置） |
| 12 | `activation.rs:134-175` `sync_mods` 的 `wanted` | 同时吃 `index.active`（意图）与 `enabled_variants`（清单），两个口径**并列且无冲突裁决** | ⚠️ 设计使然但缺规则：冲突时应以清单为准并提示（见 4.2 护栏 3） |

---

## 6. 结论：如果只能改三处

1. **把「激活」下沉成 core 的单一操作**：撤上一位（按清单）→ 铺新的（**失败回滚**）→ 切存档组。
   `activate_variant`（`lib.rs:832`）、`prepared_map`（`lib.rs:2549`）、`open_map_in_editor`（`lib.rs:958`）、
   `remove_variant`（`store.rs:176`）、`replace_variant`（`store.rs:501`）全部改调它。
   一次覆盖 **R1、R2、R4、R10**，并给 R5 一个统一的前置点（`game_running()` 只在这里问一次）。
2. **让失败与分叉用户可见**：`BootStatus` 加 `warnings` → `boot_ready` 返回 → 前端 toast；
   `launch_game` / `prepared_map` 的返回值也带上 warnings；自动挂载撞车**跳过不 takeover**。
   覆盖 **R6、R7、R8、R12**，并且让 R2/R4 留下的分叉至少不再静默。
3. **让清单可移植、且不再冒充磁盘**：`Installed.source` 改存相对库根的路径（`install.rs:476`），
   `target_of` 用当前库根重拼（`install.rs:346`）；读坏清单时**显式警示**而不是当空（`install.rs:180-192`）；
   `target_of` 失配时的库内回退要报错而不是静默（`lib.rs:988-991`、`lib.rs:2567-2570`）。
   覆盖 **R3、R9**。

> 如果还能改第四处：R14（`SlotView` 以清单为准 / UI 显示 `in_game`）—— 它是用户能直接看见的那一面。

---

## 7. 局限（没做的事）

- **没跑测试、没跑游戏**：本文全部结论来自读码，没有 `cargo test` / 实机验证；「R1 会覆盖存档组」
  是按 `switch → save_active → save_into_dir` 的调用链推的，属于**强推断**，建议先写一个
  `tempfile` 用例复现再修（`AGENTS §6` 要求修 bug 补测试）。
- **没审**：`saves.rs` 只读到 `switch`/`save_active`/`ensure_for_slot`；`patch` 绑定启停对
  `compose` 的影响；`ui/src/react/views/mods/**`（另一 agent 正在改）；`ide.html` 开发者页。
- **行号会漂**：Lead 正在改 `miyin-core` 与 `src-tauri`，本文行号以写入时工作区为准，之后按函数名检索。
