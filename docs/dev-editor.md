# 开发者页 · 编辑器设计（v2）

> 状态：**设计稿**（只有"要做什么、为什么"，不含实现）。
> 相关：[包格式](package-format.md) · [打包指南](package-authoring.md) · [前端](frontend.md) · AGENTS §14（战役库与落点）、§10.5（写盘闸门）、§18.1（控件）。

## 0. 一句话

把现在的「扫游戏目录 → 打包」扩成**三个入口、两个视图**：
工作区（游戏目录 / 任意文件夹 / 压缩包）→「编辑」→「落点预览」。

## 1. 现状与问题

- 编辑器只能对着**游戏目录**工作：`dev_scan` 依赖"已安装的游戏"（`State::installation`）。
  拿一个 zip、或一个散落文件夹，根本开不了工。
- **「包内路径 → 游戏内路径」这条换算只存在于代码里**（`library::compose::Placement` +
  `payload_target_path`）。作者在界面上看不见，于是 §14.5 那些坑（`maps/` 顶层要展平、
  子目录怎么解释、镜像根）只能靠猜 —— 猜错的表现是"游戏里找不到地图"。
- 没有 metadata 的包，只能先导入战役库、看识别结果，再回头补 metadata（或者手写）。
- 落点覆盖（`miyin.overrides`，格式 v3 已有）在界面上**没有入口**。

## 2. 目标（用户提的四条）

1. 编辑器仍是开发者页里的**子页面**，并允许再开**另一个子界面**。
2. 编辑一个打包好的项目结构，**预览并可调整"实际安装到游戏后"的具体目录**。
3. 为**没有 metadata 的压缩包**快捷生成 metadata。
4. **任意文件夹**都能作为工作区打开，并在其中做 metadata 调整与安装位置设置。

## 3. 核心概念：工作区（Workspace）

三个入口，一个模型：

| 来源 | 怎么打开 | 可写? | 说明 |
| --- | --- | --- | --- |
| 游戏目录（默认） | 现有 `dev_scan` | 读；写一律走导出 | 保持今天的行为 |
| 任意文件夹 | 新命令 `ws_open_folder` | 读 | 若选中的是整个游戏安装 → 提示改选子目录 |
| 压缩包（zip/7z/rar/tar） | 新命令 `ws_open_archive` | 解到临时工作区后编辑 | 大包要进度 + 可取消 |

统一成 `Workspace { id, kind, root | archive, entries[] }`，**两个视图都吃同一个模型**。

## 4. 视图 A ·「编辑」（沿用现有，补两块）

- 文件树 / 文本·图片预览 / 包信息表单 / 导出 / 历史（现有）
- **新增**：工作区切换（工具栏左侧）：游戏目录 · 打开文件夹… · 打开压缩包…
- **新增**：metadata 生成向导入口（工作区里没有 metadata 时出现，见 §6）
- 历史/版本管理仍只对"游戏目录 + 库里的包"有意义：压缩包与外部文件夹工作区**不参与**。

## 5. 视图 B ·「落点预览」（新）

这是本次的核心新界面。

**输入**：工作区条目 +（槽位 / 版本目录名 / `target_sub`）+（若是补丁：层与优先级）

**每条算一次** `payload_target_path(PayloadTarget, Placement)`，得到三列：

```text
包内路径                        判定                     游戏内路径
swarm/metadata.txt              战役（虫群之心）          Maps/Campaign/swarm/metadata.txt
maps/paiur01.SC2Map             顶层分类目录 → 只取文件名   Maps/Campaign/swarm/paiur01.SC2Map
Maps/Campaign/...（镜像根）      镜像根 → 按包内绝对路径     Maps/Campaign/...
foo.SC2Map（自制战役）           —                        （留在库里，等编辑器打开）
```

- 「判定」一列就是 §14.5/§14.7 的规则本身，**把代码里的规则变成作者看得见的一句话**。
- **逐条覆盖**：编辑 `miyin.overrides`（`{from, to}`，格式已有），改完实时重算。
- **冲突提示**：两个条目落到同一路径 / 会覆盖官方文件（会被备份）/ 越界（写盘闸门会拒）。
- **保存**：写回工作区的 metadata；压缩包工作区 → 另存为新 zip（不就地改写，见 §10）。

## 6. metadata 生成向导

- **触发**：工作区里既没有 `miyin/metadata.json` 也没有 `metadata.txt`。
- **识别**：跑现成的证据链（`campaign::identify`）——镜像路径 / 地图内部依赖 / 文件名前缀，
  给出归属 + **置信度**（`CampaignEvidence::is_exact()`）。
- **预填**：包名（取文件名）/ 版本 / 作者 / 注册 ID / 标签 / `main_map` / `mods`。
- **校验**：复用 `library::naming`（注册 ID、版本号）。
- **写入**：`miyin/metadata.json`（格式见 `package-format.md` §1.2）。
- **措辞**：自动识别一律标「推断」，**不许说成确定的**（AGENTS §14.7）。

## 7. 需要新增的 IPC 命令

| 命令 | 作用 | 复用什么 |
| --- | --- | --- |
| `ws_open_folder(path)` | 扫任意目录成 Workspace | `dev::scan` 去掉 installation 依赖 |
| `ws_open_archive(path)` | 读压缩包条目 / 解到临时区 | `campaign::contents::Contents` |
| `ws_detect(ws)` | 识别归属与置信度 | `campaign::identify` 三层 |
| `ws_placements(ws, slot, folder, sub, kind)` | 落点预览 | `compose::Placement` + `payload_target_path` |
| `ws_write_meta(ws, meta)` | 写 metadata | `metadata` 模型 + `naming` 校验 |
| `ws_save_archive(ws)` | 另存 zip | `dev::export::export` |
| `ws_reveal(path)` | 资源管理器里打开 | 已有 `reveal_path` |

**产物**：所有新命令保持 `#[tauri::command(async)]`（AGENTS §16），长活带进度与取消
（复用 `dev_cancel_export` 那套）。

## 8. 第三方库候选（能省事就用，不能就手写）

### Rust

| 库 | 用途 | 取舍 |
| --- | --- | --- |
| **`notify`** | 监听"任意文件夹"工作区的改动 | **推荐**：外部改了文件要能刷新；小、成熟 |
| `walkdir` | 目录遍历 + 忽略规则 | 可选：现有 `dev::scan` 手写够用，除非要加 `.gitignore` 式忽略 |
| `similar` / `imara-diff` | 文本 diff | 可选：历史面板要做"跟上一版差在哪"时用；现在是 path+size+时间 |
| `trash` | 删文件进回收站 | 可选：比硬删安全（AGENTS §10.5 要求可恢复） |
| `globset` | `overrides.from/to` 的通配 | 可选：现在按前缀匹配就够；作者要通配再加 |
| `mime_guess` | 文件类型 → 图标 | 可选：省一张后缀表 |
| `image` | 封面缩略图 | 可选：浏览器 `<img>` 已够；只在要"缩略图列"时才需要 |
| `zip` / `flate2` / `tar` | 压缩格式 | **已有**（AGENTS §3.1），不新增 |
| `sevenz-rust` / `unrar` | 原生读 7z / rar | **不推荐**：现在走系统 `tar` / 7-Zip 已经覆盖（`contents.rs`），多两套解析器不划算 |

### 前端

| 库 | 用途 | 取舍 |
| --- | --- | --- |
| **`@codemirror/*`（CM6）** | 真正"编辑"文本（现在只有只读预览） | **推荐**（若要做编辑）：模块化、离线可用、构建简单 |
| `monaco-editor` | 同上 | **不推荐**：体积与构建复杂度换不来这个页面的收益 |
| `@tanstack/react-virtual` | 几千文件的列表虚拟滚动 | **视情况**：Ring UI 没有树的虚拟实现；包大到几千项才需要 |
| `pdfjs-dist` | 说明书的 PDF 预览 | **视情况**：体积大，"用系统打开"更划算 |
| `react-arborist` / `rc-tree` | 树控件 | **不推荐**：AGENTS §18 要求控件来自 Ring UI 或自己的控件层；树已经是手搓的 |
| `zod` / `yup` | 表单校验 | **不推荐**：校验规则在后端 `naming.rs`，前端再来一套必然打架 |
| 状态库（redux / zustand） | — | **不推荐**：现有 context + hook 够用 |

## 8.5 「更 JB 的体验」：用 Ring UI 已有的，不引新库

**结论先说**：JetBrains 的 UI 库就是 **Ring UI**（`@jetbrains/ring-ui-built`，已装 7.0.130）——
所以「更 JB」不是装新包，而是**把库里更 JB 的组件用起来**。

| 已确认存在的 JB 包 | 是什么 | 我们 |
| --- | --- | --- |
| `@jetbrains/ring-ui-built` 7.0.130 | 官方 React 组件库（80+ 组件） | **已用**（19 种，还有 60+ 可用） |
| `@jetbrains/icons` 5.23.4 | 官方图标集 | **已用** |
| `@jetbrains/logos` 3.1.0 | 产品 logo（IntelliJ / WebStorm …） | 未用；页头想要产品味可加 |
| `@jetbrains/eslint-config` 6.0.5 | 官方 lint 规则 | **不建议**：会引入整套 eslint 工具链，与现有门禁重复 |

（`@jetbrains/foxtrot`、`@jetbrains/babel-preset` 在 npm 上**不存在**，别再找。）

### 该用起来的 JB 组件（按本页的价值排）

| 组件 | 用在哪 | 为什么更 JB |
| --- | --- | --- |
| `content-layout` + `header` / `footer` + `island` | 替换手搓的 `.pane` / `.pbar` / `.pbody` | IDEA 那种「面板岛」的骨架，边距与分隔线由库给 |
| **`query-assist`** | 文件搜索框 | JB 的搜索式输入：`ext:SC2Map` 这类操作符、命中高亮、补全 —— 比裸 Input 强一个量级 |
| **`shortcuts`** | 导出 / 提交 / 搜索的快捷键提示 | JB 的快捷键气泡；只写浏览器里能用的组合 |
| `table` / `data-list` | 历史与提交列表 | 列宽、表头、密度由库管；现在是拿 ul/li 拼的 |
| `editable-heading` | 包名 | 双击就地改名 —— JB 的重命名手势 |
| `code` | 包内路径、元数据键名 | 行内等宽码样式 |
| `markdown` | 包内说明文档预览 | JB 的 markdown 渲染，比 marked 更贴 IDE |
| `alert` / `alert-service` / `confirm-service` | 提示与破坏性确认 | 状态栏之外的 JB 式通知 |
| `collapse` / `group` / `grid` | 右栏表单分组 | JB 表单的分组与栅格 |
| `banner` / `avatar` / `logos` | 页头（可选） | 产品味，不是必需 |

### 落地注意

- **浏览器不是 IDE**：Ctrl+W、Ctrl+Shift+… 会被浏览器 / webview 抢走 —— `shortcuts` 只写能用的组合。
- `island` 自带内边距，工具页要紧凑就得覆盖 —— 按既有做法**就近写一行理由**。
- 先换骨架（`content-layout` + `header`/`footer` + `island`），再逐块试 `query-assist` 与 `table`。
## 9. 分期

- **P0**：工作区抽象（三来源）+ 落点预览（只读）—— **不加任何依赖**
- **P1**：`overrides` 编辑 + metadata 生成向导
- **P1.5**：JB 骨架 + `query-assist` 搜索 + `table` 历史
- **P2**：压缩包另存 / 外部改动刷新（`notify`）/ 文本编辑（CodeMirror）
- **P3**：PDF 预览 / 虚拟滚动（按真实包大小决定）

## 10. 风险与边界

1. **写盘闸门**（§10.5）：落点只能在游戏目录内。预览里越界的行要**标红并写明"会被拒绝"**。
2. **自制战役的图不进游戏目录**：`payload_target_path` 返回 `None` —— 落点表必须如实写
   「留在库里，等编辑器打开」，不能留空白让人以为丢文件。
3. **压缩包工作区不是就地改**：解到临时区 → 改 → 另存。就地改写 zip 不安全（写到一半崩了就是坏包）。
4. **大包**（几 GB）：解压/扫描要有进度条与取消，复用导出那套信号。
5. **历史/版本管理**只对"游戏目录 + 库里的包"有意义，压缩包工作区不参与。

## 11. 验收（怎么算做完）

- 拿一个**没有 metadata 的 zip**：打开 → 识别出归属与置信度 → 生成 metadata →
  落点预览 → 导出，**全程不离开这个页面**。
- 拿一个**散落文件夹**：打开 → 改 metadata → 看落点 → 导出。
- 落点表与**真实安装结果逐条一致** —— 这一条必须真装一次对比，不能只看截图。

## 12. 待定

- 落点表的"逐文件覆盖"粒度：`overrides` 现在是 `from`（前缀）→ `to`（目录）。
  够不够，还是要"单个文件精确指定"？先按前缀做，不够再扩格式（要提 `miyin.format`）。
- 压缩包工作区的临时目录放哪：`%TEMP%miyin-dev\<hash>` 还是 `data/dev/workspaces/`？
  倾向后者（可复用、可清理），但要加"清缓存"入口。
- 「另一个子界面」的形态：视图内页签，还是拆成两个 hash 路由（`#dev/place`）？
  倾向 hash 路由 —— 和现有 `#gallery` 一致，能直接分享/截图。
