# SCNexus 托管 → 弥音托管 仿真报告（2026-10-31）

## 范围与方法

- 扫描 reference/ 下 33 个压缩包；筛出 SCNexus 能作为战役安装的 **17 个 CCM 包**（有 metadata.txt 且 campaign 属于 WOL/HOTS/LOTV/NCO）。
- 安装侧按 SCNexus 源码实现：
  - ccm-process.ts:21-83：解压到 <CCM_ROOT>/<sanitized name>（含 metadata_root 的包先取元数据父目录再改名）
  - campaign-util.ts:100-180：按资料片把顶层 .SC2Map 拷到 Maps/Campaign/<sub>，.SC2Mod 拷到 Mods/；HOTS 的 evolution/、LOTV 的 voidprologue/ 单独处理；metadata.txt 拷到战役目录
- 我们侧：hub::scnexus::scan → convert → library::activate 到一份全新的游戏目录，比较 Maps/Campaign + Mods 下每个文件的相对路径与 sha256。
- 所有 case 使用独立临时游戏目录，不碰真实 D:\Game\BLZ\StarCraft II。

## 结果

| # | 包 | campaign | SCNexus 启用树文件数 | 我们启用树文件数 | 差异 |
|---|---|---|---:|---:|---|
| 1 | AI盟友虚空之遗（灵梦版）1.9.9.zip | LOTV | 1599 | 1598 | 仅缺 metadata.txt |
| 2 | AI盟友虫群之心.zip | HOTS | 1851 | 1850 | 仅缺 metadata.txt |
| 3 | Legacy of the Xel'Naga.7z | LOTV | 1574 | 1573 | 仅缺 metadata.txt |
| 4 | 伊罕利纪元v1.12.zip | LOTV | 44 | 43 | 仅缺 metadata.txt |
| 5 | 净化者纪元 1.07 汉化V3.zip | LOTV | 1958 | 1957 | 仅缺 metadata.txt |
| 6 | 净化者纪元幼儿园（提示必须看）.zip | LOTV | 1959 | 1958 | 仅缺 metadata.txt |
| 7 | 利维坦AI虫心重置版v0.53.zip | HOTS | 1865 | 1864 | 仅缺 metadata.txt |
| 8 | 帝国之翼+.zip | WOL | 463 | 462 | 仅缺 metadata.txt |
| 9 | 帝国之翼皇家兵工厂.zip | WOL | 3307 | 3306 | 仅缺 metadata.txt |
| 10 | 平衡x2虫心（有单位升级）.rar | HOTS | 29 | 28 | 仅缺 metadata.txt |
| 11 | 枪兵之翼1.03幼儿园就这版 CCM.7z | WOL | 32 | 31 | 仅缺 metadata.txt |
| 12 | 泽拉图之遗.zip | LOTV | 1574 | 1573 | 仅缺 metadata.txt |
| 13 | 精英虫心轨道支援v2版by小白的心之所念.zip | HOTS | 1363 | 1362 | 仅缺 metadata.txt |
| 14 | 肉鸽之翼1.09.1疯狂天才.zip | WOL | 1792 | 1791 | 仅缺 metadata.txt |
| 15 | 酒馆合作虫心.zip | HOTS | 314 | 313 | 仅缺 metadata.txt |
| 16 | 阿塔尼斯的自由之翼肉鸽冒险v1.0.2.zip | WOL | 1611 | 1610 | 仅缺 metadata.txt |
| 17 | 黄金之遗1.32(CCM).zip | LOTV | 30 | 29 | 仅缺 metadata.txt |

**17/17 全部通过**：地图/模组文件路径与内容逐字节一致。唯一差异是每个 case 都少一份 Maps/Campaign/<sub>/metadata.txt —— 这是星际枢纽自己的元数据标记（启用时从 CCM 包拷进去的），游戏不读它；层级本身没有变化。

## 密码安装（Legacy of the Xel'Naga.7z，密码 1234）

- 正确密码：import_with_password + activate 成功，启用树 **2153** 个文件。
- 错误密码：明确报「压缩包密码不对」，不落库、不启用。
- 同一包用「直接密码导入」与「先 7z 解压再用目录导入」得到的启用树 **0 个差异**（路径/hash 完全一致）。

## 未纳入 / 说明

- 只有 .SC2Map、没有 metadata.txt 的包（如 _合成测试包）不是战役包，SCNexus 也会拒绝安装，未纳入。
- 只有补丁/模组、没有 campaign 的包（如 净化者纪元141版单纯数值增加补丁）不是可启用战役，未纳入。
- 测试1.zip 的根 metadata.json 缺少 SCNexus 标准要求的 type: Campaign（campaign-manage.ts:36），SCNexus 扫描时不会把它当战役，未纳入。
- 仿真脚本为一次性工具（target/tmp 下，未入库）；结果 JSON：target/tmp/sim-results.json。
