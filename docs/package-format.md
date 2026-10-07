# 弥音战役包格式 v1

> 本文档是**包作者**与**启动器实现**之间的契约。
> 启动器侧的实现见 `crates/miyin-core/src/campaign/metadata.rs` 与 `package.rs`。

## 0. 设计目标

1. **兼容既有生态**：CCM（Custom Campaign Manager）与「星际枢纽」的包不改一个字节也能导入。
2. **扩展落在可选字段**：我们自己的东西放在独立命名空间里，老工具忽略即可，不会解析失败。
3. **格式版本可协商**：包声明的扩展版本高于启动器支持时，**明确拒绝**而不是猜着解析。
4. **包不能决定落点**：压缩包里的任何路径都只是内容，装到哪里由启动器决定。

## 1. 容器与形态

容器统一是 **zip**。启动器按下面的优先级识别三种形态：

| 形态 | 判据 | 兼容对象 |
| --- | --- | --- |
| 弥音 / 枢纽标准包 | **包根**有 `metadata.json` | 星际枢纽标准 |
| CCM 包 | 包内**任意层级**有 `metadata.txt` | CCM 生态 |
| 无元数据包 | 两者都没有 | 兜底：仍可导入，名字取压缩包文件名 |

- 元数据文件不要求放在包根。`MyCampaign/metadata.txt` 是最常见的 CCM 布局：
  启动器会把 `MyCampaign/` 当作**内容根**剥离掉再落盘。
- 子目录里的 `metadata.json` **不算**标准包（会被当无元数据包处理并给出提示）。

## 2. `metadata.txt`（CCM 兼容）

`键=值` 逐行，大小写不敏感，值取**第一个** `=` 之后的内容。

| 键 | 必填 | 说明 |
| --- | --- | --- |
| `title` | 否 | 显示名。缺失时用压缩包文件名 |
| `desc` / `description` | 否 | 描述 |
| `author` | 否 | 作者 |
| `campaign` | **建议** | 归属资料片，决定自动导入到哪个战役（见 §4） |
| `version` | 否 | 自由字符串，不做 SemVer 校验 |
| `cover` / `image` / `icon` / `banner` | 否 | 封面图相对路径（**弥音扩展**） |
| `tags` | 否 | 标签，逗号 / 顿号 / 空格分隔（**弥音扩展**） |

> 与上游 CCM 的差异：上游用 `ToLower()` 比较键名，我们同样**大小写不敏感**；
> 上游把值按 `=` 切成多段，我们只切第一个，因此 `desc=得分 = 10` 不会丢内容。

## 3. `metadata.json`（枢纽标准 + 弥音扩展）

### 3.1 枢纽已有字段（原样兼容）

`name`、`description`、`version`、`author`、`type`、`campaign`、
`maps_directory`、`maps`、`mods_directory`、`mods`、`dependencies`、
`bank_enable`、`banks`、`manager`、`manager_mode`、`tags`、`snid`、
`localization`、`luancher`、`website` / `social` / `sponsor`。

启动器**只读取自己需要的字段**，其余原样忽略，因此枢纽包可以直接导入。

### 3.2 弥音扩展

扩展放在 **`miyin` 命名空间**里，避免与枢纽未来的字段撞名：

```json
{
  "name": "自由之翼：重生",
  "description": "重制版自由之翼战役，含 32 张关卡",
  "version": "1.4.2",
  "author": "SomeCreator",
  "type": "Campaign",
  "campaign": "WOL",

  "miyin": {
    "format": 1,
    "cover": "assets/cover.png",
    "tags": ["重制", "全语音"]
  }
}
```

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `miyin.format` | number | 否 | 扩展格式版本。**高于启动器支持版本时拒绝导入**，当前为 `1` |
| `miyin.cover` | string | 否 | 封面图相对路径，优先级高于顶层 `cover` |
| `miyin.tags` | string[] | 否 | 标签，显示在版本卡片上 |

为方便包作者，顶层也接受一个 `cover` 字段（等价于 `miyin.cover`，但优先级更低）。

## 4. `campaign` 取值与归并规则

`campaign` 决定这个包**自动导入到哪个战役**。取值容错匹配（不分大小写、含即算）：

| 取值示例 | 归入战役 | 地图落点（相对游戏 `Maps/Campaign`） |
| --- | --- | --- |
| `WOL` / `wings` / `liberty` | 自由之翼 | 根目录 |
| `HOTS` / `swarm` | 虫群之心 | `swarm` |
| `HOTSEVO` / `evolution` | **虫群之心** | `swarm/evolution` |
| `LOTV` / `void` / `legacy` | 虚空之遗 | `void` |
| `LOTVPROLOGUE` / `prologue` | **虚空之遗** | `voidprologue` |
| `NCO` / `nova` | 诺娃隐秘行动 | `nova` |

两条关键规则：

1. **主菜单只有四大战役**。进化与序章不是独立条目，而是归并到父战役下的版本，
   通过 `target_sub` 决定地图落到哪个子目录。
2. **目标子目录属于版本，不属于战役**。所以「虫群之心」这一个战役里可以同时放
   本体包和进化包，切换时各自落到正确位置。

`campaign` 认不出来时，启动器**不会猜**：导入流程会停下来让用户选择目标战役。

## 5. 封面图

按下面的顺序取，先命中先用：

1. `miyin.cover`（JSON）或 `cover`（txt，也认 image / icon / banner）
2. 内容根下名为 `cover` / `preview` / `banner` / `poster` / `封面` 的图片
   （png / jpg / jpeg / webp / gif / bmp，可在三层子目录内）
3. 内容根下的第一张图片
4. 都没有 → 使用该战役的**官方美术**

## 6. 启动器的安全约定

包可以携带任意内容，所以启动器对这些地方一律不信任：

| 风险 | 处理 |
| --- | --- |
| 条目名含 `..`、绝对路径、盘符 | 直接**拒绝整个包**（zip-slip） |
| 元数据里的名字是 `..`、`.`、保留名、空 | 安全化后使用；不可用则退回压缩包名 |
| 解压后体积 / 条目数超限 | 拒绝（上限 8 GiB / 100000 条） |
| 元数据编码 | 优先 UTF-8，失败退回 GBK |
| 落点路径 | 由启动器拼接，并做白名单与包含性校验 |

## 7. 版本演进

- 新增可选字段**不**提升 `miyin.format`（老启动器会忽略它们）。
- 改变既有字段语义、或新增启动器**必须理解**才能正确安装的字段时，提升 `miyin.format`。
- 包作者可以省略 `miyin` 整节；那时按纯枢纽 / CCM 包处理。
