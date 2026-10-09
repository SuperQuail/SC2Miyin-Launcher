# 战役包打包指南

给**包作者**看的。格式的权威定义在 [package-format.md](package-format.md) 与
[package-format-v3.md](package-format-v3.md)，这份讲怎么把它用起来。

---

## 1. 最小可用包

一个文件夹（或一个 zip），里面放地图就行：

```text
我的战役.zip
└── Maps/
    └── Campaign/
        └── void/
            ├── 01.SC2Map
            └── 02.SC2Map
```

启动器会认出来这是**虚空之遗的改版**。认不出来的包导入时会让你选。

## 2. 加元数据，让它更聪明

在包根放 `metadata.json`：

```json
{
  "name": "复刻战役 · 重制",
  "author": "your-name",
  "version": "8.1",
  "campaign": "void",
  "description": "改了 Marine 数值，补了终章过场",
  "miyin": {
    "format": 2,
    "id": "scmr-terran",
    "tags": ["重制", "剧情"],
    "cover": "cover.png",
    "main_map": "Maps/CustomCampaigns/我的战役/入口.SC2Map"
  }
}
```

CCM 的老格式（`metadata.txt` 那种 `键=值`）**照样认** —— 想同时兼容两边就都放一份。
`miyin` 是弥音自己的命名空间，别的工具会直接忽略它。

### `miyin` 里能写什么

| 字段 | 作用 |
| --- | --- |
| `format` | 扩展格式版本。**当前是 2**；写高了启动器会明确拒绝，不会猜着解析 |
| `id` | 注册 ID。同一个战役的多个版本靠它认亲；不写就按目录名算 |
| `tags` | 卡片上的标签 |
| `cover` | 封面图（相对包根的路径）。不写就用官方美术 |
| `main_map` | 自制战役的**游玩入口**地图 |
| `mods` | 这张地图依赖哪些模组（`modid` 或模组名） |
| `doc` | 说明文档（PDF），界面上能直接看 |
| `kind` | `campaign`（默认）或 `patch` |
| `overrides` | **覆盖规则**，见下一节 |

## 3. 覆盖规则：把文件放到游戏目录里任意位置

默认情况下启动器按"战役该在哪"来摆（`Maps/Campaign/<子目录>/`）。
但有些东西**不属于任何战役** —— 界面 Mod、全局数值表、想在别处加一份的东西。
这时用 `overrides`：

```json
{
  "miyin": {
    "format": 2,
    "overrides": [
      { "from": "extra/Interfaces", "to": "Interfaces" },
      { "from": "tweaks/UnitData.SC2Mod", "to": "Mods" },
      { "from": "说明.txt" }
    ]
  }
}
```

三种写法：

| 写法 | 效果 |
| --- | --- |
| `{ "from": "说明.txt" }` | **原样覆盖**游戏目录里的同一个相对路径（这里是 `说明.txt`） |
| `{ "from": "tweaks/UnitData.SC2Mod", "to": "Mods" }` | 落到 `Mods/UnitData.SC2Mod` |
| `{ "from": "extra/Interfaces", "to": "Interfaces" }` | `extra/Interfaces` 底下的东西整体落到 `Interfaces/` 那一层 |

**`to` 一律当目录**（文件会保住自己的文件名）。三条边界：

1. **第一条命中的规则生效** —— 更具体的写前面。
2. 落点**必须在游戏目录之内**，不能有 `..` 或盘符，出目录会被拒绝。
3. 覆盖别的文件**会被记账**：启动器先把原文件备份进 `data/backup/`，
   切回原版时原样还原。你不用自己写"备份说明"。

> 铺盘前启动器会**先把这次会动哪些文件摆给你看**（新增/覆盖/接管/删除各几项），
> 你点确认它才动。

## 4. 地图与模组的两种形态都行

```text
单文件   01.SC2Map                     一个 MPQ 文件
展开树   01.SC2Map/Base.SC2Data/...    编辑器里"存成组件"之后的样子
```

两种都能导入，导入后会**保持原形态**落盘。

## 5. 打包时的几个坑

- **别把 `Maps/` 或 `Mods/` 上面再套一层目录**（比如 `我的战役/Maps/…`）。
  套了就按"任意分类目录"处理，只取文件名，游戏可能扫不到。
- **元数据放包根**。放在 `Maps/Campaign/void/` 那种深处，启动器会去找，
  但别的工具多半认不出。
- **自制战役的地图要放在 `Maps/CustomCampaigns/<你的名字>/`** ——
  放进 `Maps/Campaign/` 会污染官方目录，而且游戏不会把它当自制战役列出来。
- **不要打包游戏本体文件**。包里出现 `SC2Data/` 这类内容时想清楚：
  那是"覆盖游戏资源"，属于高级用法，用户会看到明确的警告。
- 大包没问题（几百 MB 常见），但**每个文件都会被记账**，别塞几万个碎文件。

## 6. 导入之后

启动器把包解到自己的库里（`data/campaigns/<战役>/<版本>/`），
**启用**的时候才往游戏目录铺。所以：

- 同一个战役可以**同时留好几个版本**，随时切；
- 切版本不需要重新导入；
- 停用会把铺进去的文件撤掉、把被覆盖的官方文件还原。

## 7. 想验证一下？

```bash
# 装完依赖后
pnpm -C ui dev          # 浏览器演示模式
cargo run -p miyin-launcher
```

导入时如果哪里不对，启动器会给出**具体到文件**的提示（zip 越界、体积超限、
落点出目录、格式版本太新……），照提示改就行。
