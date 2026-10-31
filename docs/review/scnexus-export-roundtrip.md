# SCNexus 导出包 → 星际枢纽安装 仿真报告（2026-10-31）

## 方法

1. 从真实战役库 `D:\Game\BLZ\miyin_launcher_data` 选三个版本的**实际内容**，调用我们的
   `library::export::export(..., ExportFormat::Scnexus, merge_patches=true)` 生成 zip。
2. 按 SCNexus 源码模拟安装与启用：
   - `compress-process.ts:280-333`：`metadata.manager === "SCNexus"` 才安装；
     zip 根 `metadata.json`；整个 zip 解到 `<LIBRARY_ROOT>/<sanitized name>`。
   - `campaign-util.ts:37-78`：启用时复制 `Maps/Campaign`、`Maps/<maps_directory>`、
     `Mods/<mods_directory>`，并把 `metadata.json` 复制到 `Maps/Campaign/metadata.json`。
3. 再把这份 SCNexus 库喂给我们的 `hub::scnexus::scan → convert → library::activate`，
   比较启用后的 `Maps/Campaign/**` + `Mods/**` 相对路径与 sha256。

## 结果

| 版本 | 资料片 | 导出 metadata | 导出内容 | SCNexus 模拟启用文件 | 我们启用文件 | 差异 |
|---|---|---|---:|---:|---:|---|
| AI盟友虚空之遗 | LOTV | 25 maps / 1 mod | 1634 文件 | 1634 | 1634 | 仅 SCNexus 的 metadata.json 标记（已排除后 0） |
| Wings of Mengsk | WOL | 31 maps / 12 mods | 3307 文件 | 3306 | 3306 | 0 |
| 酒馆合作虫心 | HOTS | 34 maps / 2 mods | 314 文件 | 313 | 313 | 0 |

导出的 SCNexus 元数据（三个包均通过模拟安装校验）：

- `manager: "SCNexus"`、`manager_mode: "standard"`、`type: "Campaign"`
- `campaign` 为 `LOTV` / `WOL` / `HOTS` 大写
- `maps_directory: "Campaign"`，`maps[]` 带 `name` / `relative_path: "void|swarm"` / `components: true`
- `mods_directory: ""`，`mods[]` 带名称与 `components`；内容位于 zip 根的 `Maps/**`、`Mods/**`

**结论：我们的「星际枢纽兼容包」导出可被按 SCNexus 源码安装/启用，并且能被我们自己的
SCNexus 检测/转换链路完整读回。**

## 说明 / 边界

- 仿真按 SCNexus 源码实现（本机没有 Electron/SCNexus 运行环境），不是实际启动枢纽客户端。
- 差异里 SCNexus 会额外写 `Maps/Campaign/metadata.json`；这是它的书签文件，游戏不读。
- 自制战役（slot=custom）不属于 SCNexus 标准 Campaign 包，导出仍会列出 losses；未做安装测试。
