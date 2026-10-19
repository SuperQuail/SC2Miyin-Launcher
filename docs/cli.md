# 命令行工具 miyin-cli

给**其他开发工具与 agent** 用的：检查一个战役包，输出机器可读的 JSON 与退出码，
不必读 Rust 源码、也不必复刻一遍解析逻辑。

看的就是界面上那套东西 —— 同一个 `miyin_core::campaign::package::inspect()`，
所以**字段名与界面同源，一个不改**。改了这里就等于改契约，请同步本文档。

> 只做四条命令（`inspect` / `identify` / `placements` / `compat`）。
> 参数解析是手写的 —— 为四条命令引 clap 不划算。

---

## 1. 快速上手

```bash
cargo build -p miyin-cli                      # 产物：target/debug/miyin-cli.exe
cargo build --release -p miyin-cli            # 发出去的：target/release/miyin-cli.exe（dist/ 里也有一份）
# 发行版里它是**单独附件**（*-cli-win64.zip）—— 只有开发者需要，不在普通用户的主包里

# 一个包：stdout 是一个对象
target/debug/miyin-cli inspect "D:/包/枪兵之翼.zip"

# 多个包：stdout 是数组
target/debug/miyin-cli inspect a.zip b.zip --pretty

# 属于哪部战役（认不出来会如实说）
target/debug/miyin-cli identify x.zip

# 每个载荷会落到游戏目录的哪里
target/debug/miyin-cli placements x.zip --slot lotv --pretty

# 按我们的约定 vs 按枢纽（CCM）：哪些文件游戏认，哪些不认
target/debug/miyin-cli compat x.zip --pretty
```

直接拿来判断（例：只看能不能装）：

```bash
target/debug/miyin-cli inspect a.zip > /dev/null 2>&1 && echo "可安装" || echo "有问题（退出码 $?）"
```

---

## 2. 契约（别的工具依赖这几条，别随手改）

| 项 | 约定 |
| --- | --- |
| **stdout** | 只有 JSON。一个包一个对象；传多个包时是**数组**。 |
| **stderr** | 人话（问题清单、失败原因）。**绝不混进 stdout**。 |
| **退出码** | `0` 全部可安装 / `1` 有 broken 级问题 / `2` 参数错或读不了包 |
| **编码** | UTF-8，中文**不转义**（直接是 UTF-8 字节，不是 `\uXXXX`） |
| **`--pretty`** | 缩进输出；默认紧凑（一行）。两者是**同一份** JSON。 |

一条容易误会的：**`identify` 认不出归属不算错误** —— 退出码仍是 `0`，
JSON 里如实写 `"slug": null`。这不是"失败"，是结论（见 §4）。

---

## 3. `inspect <包路径>...`

输出 `PackageInspection`，字段与界面完全一致。至少给一个路径。

```json
{
  "path": "D:\\包\\枪兵之翼.zip",
  "installable": true,
  "format": "ccm",
  "name": "枪兵之翼",
  "author": "未知作者",
  "version": "1.0",
  "description": null,
  "campaign_type": "wol",
  "cover": null,
  "tags": [],
  "kind": "campaign",
  "id": null,
  "requires": [],
  "priority": null,
  "payloads": [
    {
      "source": "MarineNightmareMod.SC2Mod",
      "target": { "kind": "mod", "name": "MarineNightmareMod.SC2Mod" },
      "expanded": false,
      "is_mod": true
    }
  ],
  "main_map": null,
  "overrides": [],
  "modid": null,
  "declared_mods": [],
  "doc": null,
  "identification": null,
  "suggested_slot": "wol",
  "content_root": "",
  "suggested_dir_name": "枪兵之翼",
  "map_count": 30,
  "mod_count": 1,
  "entry_count": 33,
  "unpacked_bytes": 52097258,
  "issues": []
}
```

几个**必须按语义读**的字段：

| 字段 | 怎么读 |
| --- | --- |
| `installable` | 能不能装 = 没有任何 broken 级问题（与 `issues` 里的 `level` 一致） |
| `issues[].level` | `ok` / `warning` / `broken`。**只有 broken 才影响可安装性**，warning 是提醒 |
| `issues[].code` | 稳定标识，见 §6。`message` / `hint` 是给人看的中文 |
| `format` | `ccm`（metadata.txt）/ `miyin`（metadata.json）/ `plain`（无元数据）/ `unknown` |
| `kind` | `campaign` 战役本体 / `patch` 覆盖层补丁 |
| `campaign_type` | 归属资料片。认得出的小写：`wol` `hots` `hotsevolution` `lotv` `lotvprologue` `nova` `custom`；**认不出**是 `{"other":"包内原文"}` |
| `suggested_slot` | 导入时该进哪个槽位；`null` = 认不出来，得让人选 |
| `payloads[].expanded` | 载荷是解开的**目录树**（true）还是单文件 MPQ（false）—— 落盘要保持原形态 |
| `payloads[].source` | 相对**内容根**的路径；`content_root` 是解压时要剥掉的那层 |
| `unpacked_bytes` | 解压后总字节数（防 zip bomb 用的同一个口径） |
| `identification` | 元数据没声明归属时的**证据链**结论，见 §4；元数据已声明时为 `null` |

---

## 4. `identify <包路径>`

只要一个路径（多给是参数错）。回答"这包属于哪部战役、凭什么"。

```json
{
  "path": "D:\\包\\疯批帝国军械库2.4.zip",
  "campaign_type": "wol",
  "slug": "wol",
  "evidence": "map_name_prefix",
  "evidence_label": "地图文件名",
  "is_exact": false,
  "detail": "tarcade.SC2Map",
  "installable": true
}
```

| 字段 | 说明 |
| --- | --- |
| `evidence` | `metadata` 包内声明 / `mirror_path` 包内镜像路径 / `map_dependency` 地图内的依赖声明 / `map_name_prefix` 地图文件名 —— 按可靠度从高到低 |
| `is_exact` | 这条证据是不是**精确**的。**`map_name_prefix` 是启发式，不是确定结论** |
| `detail` | 证据的具体内容（原文），排查时看它 |
| `slug` | 也就是槽位键；**`null` = 四层证据都没命中** |

三条如实性约定：

1. `slug` 为 `null` 时，`campaign_type` 仍然给出包内的声明 ——
   认不出来时会是 `{"other":"作者写的原文"}`，看得见"它自称是什么"。
2. 元数据声明归属时，核心不走证据链，这里补成 `"evidence": "metadata"`、`is_exact: true`
   —— 免得把"没有 evidence"误读成"没认出来"。
3. 补丁（`kind=patch`）没有归属可言 —— 它的目标由挂到哪个战役身上决定。
   这时同样 `slug: null`，stderr 会说明原因。

---

## 5. `placements <包路径> [--slot <slug>]`

把每个载荷展开成**游戏目录内的相对路径**，用的就是铺盘那套
（`compose::payload_target_path` + `Placement`）。

`--slot` 手动指定永远优先；不写就用包内声明的归属（`suggested_slot`）；
两个都没有就是**参数错（退出码 2）** —— 不猜。

```json
{
  "path": "D:\\包\\酒馆合作虫心.zip",
  "slot": "hots",
  "custom": false,
  "folder": null,
  "base": "Maps/Campaign",
  "summary": { "payloads": 36, "inside_game": 36, "outside_game": 0 },
  "payloads": [
    {
      "source": "0酒馆虚遗.SC2Mod",
      "kind": "mod",
      "expanded": false,
      "is_mod": true,
      "target": "Mods/0酒馆虚遗.SC2Mod",
      "raw_target": "Mods/0酒馆虚遗.SC2Mod"
    },
    {
      "source": "zchar01.SC2Map",
      "kind": "map",
      "expanded": false,
      "is_mod": false,
      "target": "Maps/Campaign/swarm/zchar01.SC2Map",
      "raw_target": "zchar01.SC2Map"
    }
  ]
}
```

| 字段 | 说明 |
| --- | --- |
| `slot` | 实际用的槽位 slug（`--slot` 或包内声明） |
| `custom` | 是不是自制战役（落点进 `Maps/CustomCampaigns/<folder>/`） |
| `folder` | 自制战役在 `Maps/CustomCampaigns/` 下的那层目录名；官方战役为 `null` |
| `base` | 这个槽位的落点基址：`Maps/Campaign` 或 `Maps/CustomCampaigns/<folder>` |
| `payloads[].target` | **游戏目录内**的相对路径（正斜杠）；`null` = 这个载荷不进游戏目录 |
| `payloads[].raw_target` | 包自己给的落点；`target` 为 `null` 时看它（实际不会编一条假路径出来） |
| `payloads[].kind` | `mirror` 包内已按游戏目录摆好 / `mod` 落到 `Mods/` / `map` 按槽位子目录展开 |

**`target` 只代表"会往哪写"，不等于已经写了。** 这份输出是**预演**，一个字节都没落盘。

### 落点由谁决定（读输出前先知道这几条）

- **镜像包**：包根上有 `Mods/` 或 `Maps/` 时整包按镜像落盘，
  连包内的大小写与层级一起保留（`package::mirrors_game_root`）
- **官方战役**：`map` 落 `Maps/Campaign/<子目录>/`，子目录由槽位定（hots → `swarm`、lotv → `void`……）
- **自制战役**：`Maps/CustomCampaigns/<folder>/` —— 那是 CCM 与玩家认的位置
- **模组**：两类战役都进 `Mods/`（地图里写死了依赖路径，只去那里找）

---

## 6. `compat <包路径>`

同一个包，**两套安装约定各算一遍落点**，并逐文件标出**游戏会不会认**。

为什么需要：纸面上两边都说「兼容 CCM」，但**落点约定并不一样** ——
我们按包内声明的 `campaign` 把官方改版放进 `Maps/Campaign/…`，
而枢纽（CCM）那一套**不看声明**，一律当自制战役塞进 `Maps/CustomCampaigns/<名字>/`。
同一个包两边算出来的路径不同，能不能被游戏认出来也就可能不同。

### 两套约定到底是什么

| | **ours**（弥音） | **nexus**（枢纽 / CCM） |
| --- | --- | --- |
| 官方改版（`campaign=wol`…） | `Maps/Campaign[/子目录]/` | `Maps/CustomCampaigns/<名字>/` |
| 自制战役（`campaign=custom`） | `Maps/CustomCampaigns/<名字>/` | 同左 |
| 模组（`.SC2Mod`） | `Mods/<名字>` | 同左 |
| 镜像载荷（包内写死了 `Maps/` 或 `Mods/`） | 按包内路径 | 同左 |

两套都走**同一份**落点规则（`library::compose` 的 `Placement` 与 `payload_target_path`），
只是喂进去的 placement 不同 —— 落点规则没有第二套实现，见 `docs/package-format.md` §1.4 与 AGENTS.md §14.5。

### `recognized` 是什么

**落点是不是在游戏会扫描的目录里**：`Maps/Campaign/**`、`Maps/CustomCampaigns/**`、`Mods/**`。
不在（被摆进 `Maps/` 下别的目录、`SC2Data/`、`Interfaces/` 之类）就是 `false`。
判据**只看目录**，不看文件本身能不能跑。

别和写盘闸门搞混：闸门放行的是「安装目录里任意位置」（AGENTS.md §10.5），那是**允许写**的边界；
这里是**游戏认不认**的边界。包能装进去 ≠ 游戏看得到。

```json
{
  "slot": "lotv",
  "files": [
    {
      "source": "paiur01.SC2Map",
      "kind": "map",
      "ours":  { "target": "Maps/Campaign/void/paiur01.SC2Map",
                 "recognized": true,
                 "why": "两套约定的落点不同，但都在游戏扫描的目录里" },
      "nexus": { "target": "Maps/CustomCampaigns/Reborn/paiur01.SC2Map",
                 "recognized": true,
                 "why": "两套约定的落点不同，但都在游戏扫描的目录里" }
    },
    {
      "source": "X.SC2Mod",
      "kind": "mod",
      "ours":  { "target": "Mods/X.SC2Mod", "recognized": true, "why": "" },
      "nexus": { "target": "Mods/X.SC2Mod", "recognized": true, "why": "" }
    }
  ],
  "summary": {
    "total": 2,
    "ours":  { "recognized": 2, "missed": 0 },
    "nexus": { "recognized": 2, "missed": 0 },
    "warnings": ["这个包声明成 lotv：枢纽那套不看声明，会把它当自制战役装进 Maps/CustomCampaigns/"]
  }
}
```

| 字段 | 说明 |
| --- | --- |
| `slot` | 我们认出来的归属槽位；**认不出来就是 `null`**（照样给两套落点，但不假装认得） |
| `files` | 逐个载荷一行，`source` 是包内相对内容根的路径 |
| `files[].kind` | `mirror` / `mod` / `map` —— 载荷落点类型 |
| `files[].ours`、`files[].nexus` | 两套约定下各自的 `target` / `recognized` / `why` |
| `why` | **只在 `recognized: false` 或两套结果不同时**才给；其余是空串 |
| `summary.total` | 载荷总数（等于 `files` 的长度） |
| `summary.ours` / `summary.nexus` | `recognized` 与 `missed` 两个计数 |
| `summary.warnings` | 值得提一句的地方：认不出归属、官方改版会被枢纽当自制战役、哪一套有文件认不出来 |

**退出码**：`0` 两套都全认；`1` 有文件不会被认（或包本身就 broken，这时 stdout 是空的）；`2` 参数错。

stderr 上有同一份汇总，方便人直接看：

```text
C:/包/枪兵之翼.zip: 这个包声明成 wol：枢纽那套不看声明，会把它当自制战役装进 Maps/CustomCampaigns/
按我们的约定：31/31 可识别；按枢纽：31/31
```

---
## 7. 退出码

| 码 | 含义 | 什么时候 |
| --- | --- | --- |
| `0` | 全部可安装 / 两套约定都全认 | 每个包的 `issues` 里都没有 `broken`；`compat` 时两套都没有 `recognized: false` |
| `1` | 有 broken 级问题 / 有文件不会被认 | 至少一个包 `installable: false`（含"存在但读不了"的包）；或 `compat` 里有文件落进游戏扫描不到的目录 |
| `2` | 参数错 / 读不了 | 命令或选项不认识、**路径不存在**、`--slot` 认不出、`placements` 定不了槽位 |

注意"包在、但读不了"是 `1` 而不是 `2` —— 那种情况核心会给出一条 `NOT_AN_ARCHIVE`
的 broken 结论，属于"这个包有问题"，不是"你参数给错了"。

`2` 时 **stdout 是空的**（不吐半截 JSON）：`inspect` 会把所有路径先验一遍，
错一个就整体不输出。

常见的 broken 码（完整清单以 `crates/miyin-core` 为准）：

| code | 什么情况 |
| --- | --- |
| `NOT_AN_ARCHIVE` | 文件在，但不是能读的压缩包（下载不完整多半是这个） |
| `UNSAFE_ENTRY` | 包里有越界路径条目（zip-slip），**拒绝安装** |
| `NO_CONTENT` | 包里没有任何 `.SC2Map` / `.SC2Mod` |
| `TOO_MANY_ENTRIES` / `TOO_LARGE` | 条目数或解压体积超上限 |
| `FORMAT_TOO_NEW` | 用了比当前启动器更新的扩展格式，明确拒绝而不是猜 |
| `UNUSABLE_NAME` | 推导不出可用的安装目录名 |

---

## 8. 一个可直接复制的完整例子

```bash
cd "D:/Code/Rust/HSCL"
cargo build -p miyin-cli
CLI=target/debug/miyin-cli.exe

# 1) 预检：能装吗？是哪个槽位？
"$CLI" inspect "D:/包/枪兵之翼.zip" --pretty

# 2) 归属：凭什么这么判？
"$CLI" identify "D:/包/枪兵之翼.zip" --pretty

# 3) 落点预演：会往游戏目录写什么？（记得看 exit code）
"$CLI" placements "D:/包/枪兵之翼.zip" --pretty
echo "exit=$?"

# 4) 兼容性：按枢纽那套装，这个包有几个文件游戏认不出来？
"$CLI" compat "D:/包/枪兵之翼.zip" --pretty
echo "exit=$?"
```

在别的语言里接（Python）：

```python
import json, subprocess

run = subprocess.run(
    ["target/debug/miyin-cli.exe", "inspect", "D:/包/枪兵之翼.zip"],
    capture_output=True,
)
report = json.loads(run.stdout)          # 好包 -> 一个对象
if run.returncode != 0:
    print("不能装：", [i["code"] for i in report["issues"] if i["level"] == "broken"])
print(report["name"], report["suggested_slot"], report["map_count"], "张地图")
```

---

## 9. 边界与已知取舍

- **只读**：四条命令都不写盘、不改包，也不会改动战役库。
- **`identify` / `placements` / `compat` 只收一个包**。要批量就 `inspect` 多个，
  或自己循环（`placements` 与 `compat` 的输出依赖包自己的归属，混着传容易看串）。
- **`placements` 的 `folder` 用包内建议的目录名**（`suggested_dir_name`）。
  包还没入库时，这正是它**将要**用的那层目录名；已入库的版本目录名可能与它不同 ——
  真正铺盘时的落点以库里那份为准（`compose`）。
- **zip 只读目录列表，不解压内容** —— 实测 862 MB 的真实包 `inspect` 约 0.12 秒。
  非 zip 的容器（7z / rar）要借系统解压器，那是另一回事：可能得整体解一遍。
- **非 zip 的容器**（7z / rar / tar）交给系统 `tar`（见
  `docs/package-format.md`）；系统上没有可用解压器时就是 `NOT_AN_ARCHIVE`。
- 这是 v1：只有四条命令。要加命令之前先想清楚别的 agent 是不是真的需要它。

## 10. 已知不一致：CLI 预测的是**今天**的规则

`inspect` / `placements` 反映的是**当前这版核心**怎么解析这个包。库里**已经导入过**的旧记录
是按当时那版规则算的落点 —— 两者可能不一样（真实例子：某个 10 月 9 日导入的包把 `evolution/`
留在了载荷名里，今天再 inspect 会把它当分类目录剥掉，于是 `placements` 给出不同的目标路径）。

拿 CLI 去核对**已入库版本的实际落点**时请记住这一点；要判断一个包**现在**会装成什么样，
CLI 是准的。这条不在核心的职责里 —— 核心只保证「重新导入一次会按新规则落盘」。
