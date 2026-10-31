# 启动器用户可见文案审查表（口语化文案 + 改写建议）

> **2026-10-29 更新：本表已按维护者要求落地。** 用户侧文案大部分改写完成（Top20 除 SlotMenu/原生
> 对话框等跨文件项外已收口；术语规则「玩家基于官方战役的改版 = 改版」已落地），详见
> `CHANGELOG.md`「未发布 / 文案」与 `docs/review/screenshots/`。**§13 需要拍板的 12 项仍未动。**
> 下文保留原始审查记录，供逐条核对。
>
> 原状态：只读审查，未改动任何代码或界面。任务：task-2，审查范围与排除项见文末「附录 A」。

## 0. 阅读说明

- **行号基准**：2026-10-29 16:2x 工作区快照。审查期间 `src-tauri/src/lib.rs`、`crates/miyin-core/src/**`、`ui/src/react/views/mods/{ModsView.tsx,mods.css}`、`ui/src/react/views/campaigns/CustomCampaignPanel.tsx` 正被并行修改（含一轮整体格式化，部分 Rust 文件因此整体位移 1 行、`lib.rs` 后半整体位移 8 行）。**本文行号已按 16:2x 的快照对齐过一次，仍可能继续漂移；审查时请以「原文」搜索定位**，不要只信行号。
- **收录标准**：用户能看到的字（通知 / 按钮 / 占位符 / 空状态 / 错误兜底 / 对话框正文 / title / 原生对话框）。纯日志、代码注释、测试、开发者页（`ui/src/ide/**`）、CLI 输出不收。
- **改写原则**：简洁、准确、不卖萌、不训人；错误提示 = **发生了什么 + 用户能做什么**。术语按文末 §11 统一。
- 含「推断」字样的是我无法从代码单方面确认、需要实测或产品决定的判断。
- **术语依据**：术语建议已对齐 2026-10-29 新增的 AGENTS §14.12（模组 `mounted`＝配置、`in_game`＝实际铺入 Mods/，界面须区分「已勾选」与「已生效」）。

## 1. 战役列表页（CampaignsView / SlotCard / VariantCard）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/campaigns/CampaignsView.tsx:75` | 还没有找到星际争霸 II | 未检测到星际争霸 II | 「还没有找到」把失败说成进行时；空状态标题宜用结论句。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:77-78` | 启动器会先读取注册表里的安装位置。如果没有找到，请手动指定游戏根目录（即包含 StarCraft II.exe 的那一层）。 | 启动器会先尝试从注册表读取安装位置。如果未找到，请手动选择游戏根目录（含 StarCraft II.exe 的文件夹层级）。 | 「读取注册表」是内部实现；「那一层」口语，但括号解释可保留更具体的「文件所在目录」。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:82` | 选择游戏目录 | 选择游戏目录 | 保持。与设置页「重新选择」动作名一致。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:95` | 版本未知 | 未知版本 | 保持。但建议与设置页的「未初始化」等状态词统一为「未知」前缀。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:115` | 官方战役 | 官方战役 | 全站同时存在「官方战役 / 原版战役 / 原版」，指同一概念（AGENTS §14.2）。界面只留一个。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:120` | 已导入版本 | 已安装版本 | 此数字统计的是已导入库的变体；「导入」在别处指动作，名词化后与「可选版本」不统一。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:124` | 已切换 | 已启用 | 「切换」是动作，「已启用」才是状态；且卡片、右键菜单都说「启用」。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:143` | 读取中… / 重新读取 | 读取中… / 重新读取 | 保持。但与右键菜单「重新读取战役库」不一致，建议统一为「重新读取」。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:146` | 小工具 ▾ | 工具 ▾ | 「小工具」是口语化降格说法；菜单里只有作弊码查询与重读，叫「工具」即可。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:153` | 更新战役包… | 更新已安装战役包… | 「更新战役包」读起来像导入一个新的包；实际是替换已安装版本（见 SlotMenu 的「更新这一版」）。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:161` | ＋ 导入战役包 | ＋ 导入战役包 | 保持。建议统一省略号规则：打开对话框的按钮/菜单项带「…」，直接执行的不带。 |
| `ui/src/react/views/campaigns/CampaignsView.tsx:167-168` | 导入时启动器会读取包里的战役信息，自动归入对应的战役；每部战役默认是原版，点开可以导入并切换不同玩家制作的版本，多个版本同时保留、互不覆盖。 | 导入时启动器会读取包内信息并归入对应战役。点击进入菜单切换不同改版。 | 两句话里塞了四件事，第二句主语从「启动器」滑到「玩家」；「每部」也不如「每部战役」清楚。 |
| `ui/src/react/components/SlotCard.tsx:15` | 已启用：{active_name} / 原版战役 | 当前版本：{active_name} / 原版战役 | 「已启用：名字」在卡片上同时承担状态和名称，读到冒号后不知道是版本名还是战役名。 |
| `ui/src/react/components/SlotCard.tsx:22` | 已切换 | 已启用 | 同 CampaignsView:124，与全站「启用」术语不一致。 |
| `ui/src/react/components/VariantCard.tsx:60` | 暴雪官方战役，不做任何修改 | 暴雪官方战役 | 卡片说明用短句；「不做任何修改」像承诺。 |
| `ui/src/react/components/VariantCard.tsx:107` | 使用包内封面 | 已使用包内封面图 | 「使用」是动作描述，这里表达的是状态。 |
| `ui/src/react/components/VariantCard.tsx:114` | 从库中删除这个版本 | 从战役库删除此改版 | 「库」在界面上没有定义；与 SlotMenu 的「删除这个版本」、CustomView 的「从库中删除」三处三种说法。 |

## 2. 战役菜单页与启用流程（SlotMenu / ActivationPreviewDialog / ActivationConflictDialog / CustomCampaignPanel）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/campaigns/SlotMenu.tsx:139` | 选中这个版本 | 选择此改版 | 「选中」偏控件操作口语；右键菜单项宜用动词短语。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:140-141` | 启用这个版本 / 启用并开始游戏 | 启用此改版 / 启用并开始游戏 | 保持语义，仅统一「这个 / 此」。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:149-150` | 切回原版战役 / 开始游戏 | 切换为原版战役 / 开始游戏 | 「切回」隐含「曾经是」，新用户第一次就点会困惑。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:209` | 已自动挂载 {N} 个补丁 | 已自动启用 {N} 个补丁 | 「挂载」是全站最混乱的术语之一（见 §11）。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:210` | 没有能自动匹配的补丁（没有声明依赖的只能手动挂） | 没有可自动匹配的补丁（未声明依赖的需手动启用） | 括号里「手动挂」口语且省略宾语。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:224` | 补丁「{name}」已进库 | 补丁「{name}」已导入到补丁库 | 「进库」是内部黑话，用户第一次看到不知道进了哪个库。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:328` | 先选一个版本再导出 | 请先选择要导出的版本 | 命令式短句配「先…再…」像口头提醒；错误/前置条件提示应直接给动作。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:342` | （有 {N} 项是目录树形态，CCM 可能读不了） | （有 {N} 项以文件夹形式存放，CCM 可能无法读取） | 「目录树形态」是包格式内部术语；「读不了」口语。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:370` | 挑一部战役，直接启动游戏来玩 | 选择要游玩的战役，直接启动游戏 | 「挑」口语化；「来玩」口播腔。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:371` | 选择要游玩的版本 —— 原版战役，或导入的玩家版本 | 选择要游玩的改版：原版战役，或玩家制作的改版 | 「导入的玩家版本」把动作和来源混在名词里。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:387` | 导入新版本请回到战役列表页 | 新改版请在战役列表页导入 | 「请回到…」像训人；改成陈述约束。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:451` | 点上面任意一部战役，这里会出现它的地图列表与模组挂载。 | 选择上方任意一部战役，这里会显示它的地图列表与已启用的模组。 | 方位词「上面」在窄窗会换位置；「模组挂载」是内部概念。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:456` | 还没有导入任何玩家版本。回到战役列表页点「导入战役包」，启动器会自动判断它属于哪部战役。 | 尚未导入改版。在战役列表页点「＋ 导入战役包」，启动器会尝试判断所属战役。 | 「还没有」口语；「自动判断」与 ImportDialog 里「自动识别是尽力而为」的措辞冲突。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:477` | 挂载补丁（{N}） | 启用补丁（{N}） | 与 §11 术语统一：补丁是覆盖层，用启用/停用。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:483` | 补丁是覆盖层：它不复制战役本体，启用时按优先级叠加。关掉即撤销，随时可恢复。 | 补丁以覆盖层方式叠加在战役之上，不复制战役本体；停用即撤销，随时可恢复。 | 「关掉即撤销」混用两个动作词；「本体」偏开发内部说法。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:518` | 移出 | 停用 | 「移出」既可能指移出补丁列表也可能指移出游戏目录；右上角按钮应写动作结果。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:525` | 这个战役还没有挂补丁。声明了依赖的补丁可以「自动匹配」；没声明的只能手动挂。 | 该改版尚未启用补丁。声明了依赖的补丁可自动匹配，未声明的需手动启用。 | 同一句里「挂」「自动匹配」「手动挂」三种动作词。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:541` | 挂上 | 启用 | 按钮文案与列表状态「已启用」不成对。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:545` | 补丁库是空的，先导入补丁包。 | 补丁库为空。请先导入补丁包。 | 「空的，先…」口语流水句；补一句「导入补丁包」入口更好，但属功能改动，需另议。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:572` | 用压缩包把这一版换掉（删旧的、装新的；包里没有元数据也能用） | 用压缩包替换此版本（删除旧内容、安装新内容；无元数据的包也可用） | title 提示里「换掉」「删旧的、装新的」是最口语的一处。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:607` | 已是当前版本 | 已启用 | 「当前版本」在别处指「选中的版本」，这里其实指「已启用」，概念撞车。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:641` | 同样只改启动器记录的元数据，不动包内容，也不影响它已经挂在哪些战役上。 | 只修改启动器记录的元数据，不改动包内文件，也不影响该补丁已启用的战役。 | 「挂在哪些战役上」口语；「元数据」对普通用户仍偏技术，可考虑「包信息」。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:701-702` | 只改启动器记录的元数据，不动包里的原始文件 —— 随时可以改回来。改名字不会改版本目录，挂在它上面的补丁不受影响。 | 只修改启动器记录的元数据，不改动包内原始文件，随时可撤销。改名字不会改动版本目录；启用在该版本上的补丁不受影响。 | 「挂在它上面的补丁」需要读者自己还原关系。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:743-744` | 勾上的表示地图需要它才能正常打开。导出这个包时会一并带上，模组管理页也会给它打「依赖」标。 | 勾选的模组是地图正常运行所必需的。导出此包时会一并包含；模组页会将其标记为「依赖」。 | 「打…标」口语；「勾上的」省略中心语。 |
| `ui/src/react/views/campaigns/SlotMenu.tsx:802-803` | 会把这一版从库里删掉，连带它已经装进游戏目录的地图和模组一起撤回。删了就得重新导入。 | 将从战役库删除此改版，并一并从游戏目录移除它写入的地图与模组。后续游玩需重新导入。 | 「删了就得」训人 + 口语；「装进游戏目录」「撤回」术语混。 |
| `ui/src/react/views/campaigns/ActivationPreviewDialog.tsx:28` | 启用「{variant}」会改动这些 | 启用「{variant}」将写入以下内容 | 「这些」没有中心语；对话框标题应直说对象。 |
| `ui/src/react/views/campaigns/ActivationPreviewDialog.tsx:34` | 继续，写进游戏目录 | 继续写入游戏目录 | 逗号断句的口语命令；按钮文案不宜带语气词。 |
| `ui/src/react/views/campaigns/ActivationPreviewDialog.tsx:60` | 删掉上次装的 | 移除上次写入的文件 | 「上次装的」口语且指代不清（上次装的什么）。 |
| `ui/src/react/views/campaigns/ActivationPreviewDialog.tsx:66` | 要往游戏目录写 {n} MB。 | 将向游戏目录写入 {n} MB。 | 「要往…写」口播句。 |
| `ui/src/react/views/campaigns/ActivationPreviewDialog.tsx:67` | 被覆盖的文件会先挪进 data/backup，切回原版时原样还原。 | 被覆盖的文件会先备份到数据目录，切回原版时自动还原。 | 向用户暴露内部目录名 `data/backup`；「挪进」「原样还原」口语。 |
| `ui/src/react/views/campaigns/ActivationPreviewDialog.tsx:71` | 看覆盖了哪些（前 20 个） | 查看将被覆盖的文件（前 20 个） | 「看覆盖了哪些」是口语问句。 |
| `ui/src/react/views/campaigns/ActivationConflictDialog.tsx:41` | {a} 和 {b} 冲突 | {a} 与 {b} 需要写入重名的文件 | 「冲突」没说清冲突对象；标题应包含后果。 |
| `ui/src/react/views/campaigns/ActivationConflictDialog.tsx:55` | 它们都要往游戏目录写同一批文件（{n} 个）。同一时间只能生效一个，选哪个？ | 两者都需要向游戏目录写入同一批文件（{n} 个）。同一时间只能启用一个，请选择要启用的战役。 | 「写同一批文件」可保留但补主语；「选哪个？」是口播式反问。 |
| `ui/src/react/views/campaigns/ActivationConflictDialog.tsx:60` | 会撞的位置 · {n} 个 | 重叠的文件 · {n} 个 | 「撞」是口语比喻，术语表里没有对应概念。 |
| `ui/src/react/views/campaigns/ActivationConflictDialog.tsx:67` | 被 {slot} · {variant} 占着 | 当前由 {slot} · {variant} 使用 | 「占着」口语且略带指责。 |
| `ui/src/react/views/campaigns/ActivationConflictDialog.tsx:72` | 等 {rest} 个 | 另有 {rest} 个 | 「等 N 个」是省略式口语，单独成行时读不通。 |
| `ui/src/react/views/campaigns/ActivationConflictDialog.tsx:76` | 选 {b}：什么都不改。选 {a}：它的文件会盖掉对方的，对方会有一部分失效。 | 选择 {b}：不修改任何文件。选择 {a}：它的文件会覆盖对方，对方将有一部分内容失效。 | 「盖掉」口语；「有一部分失效」需要说明是哪部分（文件层面）。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:128` | 已停用，装进去的文件都撤回了 | 已停用，已从游戏目录移除本改版写入的文件 | 「装进去的文件」「撤回了」术语混（见 §11）。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:147` | 已挂载 {N} 个模组 / 已全部取消挂载 | 已启用 {N} 个模组 / 已停用全部模组 | 「取消挂载」在界面其他处叫「停用」「关闭」，一个动作三种叫法。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:161` | 已把「{map}」设为主地图 | 已将「{map}」设为主地图 | 「把」字句口语；通知宜用书面短句。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:180` | 还没定主地图 —— 在地图列表里挑一张，点「设为主地图」 | 尚未设置主地图。请在地图列表中选择一张，并点击「设为主地图」。 | 「还没定」「挑一张」口语；破折号连接两个动作不利于扫读。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:252` | 已装进游戏目录，点右边那个绿色的可以停用卸下。 | 已启用。再次点击按钮可停用并从游戏目录移除。 | **「右边那个绿色的」是最该改的一条**：方位 + 颜色指代，改版/窄窗/主题色一变就失效。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:253` | 还没装进游戏目录 —— 点右边那个把它装进去，就能直接启动游戏。 | 尚未启用。点「已停用」按钮启用后即可直接启动游戏。 | 同上；且按钮显示的是状态（已启用/已停用），文案却按动作描述，冲突。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:271` | 点击停用：把装进游戏目录的文件撤回 / 点击启用：装进游戏目录 | 点击停用：从游戏目录移除本改版写入的文件 / 点击启用：将本改版写入游戏目录 | title 里「装进」「撤回」继续制造术语分歧。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:254` | 共 {N} 张地图。 | 共 {N} 张地图 | 句末句号在状态短句里多余（同页其他行无句号）。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:274` | 已启用 / 已停用 | 已启用 / 已停用 | 保持。这是全站最准确的一对状态词，应作为模组页的参照。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:296` | 还没定主地图。挑一张点「设为主地图」，之后「启动」按钮就打开它。 | 尚未设置主地图。选择一张地图并点「设为主地图」，之后「启动」按钮将自动进入该地图。 | 「挑一张点…」「就打开它」口语；「它」指代需回读。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:304` | 挂载模组 | 启用模组 | 模组区块标题使用「挂载」，与模组页「启用/停用」冲突（见 §11）。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:306` | 已挂 {mounted} / {total} | 已启用 {mounted} / {total} | 「已挂」缩略口语。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:310-311` | 地图需要这些模组才能正常打开。不同战役的模组可能互相冲突，按需勾选即可；一个都不挂的话，地图打开会报错。 | 地图需要这些模组才能正常打开。不同战役的模组可能冲突，请按需勾选；缺失模组可能导致地图打开会报错。 | 「按需勾选即可」「一个都不挂」「报错」三处口语。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:349` | 正在读取库里地图… | 正在读取地图… | 「库里地图」缺结构助词，读起来像错字。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:351` | 这个版本里没有 .SC2Map 地图。 | 此版本中没有地图文件（.SC2Map）。 | 文件扩展名夹在中文句子里；「里」口语。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:386` | 设为主地图 | 设为主地图 | 保持。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:394` | 启动这张 | 启动此地图 | 「这张」省略中心语；旁边按钮是「设为主地图」，可对齐。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:419` | 一个模组都没挂 | 未启用任何模组 | 标题用口语否定句，作为错误中间态标题过重。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:421` | 这个战役带了 {N} 个模组，而你一个都没勾。 | 此战役包含 {N} 个模组，当前尚未启用任何MOD。 | 「而你」有责备感，违反「不训人」。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:424-425` | 地图需要它们才能正常打开，现在直接启动，多半会报错或者出现紫色方块。 | 地图需要这些模组才能正常打开；直接启动可能导致报错或资源缺失。 | 「多半会」「紫色方块」是玩家黑话，产品文案不宜用。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:429` | 我去挂上 | 去启用 | 「我去…」卖萌口语；按钮文案应说明动作。 |
| `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:432` | 还是打开 | 仍然启动 | 「还是」是口语转折；与旁边「启动」动词不一致（打开 vs 启动）。 |

## 3. 自制战役页（CustomView）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/custom/CustomView.tsx:76` | 打开（看地图和模组） | 打开（查看地图与模组） | 括号里「看」口语；「打开」与内页「启动」不是同一动作，易混。 |
| `ui/src/react/views/custom/CustomView.tsx:177` | 独立整部战役 | 独立的完整战役 | 两个短词拼接，缺结构助词。 |
| `ui/src/react/views/custom/CustomView.tsx:180-181` | 不装进游戏目录 —— 地图留在启动器里，直接启动游戏就能玩。 | 【需拍板】说明改为与实际行为一致：自制战役在启用后写入游戏目录，启动时由启动器加载。 | **与同一页下方面板直接矛盾**：面板说「还没装进游戏目录…装进去就能直接启动」。见 §13 待拍板项 1。 |
| `ui/src/react/views/custom/CustomView.tsx:236` | 还没有自制战役 | 暂无自制战役 | 「还没有」口语；空状态标题宜用中性结论。 |
| `ui/src/react/views/custom/CustomView.tsx:238` | 点「＋ 导入战役包」选一个包，启动器会自动认出它属于自制战役。 | 点「＋ 导入战役包」选择压缩包；在此页导入的包都会归为自制战役。 | 「选一个包」「自动认出」口语，且与 ImportDialog 里「这一页导入的都算自制战役」重复解释。 |
| `ui/src/react/views/custom/CustomView.tsx:83` | 从库中删除 | 从战役库删除 | 与 SlotMenu / VariantCard 三处措辞不同（见 §11）。 |
| `ui/src/react/views/custom/CustomView.tsx:99` | 已复制：{path} | 已复制路径：{path} | 别处「复制版本目录路径」通知同样省略了「路径」，建议统一。 |
| `ui/src/react/views/custom/CustomView.tsx:111` | 已导出：{N} 张地图 · {N} 个模组 | 已导出：{N} 张地图、{N} 个模组 | 顿号用「·」是排版风格，中文句内并列建议顿号；不强制。 |
| `ui/src/react/views/custom/CustomView.tsx:294-295` | 会把这一版从库里删掉，连带它已经装进游戏目录的地图和模组一起撤回。删了就得重新导入。 | 将从战役库删除此改版，并从游戏目录移除它写入的地图与模组。删除后需重新导入。 | 与 SlotMenu:802 同文重复，且同样口语。 |

## 4. 导入、安装整合包与合并（ImportDialog / InstallView / MergeDialog）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/campaigns/ImportDialog.tsx:95` | 未命名战役 | 未命名战役 | 保持。但 ImportDialog:143 的「未标版本」建议改为「未标注版本」，两处「未」字短语风格不一致。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:99-101` | 包内声明的资料片 / 自动识别 / 无法识别 | 包内声明的资料片 / 自动识别 / 未能识别 | 「无法识别」像在说用户无法识别；「未能识别」指向启动器。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:105` | 依据：{detail} | 依据：{detail} | 保持。建议在 detail 为空时隐藏整串，避免只剩「依据：」。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:138-141` | 更新的版本 / 相同的版本 / 更旧的版本 / 无法比较版本 | 较新版本 / 相同版本 / 较旧版本 / 无法比较 | 「更旧的版本」拗口；四项里只有第一项带「的」，格式不齐。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:143-144` | 未标版本 | 未标注版本 | 「标」是缩写口语。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:146-147` | 库里已有「{existing}」{ver}，本次导入 {incoming} —— {relation} | 战役库已有「{existing}」{ver}，本次导入 {incoming}（{relation}）。 | 「库里」内部叫法；破折号连接关系词，长句难读。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:203` | 补丁「{name}」已进库，到对应战役里挂载即可 | 补丁「{name}」已导入补丁库，到对应战役中启用即可 | 「进库」「挂载」两处术语。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:227` | 已更新「{name}」 | 已更新「{name}」 | 保持。建议补上版本号（后端已返回），与其他成功提示带对象信息的风格一致。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:285` | 已取消这几个新模组的挂载 | 已停用这些新模组 | 「取消挂载」全站仅此一处；且「这几个」口语。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:316` | 这一页导入的都算自制战役，不看包属于哪部原版战役 | 此页导入的包都会归为自制战役 | 「都算」「不看」口语。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:329-330` | 这是一个补丁包，它不归属任何战役。导入后到对应战役的菜单里挂载即可，可以同时挂多个并调整优先级。 | 这是补丁包，不归属任何战役。导入后到对应战役菜单中启用；可同时启用多个并调整优先级。 | 「挂载」「挂多个」「挂」同段三种说法。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:337-338` | 更新：{name} / 导入到：{name} | 更新：{name} / 导入到：{name} | 保持。但「导入到：自制战役」在自制战役入口下是固定值，可以整行省略。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:356` | 自动识别没能判断出归属，请手动选择 | 自动识别未能判断归属，请手动选择 | 「没能」口语；「归属」两次出现略绕。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:364-365` | 这个包明确属于「{a}」，你把它改到了「{b}」。装错地方通常会让战役里找不到关卡，确认前请想一下。 | 此包明确属于「{a}」，当前选择为「{b}」。选择错误可能导致战役内找不到关卡。 | 「你把它改到了」有质问感；「请想一下」训人。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:378-379` | 一起挂载这 {N} 个模组（地图需要它们才能打开，建议勾上；之后可以到战役页面里改） | 同时启用这 {N} 个模组（地图需要它们才能打开，建议勾选；之后可在战役页修改） | 「挂载」「勾上」口语；括号里两句话用分号断开即可。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:422-423` | 更新会先删掉这一版、再装进新的；它正启用着的话，会先切回原版、装完再应用回去。存档档案与补丁绑定挂在战役上，不受影响。 | 更新将先删除此版本、再安装新版本；若该版本正在启用，会先切回原版，安装完成后再启用新版本。存档与补丁绑定挂在战役上，不受影响。 | 「装进新的」「正启用着」「应用回去」口语；「存档档案」重复。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:433` | 导入补丁库 | 导入到补丁库 | 按钮缺介词，读起来像「导入一个补丁库」。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:450` | 新包里多了 {N} 个模组 | 新包包含 {N} 个新模组 | 「多了」口语；对话框标题宜陈述事实。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:452-453` | 已经按默认一起启用了。不想要的取消勾选即可 —— 取消只影响这几个，其余保持原样。 | 已按默认设置一并启用。取消勾选即可停用对应模组，只影响所选模组，其余保持不变。 | 「不想要的」「即可」混口语；破折号后重复解释。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:478` | 确定改到「{slot}」吗？ | 确认改为「{slot}」？ | 「改到」口语；确认框标题用「确认」。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:487-491` | 这个包属于 {slot}。装到别的战役里，地图会落在游戏找不到的位置 —— 多半进游戏后找不到关卡，玩不了。 | 此包属于 {slot}。导入到其他战役会使地图落在游戏无法读取的位置，进入游戏后可能找不到地图。 | 「装到」「落在」「多半」「玩不了」四处口语。 |
| `ui/src/react/views/campaigns/ImportDialog.tsx:500` | 我知道，仍然导入 | 仍然导入 | 「我知道」+「仍然」是对峙式口吻，按钮只需表达动作与后果。 |
| `ui/src/react/views/install/InstallView.tsx:43-47` | 确定（包内自己声明） / 较可靠（镜像路径 / 地图依赖） / 猜的（文件名前缀） | 确定（包内声明） / 较可靠（目录结构或地图依赖） / 推测（地图文件名前缀） | 「猜的」自嘲且不准确——这层是启发式，应写「推测」；「镜像路径」是内部术语。 |
| `ui/src/react/views/install/InstallView.tsx:165` | 先选一个整合包 | 请先选择要安装的整合包 | 口语祈使句缺宾语。 |
| `ui/src/react/views/install/InstallView.tsx:183` | 预检没通过 | 预检未通过 | 「没」口语；且这是错误兜底文案，应说明能做什么（重选文件）。 |
| `ui/src/react/views/install/InstallView.tsx:189` | 还没选装到哪部战役 | 尚未选择安装到哪部战役 | 「选装到」是病句。 |
| `ui/src/react/views/install/InstallView.tsx:216` | 一个都没装上：{failures} | 全部安装失败：{failures} | 「一个都没装上」是最口语的通知之一，且丢掉了「失败原因」的主语。 |
| `ui/src/react/views/install/InstallView.tsx:218` | 已安装 {N} 个，失败 {M} 个：{failures} | 成功 {N} 个，失败 {M} 个：{failures} | 同一句里「安装」与「失败」不对称；建议统一为「成功/失败」。 |
| `ui/src/react/views/install/InstallView.tsx:241` | 把整合包拖到这里 | 将整合包拖到此处 | 「把…到这里」口语；空状态拖放区可保留亲切度，但建议与全局「请…」一致。 |
| `ui/src/react/views/install/InstallView.tsx:243` | 可以一次拖好几个，一起装；也可以点这里选文件（zip / 7z / rar 都行）—— 这一步只检查，不写任何文件 | 可一次拖入多个包批量安装，也可点击选择文件（支持 zip / 7z / rar） | 「好几个」「都行」「一起装」口语；用省略号破折号拼接三句。 |
| `ui/src/react/views/install/InstallView.tsx:313` | 全部安装（{N}） | 全部安装（{N}） | 保持。 |
| `ui/src/react/views/install/InstallView.tsx:336` | 库里已经有同名的版本，整批按哪种方式处理？ | 战役库已有同名版本，请选择处理方式 | 「库里」内部叫法；「同名」不够准确（实为同一注册 ID/同名）。 |
| `ui/src/react/views/install/InstallView.tsx:340-341` | 覆盖更新（沿用原目录名，旧版被替换掉） / 重命名后导入（两个版本并存，各自启用） | 覆盖更新（沿用原目录名，旧版被替换） / 重命名后导入（两个版本并存，可分别启用） | 「替换掉」「各自启用」口语且缺主语。 |
| `ui/src/react/views/install/InstallView.tsx:424` | ✗ 读不了这个包：{error} | ✗ 无法读取此包：{error} | 「读不了」口语；错误提示应说明下一步（换一个包/解压后重试）。 |
| `ui/src/react/views/install/InstallView.tsx:430` | 游戏实例名称（装到哪部战役名下；决定包里的地图落到哪个战役目录） | 安装目标（选择安装到哪部战役；决定包内地图写入哪个战役目录） | 「游戏实例名称」是技术词，实际字段就是目标战役；hint 里「装到…名下」「落到」口语。 |
| `ui/src/react/views/install/InstallView.tsx:451` | 认不出来，上面自己选一个 | 无法识别，请在上方手动选择 | 「认不出来」「自己选一个」口语，且方位「上面」。 |
| `ui/src/react/views/install/InstallView.tsx:458-459` | 可信度：{tier} —— 依据：{detail}。这不是确定的，认错了把上面那行改掉就行。 | 可信度：{tier}。依据：{detail}。该结果并非确定，如判断有误可在上方修改归属。 | 「认错了」「改掉就行」口语 + 训人混合。 |
| `ui/src/react/views/install/InstallView.tsx:467` | （模组是地图的依赖，安装时会一起挂上） | （模组是地图的依赖，安装时将一并启用） | 「挂上」术语（见 §11）。 |
| `ui/src/react/views/install/InstallView.tsx:484` | ✓ 已安装 | ✓ 已安装 | 保持。注意与「已导入」混用：同一动作在导入对话框叫「导入」，在这里叫「安装」。见 §13 拍板项。 |
| `ui/src/react/views/install/InstallView.tsx:490` | 换一个包 | 重新选择文件 | 「换一个包」口语；且按钮实际是重新选文件。 |
| `ui/src/react/views/install/InstallView.tsx:540` | 归属：{name} / 认不出来 —— 右边自己选一部 | 归属：{name} / 无法识别，请在右侧选择战役 | 与 :451 同文重复（两套版式各写一遍），建议抽共用文案。 |
| `ui/src/react/views/install/InstallView.tsx:545-546` | 可信度：… 依据：…。这不是确定的，认错了把右边的归属改掉就行。 | 可信度：…。依据：…。该结果并非确定，如判断有误可在右侧修改归属。 | 同 :458，两处重复维护。 |
| `ui/src/react/views/install/InstallView.tsx:566` | 移除 | 从列表移除 | 「移除」在模组/补丁语境里承担了「停用」的含义，这里只是移出待安装列表，需限定。 |
| `ui/src/react/views/merge/MergeDialog.tsx:7-9` | 保持游戏目录那份 / 用本地这份 / 两份都留 | 保留游戏目录中的数据 / 使用本地数据 / 两份都保留 | 「那份」「这份」口语指代；选项说明建议直接写对象。 |
| `ui/src/react/views/merge/MergeDialog.tsx:34` | 发现另一份数据 | 发现另一份战役库数据 | 「另一份数据」缺少中心语，用户不知道是什么数据。 |
| `ui/src/react/views/merge/MergeDialog.tsx:47-49` | 程序旁边那份数据（…）和游戏目录这份（…）都想当战役库。不冲突的会并进来，下面这几个版本两边都有 —— 你说了算。 | 程序旁的旧数据（…）与游戏目录旁的战役库（…）都希望作为战役库。无冲突的条目会合并，以下版本两边都存在，请选择保留哪一份。 | 「都想当」「并进来」「你说了算」口语；且「程序旁边」与 AGENTS §14.1 的实际数据位置需核对。 |
| `ui/src/react/views/merge/MergeDialog.tsx:53` | 会一起并进来 · {N} 个版本（{bytes}） | 将合并 · {N} 个版本（{bytes}） | 「并进来」口语；标题宜用结果词。 |
| `ui/src/react/views/merge/MergeDialog.tsx:69` | 两边都有 · {N} 个，要你选 | 两端均存在 · {N} 个，需要选择 | 「要你选」口播式。 |
| `ui/src/react/views/merge/MergeDialog.tsx:90` | 这几样和当前安装绑死，一律用游戏目录那份：{list} | 以下内容与当前安装绑定，将统一使用游戏目录中的数据：{list} | 「绑死」「那份」口语。 |

## 5. 模组页（ModsView / ModSheets / useMods / data.ts）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/mods/ModsView.tsx:31` | 启用 / 停用 | 启用 / 停用 | 保持。这是右键菜单里的写法，也是 §11 推荐用词；应反向统一其他位置。 |
| `ui/src/react/views/mods/ModsView.tsx:33` | 复制挂载名 | 复制模组名 | 「挂载名」是内部概念（写入 Mods 目录后的名字），普通用户不知道是什么。 |
| `ui/src/react/views/mods/ModsView.tsx:79` | 这里能看到模组的来源，启停它们，也能单独导入、导出、改信息。 | 这里可以查看模组来源、启用或停用模组，也可以单独导入、导出和编辑信息。 | 「启停」是缩写口语；「改信息」与菜单「编辑信息」不一致。 |
| `ui/src/react/views/mods/ModsView.tsx:113-118` | 模组由启动器管理 / 模组由启动器在启动时自动挂载，非开发者请勿手动调整模组状态；手动改动可能导致游戏内资源缺失或战役无法启动。/ 确有需要时，仍可在下面的列表里单独启用/停用；那里的开关只表示勾选状态，不代表模组已经生效。 | 模组由启动器管理。启动时会自动同步已勾选的模组；请勿手动改动游戏目录中的模组文件，否则可能导致资源缺失或战役无法启动。确有需要时，可在下方列表勾选或取消勾选；开关仅表示勾选状态，已生效状态以游戏目录同步结果为准。 | **本条为并行新增的警示，纳入审查**：「自动挂载」「启用/停用」与 AGENTS §14.12 的「勾选 / 已生效」两状态模型仍不贴合；「非开发者」把用户分等级；正文一句 60+ 字。新增末句方向正确，但「开关 + 勾选状态」仍偏控件视角。 |
| `ui/src/react/views/mods/ModsView.tsx:140` | 现在还没有任何模组。导入带 .SC2Mod 的战役包，或者直接用上面的按钮单独导一个。 | 暂无模组。可导入包含 .SC2Mod 的战役包，或使用上方按钮单独导入模组。 | 「还没有」「导一个」口语；「上面」方位。 |
| `ui/src/react/views/mods/ModsView.tsx:161` | 全部启用 | 全部启用 | 保持。 |
| `ui/src/react/views/mods/ModsView.tsx:165` | 全部关闭 | 全部停用 | 同一页里「停用」（右键菜单）与「关闭」（本按钮、开关文字）混用，见 §11。 |
| `ui/src/react/views/mods/ModsView.tsx:195` | 铺进游戏目录后的样子 | 写入游戏目录后的路径 | 「铺进」是内部黑话（AGENTS 里也用它描述写盘）；title 应说明这是落点预览。 |
| `ui/src/react/views/mods/ModsView.tsx:211` | 基准… | 对比基准… | 「基准」单字省略，下拉里不知道基准是什么。 |
| `ui/src/react/views/mods/ModsView.tsx:216` | 与 v{version} 比 | 与 v{version} 对比 | 「比」口语省略。 |
| `ui/src/react/views/mods/ModsView.tsx:243` | 点击关闭 | 点击停用 | 与右键菜单「停用」、卡片状态「已启用/已关闭」不一致。 |
| `ui/src/react/views/mods/ModsView.tsx:247` | 已启用 / 已关闭 | 已启用 / 已停用 | 「已关闭」在用户看来是「关闭了游戏/窗口」；模组语境应说停用。 |
| `ui/src/react/views/mods/useMods.ts:102` | 已关闭 {name} / 已启用 {name} | 已停用 {name} / 已启用 {name} | 通知动词与按钮状态词不一致。 |
| `ui/src/react/views/mods/useMods.ts:128` | 已全部启用 / 已全部关闭 | 已启用全部模组 / 已停用全部模组 | 同上；「已全部关闭」歧义。 |
| `ui/src/react/views/mods/useMods.ts:243` | 到列表里把它启用就会铺进游戏目录 | 在列表中启用后，文件会写入游戏目录 | 「铺进」内部黑话；「它」指代不明。 |
| `ui/src/react/views/mods/ModSheets.tsx:25` | 「{name}」要怎么放？ | 「{name}」的导入方式 | 标题用口语问句；且「放」不是全站任何动作词。 |
| `ui/src/react/views/mods/ModSheets.tsx:38-40` | 这一包里带了 {N} 个模组，下面说的是其中第一个（{folder}）。其余会按各自的名字分别导入。 | 此包包含 {N} 个模组；以下说明针对其中第一个（{folder}），其余模组将按各自名称分别导入。 | 「这一包里带了」口语；「说的是」口播。 |
| `ui/src/react/views/mods/ModSheets.tsx:56-59` | 作为新版本：归到同一个模组下面，之后可以自由切换用哪一版。 / 作为独立改版：单独显示一个 —— 适用于「拿别人的模组改的」，它跟原版是两码事。 | 作为新版本：归入同一模组，之后可切换使用任意版本。 / 作为独立模组：单独显示，适用于基于他人模组修改的版本。 | 「下面」「单独显示一个」「拿别人的」「两码事」四处口语。 |
| `ui/src/react/views/mods/ModSheets.tsx:90-91` | 文件会从库里移除。如果它已经铺进游戏 Mods/ 目录，那边也会一起撤掉；你自己手动放的模组不受影响。 | 文件将从战役库移除。若该模组已写入游戏 Mods/ 目录，也会一并移除；你手动放入的模组不受影响。 | 「库里」「铺进」「那边」「撤掉」四处口语。 |
| `ui/src/react/views/mods/ModSheets.tsx:203` | 语义 diff：{path} | 内容对比：{path} | 「语义 diff」是开发者术语，出现在用户侧模组对比面板里。 |
| `ui/src/react/views/mods/ModSheets.tsx:219` | 游戏目录里的模组 | 游戏目录中的模组 | 「里」口语；同页其他标题用「中」。 |
| `ui/src/react/views/mods/ModSheets.tsx:223-224` | 这是 Mods/ 里实际存在的文件 —— 包括你自己放的、别的工具留下的。上面启用模组后会立刻铺进来。 | 此处显示 Mods/ 目录中实际存在的文件，包括手动放入和其他工具留下的文件。启用模组后会立即写入。 | 「这是」「你自己放的」「别的工具」「铺进来」口语；方位「上面」。 |
| `ui/src/react/views/mods/data.ts:66-68` | 单独导入 / 自制战役包 / 原版战役包 | 独立模组 / 自制战役包 / 原版战役包 | 来源标签「单独导入」描述的是动作而非来源；且 ModSheets 里同类概念叫「独立改版」。 |
| `ui/src/react/views/mods/data.ts:67` | 自制战役包 | 自制战役包 | 保持。注意与「自制战役」页面名称一致，无问题。 |

## 6. 存档页（SavesView / GroupDetailDialog / useSaves）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/saves/SavesView.tsx:93-94` | 这里列的是游戏目录里真正的存档文件（Documents/StarCraft II/Accounts/<账号>/<地域>/Saves 下的 .SC2Save），和游戏里「加载游戏」看到的是同一批。 | 此处列出游戏实际使用的存档文件（位于 Documents/StarCraft II/Accounts/<账号>/<地域>/Saves 的 .SC2Save），与游戏内「加载游戏」列表一致。 | 「真正的」「同一批」口语；路径里尖括号会被 HTML 转义，需确认实际渲染。 |
| `ui/src/react/views/saves/SavesView.tsx:159` | 没扫到存档。先在游戏里存一次档，或者确认游戏用的是哪个 Windows 账号的「文档」目录。 | 未找到存档。请先在游戏中保存一次，或确认当前 Windows 账户的「文档」目录位置。 | 「没扫到」「存一次档」口语；「哪个 Windows 账号」指向不清。 |
| `ui/src/react/views/saves/SavesView.tsx:190-191` | 按战役分组是从存档文件名认出来的（`虚空之遗 …` / `LotV …` 之类）。认不出的进「未归类」，之后可以手动归类；备份与切换还在做。 | 按战役分组依据存档文件名识别（如「虚空之遗 …」「LotV …」）。无法识别的归入「未归类」，之后可手动归类；备份与切换功能仍在开发中。 | **Markdown 反引号会原样显示给用户**；「认出来」「还在做」口语且暴露开发进度。 |
| `ui/src/react/views/saves/GroupDetailDialog.tsx:35` | 「{group}」里有什么 | 「{group}」的内容 | 口语问句标题。 |
| `ui/src/react/views/saves/GroupDetailDialog.tsx:49` | 这个组是一份 Banks 快照。如果某一份其实属于别的战役，把它归过去 —— 只动这一份，组名和别的存档都不受影响。 | 此组是一份 Banks 快照。若某份存档实际属于其他战役，可将其归入对应战役；操作只影响该存档，组名与其他存档不受影响。 | 「一份」「归过去」口语；「快照」是技术词但此处可保留。 |
| `ui/src/react/views/saves/GroupDetailDialog.tsx:57` | 这个组里还没有存档（空组）。 | 此组暂无存档。 | 「还没有」口语；括号重复解释。 |
| `ui/src/react/views/saves/GroupDetailDialog.tsx:67` | 归到… | 归入战役… | 下拉占位符缺中心语。 |
| `ui/src/react/views/saves/GroupDetailDialog.tsx:82` | 只动存档库（data/saves）里的这一份，不会动游戏目录的 Banks。目标里已有同名存档时不会覆盖，会提示你。 | 仅修改存档库中的该文件，不会修改游戏目录的 Banks。目标位置已有同名存档时不会覆盖，并会给出提示。 | 「只动」「一份」口语；暴露 data/saves 内部路径。 |
| `ui/src/react/state/useSaves.ts:106` | 已收进库 —— 原目录没动，战役列表里能看到了 | 已加入战役库，原目录未改动；可在战役列表中查看 | 「收进库」「没动」口语；破折号拼接。 |
| `ui/src/react/state/useSaves.ts:140` | 存档隔离已关掉：不再动 Banks，已有的组都留着 | 存档隔离已关闭：不再修改 Banks，已有分组保留 | 「关掉」「动」「留着」口语。 |
| `ui/src/react/state/useSaves.ts:142-143` | 存档隔离已打开 —— 现在这份进度存成了「原版」/ 存档隔离已打开 —— 还没有存档，玩过之后第一份进度会收成「原版」 | 存档隔离已开启。当前进度已保存为「原版」分组。/ 存档隔离已开启。暂无存档，首次游戏后的进度会保存为「原版」分组。 | 「打开」「收成」「玩过之后」口语；「原版」与战役「原版」撞名（见 §13 拍板项）。 |
| `ui/src/react/state/useSaves.ts:159` | 现在这份存档已存好 | 当前存档已保存 | 「存好」口语。 |
| `ui/src/react/state/useSaves.ts:187` | 已指派给这个战役 / 已改为「不算任何战役」 | 已归属到该战役 / 已改为「不归属任何战役」 | 「不算任何战役」语义含糊；「指派」偏管理腔。 |
| `ui/src/react/state/useSaves.ts:232` | 「{file}」已挪到「{to}」 | 「{file}」已移动到「{to}」 | 「挪」口语。 |
| `ui/src/react/state/useSaves.ts:248` | 「{file}」已归到「{label}」 | 「{file}」已归入「{label}」 | 「归到」口语。 |
| `ui/src/react/state/useSaves.ts:278` | 已还原，还原前那份存为 {name} | 已还原。还原前的存档已保存为 {name} | 「还原前那份」口语，且缺句读。 |

## 7. 设置页（SettingsView / ToolsPanel / UpdatePanel）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/views/settings/SettingsView.tsx:115` | 尚未设置游戏目录。启动器会先尝试从注册表自动发现。 | 尚未设置游戏目录。启动器会先尝试从注册表自动检测。 | 「自动发现」是内部实现词；对用户是「检测」。 |
| `ui/src/react/views/settings/SettingsView.tsx:126` | 导入的每个玩家版本都完整保存在这里（软件同级的 data 目录），与游戏目录解耦。 | 导入的每个版本都完整保存在战役库中（位于游戏安装目录同级），与游戏目录分离。 | **事实错误**：AGENTS §14.1 明确数据根在游戏安装目录同级而不是「软件同级」；「解耦」是开发词。 |
| `ui/src/react/views/settings/SettingsView.tsx:137-139` | 导入前先看一遍。打开压缩包时只做检查 —— 确认里面是不是战役包、有没有危险的文件路径、解开后有多大。这一步不写任何文件。 | 导入前可先预检。打开压缩包时只做检查：确认是否为战役包、是否包含危险路径、解压后的大小。此步骤不写入任何文件。 | 「先看一遍」「是不是」「有没有」口语；「里面」指代压缩包内容。 |
| `ui/src/react/views/settings/SettingsView.tsx:142-143` | 出错不会留下半成品。内容先解到临时目录，确认完整了才正式入库；中途失败会自动清理干净。 | 失败不会留下未完成的数据：内容先解压到临时目录，校验完整后才写入战役库；中途失败会自动清理。 | 「半成品」「入库」「清理干净」口语/内部词。 |
| `ui/src/react/views/settings/SettingsView.tsx:146-148` | 切换只动自己放的文件。启用某个版本时，放进去的每个文件都会记账；切回原版时只删这些文件。遇到同名的官方文件会先备份，切回时原样还原 —— 不会去删官方目录。 | 切换只操作启动器写入的文件：启用版本时逐个记录写入的文件，切回原版时只删除这些文件；遇到同名原版文件会先备份，切回时还原。不会删除原版目录中的文件。 | 「只动」「放进去」「记账」「不会去删」口语；「官方」与「原版」混用。 |
| `ui/src/react/views/settings/SettingsView.tsx:151-152` | 只往游戏目录里写。所有写入都限制在游戏的 Maps / Mods 等目录之内，不会碰别的地方。 | 只写入游戏目录。所有写入都限制在游戏安装目录内，不会修改其他位置。 | **与 AGENTS §10.5 现状不符**：写盘闸门已放开到安装目录任意位置，不再是 Maps/Mods；继续这样写会误导用户。 |
| `ui/src/react/views/settings/SettingsView.tsx:155-156` | 中文不会乱码。包里的说明文字会自动识别编码，简繁中文的战役包都能正常显示。 | 中文编码自动识别。包内说明文字会自动识别编码，简繁中文均可正常显示。 | 小标题「中文不会乱码」是口语承诺；应与其余安全条目的「主谓短语」格式一致。 |
| `ui/src/react/views/settings/SettingsView.tsx:164-169` | 游戏目录里已经装着的 / 这些是直接扔进 Maps/CustomCampaigns 的战役，还没进库 —— 收编之后就能用启动器管版本了。/ 收编不会删原目录。 | 游戏目录中已存在的战役 / 这些是直接放入 Maps/CustomCampaigns 的战役，尚未加入战役库。加入战役库后即可用启动器管理版本。/ 加入战役库不会删除原目录。 | 「装着的」「扔进」「还没进库」「收编」四处口语/黑话；「收编」是军事词。 |
| `ui/src/react/views/settings/SettingsView.tsx:178` | 收进库 | 加入战役库 | 与上一条同源；按钮应使用同一术语。 |
| `ui/src/react/views/settings/SettingsView.tsx:191` | 还没有存档 —— 玩过一关之后这里才会有东西。 | 暂无存档。完成一关后此处将显示存档。 | 「东西」口语且不准确（应是存档文件）。 |
| `ui/src/react/views/settings/SettingsView.tsx:211-214` | 默认关闭：打开之后每个战役用自己的一份存档，换战役不会把上一部的进度冲掉。打开的那一刻会把现在这份存档收成「原版」，不会丢进度。关掉之后启动器不再动 Banks，已经存下的组都留着。组的管理（改归属 / 切换 / 存回）在顶栏的「存档」页。 | 默认关闭。开启后每个战役使用独立的存档，切换战役不会覆盖上一部进度；开启时会将当前存档保存为「原版」分组，不会丢失进度。关闭后启动器不再修改 Banks，已有分组保留。分组管理（修改归属 / 切换 / 保存）在顶栏「存档」页。 | 「打开」「冲掉」「收成」「动」「留着」口语；「原版」命名与战役撞名（见 §13）。 |
| `ui/src/react/views/settings/SettingsView.tsx:220` | 把现在这份存起来 | 保存当前存档 | 「把…存起来」口语。 |
| `ui/src/react/views/settings/SettingsView.tsx:223` | 另外备份一份 | 另存为备份 | 口语；与「存档已备份」通知用词不一致。 |
| `ui/src/react/views/settings/SettingsView.tsx:230-231` | 已存档 {N} 份。切过去、或者把某一份指给别的战役。切换时会先把现在这份存回去。 | 已保存 {N} 份存档。可切换使用，或将某份存档归属到其他战役；切换时会先保存当前存档。 | 「切过去」「指给」「存回去」口语；第二句无主语。 |
| `ui/src/react/views/settings/SettingsView.tsx:245` | 不算任何战役 | 不归属任何战役 | 与 useSaves:187 同源；「不算」语义不清。 |
| `ui/src/react/views/settings/SettingsView.tsx:287-288` | 打开后这里会多出两个入口。它只存在本机（localStorage），换台机器要重新打开 —— 也不会把作者模式带给普通用户。 | 开启后此处会显示两个入口。该设置只保存在本机，更换设备后需重新开启；不会影响普通用户。 | 「入口」是开发词；直接展示 localStorage；「带给普通用户」含义不明。 |
| `ui/src/react/views/settings/SettingsView.tsx:295-296` | 开发者房间是所有控件的展示台：新控件必须先在房间里出现，确认外观与手感之后，才能拿到生产界面用（见 AGENTS §18.1）。 | 开发者房间用于展示所有控件。新控件需先在房间中确认外观与交互，之后才能用于正式界面。 | 用户侧文案引用内部规范 `AGENTS §18.1`；「生产界面」「手感」是开发词。 |
| `ui/src/react/views/settings/SettingsView.tsx:299-301` | 开发者页是给包作者的工作台：扫游戏目录、勾选要打进包的内容、按行读文本文件。地图和模组这类二进制会明确告诉你打不开、可以用什么打开。版本管理（提交 / 回滚 / 日志）还没接，界面里那一块标着「示例」。 | 开发者页是面向包作者的工作台：扫描游戏目录、勾选要打包的内容、按行读取文本文件。地图和模组等二进制文件会提示无法直接打开及可用的打开方式。版本管理尚未接入，对应区域标记为「示例」。 | 「打进包」「打不开」「还没接」「标着」口语；「告诉你」拟人。 |
| `ui/src/react/views/settings/SettingsView.tsx:341-342` | 弥音启动器（MiYin Launcher）—— 用 Rust 编写的星际争霸 II 战役与 Mod 管理器。 / 美术风格参考 HMCL；战役格式兼容 CCM 与弥音标准包。 | 弥音启动器（MiYin Launcher）：星际争霸 II 战役与 Mod 管理器，使用 Rust 编写。/ 界面风格参考 HMCL；包格式兼容 CCM 与弥音标准格式。 | 「用 Rust 编写」是技术自豪点，用户未必关心，但不是错误；「美术风格」应为「界面风格」。 |
| `ui/src/react/views/settings/SettingsView.tsx:345` | 当前运行模式：{桌面版（Tauri） / 浏览器演示模式（数据为示例）} | 当前运行模式：{桌面版 / 浏览器演示模式（数据为示例）} | 向用户暴露框架名 Tauri 无信息价值。 |
| `ui/src/react/views/settings/ToolsPanel.tsx:45` | {name} {version} 已装好 | {name} {version} 已安装 | 「装好」口语；与工具面板「已安装」标签不一致。 |
| `ui/src/react/views/settings/ToolsPanel.tsx:97` | 不装也能正常用 | 不安装也可正常使用启动器 | 口语短句，且「用」的对象不明。 |
| `ui/src/react/views/settings/ToolsPanel.tsx:118` | 处理中… / 更新到最新 / 一键安装 | 处理中… / 更新 / 安装 | 「一键安装」是营销腔；实际就是安装，按钮不必自夸。 |
| `ui/src/react/views/settings/ToolsPanel.tsx:134-135` | 上次自动安装失败{：error}。之后启动不会再自动尝试 —— 想装的话点上面的「一键安装」。 | 上次自动安装失败：{error}。后续启动不会再自动尝试；如需安装，请点上方「安装」。 | 「想装的话」「上面的」口语 + 方位指代。 |
| `ui/src/react/views/settings/ToolsPanel.tsx:159` | 装这个版本 / 此版本没有附件 | 安装此版本 / 此版本无附件 | 同一行内「装」与「此版本」语体不一致。 |
| `ui/src/react/views/settings/UpdatePanel.tsx:107-108` | 启动时会自动检查一次更新，有新版本会在右上角显示下载标记。国内直连较慢时会自动并发试多个镜像；代理被限流时会自动改直连重试。 | 启动时自动检查一次更新；有新版本时右上角会显示下载标记。直连较慢时会并发尝试多个镜像；代理被限流时会自动改用直连重试。 | 「试多个镜像」「改直连」是技术流程，用户可读性低，但属可接受的透明说明；建议缩短。 |
| `ui/src/react/views/settings/UpdatePanel.tsx:172` | 这里能看到刚才到底做了什么 | 显示本次更新的完整过程 | 「到底」口语；「这里能看到」口语。 |
| `ui/src/react/views/settings/UpdatePanel.tsx:176` | 还没有日志。启动时的自动检查跑完就会有内容。 | 暂无日志。启动时自动检查完成后将显示日志。 | 「跑完」「就会有内容」口语。 |
| `ui/src/react/views/settings/UpdatePanel.tsx:200` | 自动（环境变量 → 系统代理 → 直连） | 自动（依次尝试：环境变量、系统代理、直连） | 箭头链对普通用户是代码味；括号内可读性差。 |
| `ui/src/react/views/settings/UpdatePanel.tsx:225` | 下载时自动尝试 GitHub 镜像加速（国内推荐） | 下载时自动尝试 GitHub 镜像加速（中国大陆网络推荐） | 「国内」地理表述模糊，但非错误；不强制。 |

## 8. 顶栏、外壳、启动兜底与全局提示

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `ui/src/react/app/TopBar.tsx:107-110` | 跟随系统（点一下切到日间） / 日间（点一下切到夜间） / 夜间（点一下切回跟随系统） | 跟随系统（切换为日间） / 日间（切换为夜间） / 夜间（切换为跟随系统） | 「点一下」口语；title 里说明动作即可。 |
| `ui/src/react/app/TopBar.tsx:142` | 开发者页（实验）：扫游戏目录、看文件、记版本 | 开发者页（实验）：扫描游戏目录、查看文件、记录版本 | 三个单字动词并列，「记版本」口语。 |
| `ui/src/react/app/TopBar.tsx:159` | 有新版本可用，点开设置页查看 | 有新版本可用，打开设置页查看 | 「点开」口语。 |
| `ui/src/react/app/TopBar.tsx:171` | 演示模式 | 演示模式 | 保持。 |
| `ui/src/react/app/AppShell.tsx:166` | 松手即可导入 | 松开即可导入 | 「松手」口语（拖放语境可接受，但全站未统一）。 |
| `ui/src/react/app/AppShell.tsx:168-169` | 支持 zip / 7z / rar / tar —— 启动器会自动判断它属于哪部战役，认不出来会让你选，不会瞎猜。 | 支持 zip / 7z / rar / tar。启动器会尝试自动判断所属战役；无法识别时会请你选择。 | **「不会瞎猜」是自嘲式口语**，且暗示别处可能乱猜；应正向陈述。 |
| `ui/src/react/app/Dialogs.tsx:31` | 需要重启启动器才能完成安装 —— Windows 上没法覆盖正在运行的程序。 | 需要重启启动器才能完成安装：Windows 无法替换正在运行的程序文件。 | 「没法」口语；原因表述可保留但更适合「无法替换」。 |
| `ui/src/react/app/Dialogs.tsx:33-34` | 重启后版本会变成 {version}，你导入的战役、补丁和设置都不会受影响。 | 重启后版本将变为 {version}；导入的战役、补丁和设置不受影响。 | 「你导入的」第二人称，全站其他通知不用「你」。 |
| `ui/src/react/app/Dialogs.tsx:55` | 用浏览器打开{site.name}？ | 在浏览器中打开「{site.name}」？ | 口语省略介词；网站名应加引号。 |
| `ui/src/react/app/Dialogs.tsx:59` | （正在确认能不能走 HTTPS…） | （正在检测 HTTPS 可用性…） | 「能不能走」口语。 |
| `ui/src/react/app/BootGate.tsx:40-41` | 启动失败 / 后端的启动任务没有成功。可以点下面重新加载。 | 启动失败 / 后台初始化未完成。可点击下方「重新加载」重试。 | 「后端」是开发词；「点下面」口语 + 方位。 |
| `ui/src/react/app/BootGate.tsx:54-55` | 正在准备数据… / 第一次启动要迁移旧数据，库大的话会慢一会儿 | 正在准备数据… / 首次启动需要迁移旧数据；战役库较大时耗时较长。 | 「库大的话」「慢一会儿」口语。 |
| `ui/src/react/app/ErrorBoundary.tsx:39-41` | 界面出错了 / 可以点下面重新加载；还不行就把这段错误发给开发者。 | 界面发生错误 / 可点击下方「重新加载」重试；若问题仍然存在，请将错误信息反馈给开发者。 | 「出错了」「还不行」口语；「这段错误」措辞不准（应为错误信息）。 |
| `ui/src/react/app/startup.ts:62` | 启动失败 | 启动失败 | 保持。 |
| `ui/src/react/app/startup.ts:66` | 界面没能加载出来。可以点下面重新加载；还不行就把这段错误发给开发者。 | 界面未能加载。可点击下方「重新加载」重试；若问题仍然存在，请将错误信息反馈给开发者。 | 与 ErrorBoundary:41 同文但措辞不同（「没能」vs「出错了」），应共用一份。 |
| `ui/src/react/app/startup.ts:102` | 启动超时：{N} 秒内界面没有挂载出来。 | 启动超时：{N} 秒内界面未能完成加载。 | 「挂载」是前端术语；用户看到只会困惑。 |
| `ui/index.html:20` | 正在启动弥音启动器… | 正在启动弥音启动器… | 保持。 |
| `ui/index.html:22-25` | 一直停在这里说明前端没能加载。可以先重新加载；还是不行的话，可能是系统缺少 WebView2 运行时（重装过系统 / 精简版系统常见），装上 Microsoft Edge WebView2 Runtime 再打开即可。 | 若长时间停留在此页面，说明界面未能加载。可先重新加载；若仍无法打开，可能缺少 Microsoft Edge WebView2 运行时（重装系统或精简版系统常见），安装后重新打开即可。 | 「前端」「还是不行」口语/开发词；这段是白屏时唯一可读信息，应格外准确。 |
| `ui/src/react/app/useShellEffects.ts:112-115` | 剪切 / 复制 / 粘贴 / 全选 | 剪切 / 复制 / 粘贴 / 全选 | 保持。与系统惯例一致。 |
| `ui/src/react/app/useShellEffects.ts:125-127` | 刷新数据 / 设置 / 项目主页 | 刷新数据 / 设置 / 项目主页 | 保持。 |
| `ui/src/react/state/useLibrary.ts:113` | 已切回原版战役 | 已切换到原版战役 | 「切回」隐含曾经启用过；首次启用原版时不通顺。 |
| `ui/src/react/state/useLibrary.ts:115` | 已启用：{variant} | 已启用「{variant}」 | 冒号格式与全站「已导入「…」」不一致；建议统一用引号包裹对象。 |
| `ui/src/react/state/useLibrary.ts:219` | 已删除：{variant} | 已删除「{variant}」 | 同上。 |
| `ui/src/react/state/useLibrary.ts:232` | 正在启动星际争霸 II… | 正在启动星际争霸 II… | 保持。 |
| `ui/src/react/state/useUpdate.ts:162` | 下载完成，点「安装并重启」即可换上 {version} | 下载完成。点击「安装并重启」以更新到 {version} | 「换上」口语；日志里也应避免。 |
| `ui/src/react/state/useUpdate.ts:190` | 已设为不再提示 {version}，只显示角标 | 已设置不再提示 {version}，仅显示角标 | 「只」口语；「角标」对用户不直观，可写「右上角更新标记」。 |
| `ui/src/react/state/useTools.ts:22` | {name} 已自动装好 | {name} 已自动安装 | 「装好」口语。 |
| `ui/src/react/ui/Toast.tsx:95` | 关掉这条提示 | 关闭提示 | 无障碍标签用「关掉」口语；且「这条」无上下文。 |
| `ui/src/react/components/DocViewer.tsx:158` | 正在读取说明文档… | 正在读取说明文档… | 保持。 |
| `ui/src/react/views/cheats/CheatsView.tsx:44` | 复制失败，请手动选中 | 复制失败，请手动选中文本 | 缺中心语。 |
| `ui/src/react/views/cheats/CheatsView.tsx:91` | 没有匹配的作弊码。换个关键词试试，或者点「全部」。 | 没有匹配的作弊码。可更换关键词，或点「全部」查看所有作弊码。 | 「换个…试试」口语。 |
| `ui/src/react/views/cheats/CheatsView.tsx:118` | 共 {N} 条。带战役名的只在对应战役里生效，其余通用。 | 共 {N} 条。标注战役的作弊码仅在该战役中生效，其余为通用。 | 「带战役名的」「其余通用」口语省略。 |

## 9. 原生对话框（rfd，Rust 侧）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `src-tauri/src/lib.rs:551` | 选一个目录加进这个包 | 选择要加入包的目录 | 「加进这个包」口语，且没有说明是加入哪个包。 |
| `src-tauri/src/lib.rs:571` | 导出到哪里 | 选择导出位置 | 「导出到哪里」是问句式标题，原生对话框标题宜用动宾短语。 |
| `src-tauri/src/lib.rs:573` | 压缩包 | ZIP 压缩包 | 过滤器名只有「压缩包」，而实际只允许 zip；写明格式减少误选。 |
| `src-tauri/src/lib.rs:582-583` | 选一张封面图 / 选一份说明书（PDF） | 选择封面图 / 选择说明书（PDF） | 「选一张」「选一份」量词口语。 |
| `src-tauri/src/lib.rs:587` | 文件 | 说明书文件 | 过滤器名「文件」没有信息量。 |
| `src-tauri/src/lib.rs:1141` | 选择模组包 | 选择模组包 | 保持。 |
| `src-tauri/src/lib.rs:1151` | 模组包 | 模组包（zip） | 过滤器未说明支持格式；导入实际走系统解压工具，可接受但建议注明。 |
| `src-tauri/src/lib.rs:1156` | 所有文件 | 所有文件 | 保持（兜底过滤器）。 |
| `src-tauri/src/lib.rs:1301` | 导出模组包 | 导出模组包 | 保持。 |
| `src-tauri/src/lib.rs:1338-1340` | 导出模组版本 / {name}-版本包.zip | 导出模组版本 / {name}-版本.zip | 同一流程里「模组包」与「版本包」两个词；默认文件名带「版本包」对用户是多余负担。 |
| `src-tauri/src/lib.rs:1691` | 战役包 | 战役包（zip） | 同 :573。 |
| `src-tauri/src/lib.rs:1966` | 选择战役包 / 补丁包 | 选择战役包或补丁包 | 斜杠连接两个名词，中文宜用「或」。 |
| `src-tauri/src/lib.rs:1982` | 选择星际争霸 II 安装目录 | 选择星际争霸 II 安装目录 | 保持。与「游戏目录」术语需统一（见 §11）。 |
| `src-tauri/src/lib.rs:2212` | 弥音启动器出错了 | 弥音启动器发生错误 | 「出错了」口语；这是崩溃对话框标题，应更正式。 |
| `src-tauri/src/lib.rs:2213-2215` | 启动器遇到了一个没处理的问题。原因已经写进程序目录的 miyin-crash.log，反馈时可以把它一起发给我们。（换行）{text} | 启动器遇到未处理的错误。详细信息已写入程序目录的 miyin-crash.log；反馈问题时请附上该文件。（换行）{text} | 「遇到了一个没处理的问题」「把它一起发给我们」口语；「程序目录」应说明是可执行文件所在目录。 |
| `src-tauri/src/lib.rs:2296` | 缺少 WebView2 运行时 | 缺少 Microsoft Edge WebView2 运行时 | 标题可更具体，与正文一致。 |
| `src-tauri/src/lib.rs:2297-2301` | 弥音启动器要靠 Microsoft Edge WebView2 运行时才能显示界面，这台机器上没有装。（换行）请打开 https://… 下载并安装「常青版」（Evergreen Runtime），装好之后再打开启动器。（换行）Windows 10/11 一般自带；重装过系统或精简版系统上可能需要手动装。 | 弥音启动器需要 Microsoft Edge WebView2 运行时才能显示界面，当前系统未安装。（换行）请访问 https://… 下载并安装「常青版（Evergreen Runtime）」，安装完成后重新打开启动器。（换行）Windows 10/11 通常已自带；重装系统或精简版系统可能需要手动安装。 | 「要靠」「这台机器上没有装」「装好之后」口语；正文信息准确，只差语气。 |

## 10. 返回给界面的错误串与提示串（src-tauri / miyin-core）

| 文件:行 | 原文 | 建议改写 | 理由/问题 |
| --- | --- | --- | --- |
| `src-tauri/src/lib.rs:232` | 内部状态异常，请重启启动器 | 内部状态异常，请重启启动器 | 保持。简短、有下一步，合格。 |
| `src-tauri/src/lib.rs:255` | 这个版本带了模组，但挂载清单是空的 —— 直接打开地图会缺资源。到「模组」页把它们重新打开 | 此版本包含模组，但未启用任何模组，直接打开地图会缺少资源。请到「模组」页重新启用所需模组。 | **Lead 本轮新改的拦截文案，纳入审查**：①「挂载清单」是内部数据名；②「它们」指代不清；③「重新打开」易被理解成重复点击；④破折号之前缺句读。 |
| `src-tauri/src/lib.rs:269` | 尚未设置星际争霸 II 安装目录，请到「设置」中选择 | 尚未设置星际争霸 II 安装目录。请到「设置」中选择游戏目录。 | 「尚未」与同文件其他「还没找到」语体不一；「安装目录」与界面「游戏目录」混用（见 §11）。 |
| `src-tauri/src/lib.rs:491,515,543,562` | 还没找到星际争霸 II 的安装目录 | 尚未检测到星际争霸 II 安装目录 | 同一句话在 4 处重复（可抽常量）；「还没找到」口语。 |
| `src-tauri/src/lib.rs:495` | 找不到「我的文档 / StarCraft II」—— 游戏还没产生过存档？ | 找不到「我的文档 / StarCraft II」目录，可能尚未产生存档。 | 反问句不适合错误提示；应用陈述句 + 可能原因。 |
| `src-tauri/src/lib.rs:819` | 无法从包内识别它属于哪个战役，请手动指定 | 无法从包内识别所属战役，请手动指定。 | 「它」口语指代。 |
| `src-tauri/src/lib.rs:927` | 库里找不到版本 {variant_id} | 战役库中找不到版本 {variant_id} | 「库里」内部叫法。 |
| `src-tauri/src/lib.rs:968` | 没找到游戏编辑器 —— 到「设置」里重新指定游戏目录试试 | 未找到游戏编辑器。请到「设置」中重新选择游戏目录。 | 「没找到」「试试」口语；「重新指定」与按钮「重新选择」不一致。 |
| `src-tauri/src/lib.rs:1001` | 编辑器已打开这张地图，按 Ctrl+F9（菜单「测试文档」）就能进入游戏。 | 编辑器已打开此地图。按 Ctrl+F9（菜单「测试文档」）开始游戏。 | 「就能」口语；「这张」→「此」。 |
| `src-tauri/src/lib.rs:1021` | 不认识这个工具：{id} | 未知工具：{id} | 「不认识」拟人化。 |
| `src-tauri/src/lib.rs:1106,1110` | 没找到 {wanted} 这个版本 / {name} 还没有可下载的版本 | 未找到版本 {wanted} / {name} 暂无可下载版本 | 两处口语；且「这个版本」与前半句重复。 |
| `src-tauri/src/lib.rs:1129` | 打不开浏览器：{error} | 无法打开浏览器：{error} | 「打不开」口语。 |
| `src-tauri/src/lib.rs:1305,1342` | 已取消 | 已取消 | **不作为错误返回**：前端靠 `text.includes("取消")` 判断是否已取消（useMods.ts:173/190），改文案会悄悄破坏取消逻辑。见 §13 拍板项。 |
| `src-tauri/src/lib.rs:1345` | Ok({N} 个文件 -> {path}) | 已导出 {N} 个文件到 {path} | 面向用户的成功串里用 `->` 是命令台风。 |
| `src-tauri/src/lib.rs:1575` | 没找到 {version} 这个版本，请重新检查更新 | 未找到版本 {version}。请重新检查更新。 | 口语 + 逗号粘连。 |
| `src-tauri/src/lib.rs:1578` | 这个发行版没有适合 Windows 的包 | 该版本没有适用于 Windows 的安装包 | 「发行版」是内部词（Release）。 |
| `src-tauri/src/lib.rs:1628` | 还没有下载好的更新 | 尚未下载更新 | 口语。 |
| `src-tauri/src/lib.rs:1653` | 只允许打开 http(s) 链接 | 仅支持打开 http(s) 链接 | 「只允许」训人感。 |
| `src-tauri/src/lib.rs:1898,1940,1949,1953,1959` | 路径不存在：{path} / 无法打开目录：{path} / 无法定位这个文件：{path} / 无法在资源管理器里选中它：{error} / 当前平台暂不支持打开资源管理器 | 路径不存在：{path} / 无法打开目录：{path} / 无法定位文件：{path} / 无法在资源管理器中选中该文件：{error} / 当前平台不支持打开资源管理器 | 一组「无法…」句式基本合格；只需去掉「这个」「里」「它」。 |
| `src-tauri/src/lib.rs:2494` | 启动游戏失败：{error} | 启动游戏失败：{error} | 保持。但应补一句可做什么（检查游戏目录/是否已在运行），见 §12。 |
| `src-tauri/src/lib.rs:2498` | 已经交给星际争霸，正在加载这张地图。 | 已交给星际争霸，正在加载地图。 | 「这张」口语；「已经」与句尾句号可省。 |
| `crates/miyin-core/src/error.rs:11` | 不是一个有效的星际争霸 II 安装目录：{path}（缺少 {marker}） | 不是有效的星际争霸 II 安装目录：{path}（缺少 {marker}） | 「一个」口语量词；括号里英文文件名可保留。 |
| `crates/miyin-core/src/error.rs:15` | 没有找到星际争霸 II 安装，请手动指定游戏目录 | 未找到星际争霸 II 安装，请手动选择游戏目录 | 「没有找到」口语；「指定」与按钮「选择」不一致。 |
| `crates/miyin-core/src/error.rs:23` | 路径越界：{target} 不在允许的根目录 {root} 内 | 路径越界：{target} 不在允许的目录 {root} 内 | 「根目录」是开发词；但此错误属内部兜底，用户很少见到。 |
| `crates/miyin-core/src/error.rs:35` | 战役包被拒绝：{0} | 战役包不可用：{0} | 「被拒绝」有指责感，且外层再套一层「拒绝」会读成双主语。 |
| `crates/miyin-core/src/error.rs:47,51,55` | IO 错误：… / JSON 处理失败：… / 压缩包处理失败：… | 读取或写入失败：… / 数据文件解析失败：… / 压缩包处理失败：… | IO/JSON 是开发缩写，直接进用户提示。 |
| `crates/miyin-core/src/campaign/contents.rs:442` | 无法解开这个压缩包。请安装 7-Zip，或解压后重新打成 zip。已尝试：{} | 无法解压此压缩包。请安装 7-Zip，或解压后重新打包为 zip。已尝试：{} | 「解开」「打成」口语；「已尝试」后面直接列程序名，建议加「：」。 |
| `crates/miyin-core/src/library/install.rs:659` | 拒绝往 {} 写东西 —— 只能写星际争霸安装目录里面的文件 | 无法写入 {}：只能写入星际争霸 II 安装目录内的文件 | 「拒绝」「写东西」「里面」三处口语；且缺「II」。 |
| `crates/miyin-core/src/library/install.rs:464` | {} 也要用 {}，现在由{}占着 —— 已改由{}接管 | {} 也需要 {}，当前由 {} 占用，现已改由 {} 接管 | 「占着」口语；句子没有标点。 |
| `crates/miyin-core/src/library/store.rs:366` | 同名版本过多，请先清理一些版本 | 同名版本过多，请先删除部分版本 | 「清理」含糊（删还是改名？）。 |
| `crates/miyin-core/src/library/store.rs:88,93` | 压缩包内容为空 / 解压后超出体积或条目上限 | 压缩包为空 / 解压后超出体积或文件数量上限 | 「条目」是压缩包 API 术语。 |
| `crates/miyin-core/src/library/mods.rs:151` | 模组 {} 的内容不是一个文件 | 模组 {} 的内容不是单个文件 | 「不是一个文件」口语且否定范围含糊。 |
| `crates/miyin-core/src/library/mods.rs:399` | 同名模组太多了 | 同名模组过多 | 「太多了」口语。 |
| `crates/miyin-core/src/library/mods.rs:712` | 这个包里没有找到模组（既没有 .SC2Mod 也没有 Modules 目录） | 未在包中找到模组（既无 .SC2Mod 文件，也无 Modules 目录） | 「这个包里没有找到」语序口语。 |
| `crates/miyin-core/src/library/mods.rs:1210,1245` | 模组文件已经不在了 / 这个模组目录是空的 | 模组文件已不存在 / 模组目录为空 | 「不在了」「空的」口语。 |
| `crates/miyin-core/src/library/mods.rs:1265` | 没有选中任何版本 | 未选择任何版本 | 口语。 |
| `crates/miyin-core/src/library/patch.rs:101` | 这不是补丁包（包内既有地图又有模组，或声明了 campaign） | 这不是补丁包：包内同时包含地图与模组，或声明了 campaign 字段 | 「campaign」是字段名，用户看不懂；括号解释应写成原因。 |
| `crates/miyin-core/src/library/patch.rs:258` | 补丁名不能为空 | 补丁名称不能为空 | 与界面「补丁名」字段 label 不一致（前者带「称」）。 |
| `crates/miyin-core/src/library/export.rs:73,87` | 这个版本里没有可导出的内容 / 这个版本本体里没有内容，只有补丁 | 此版本没有可导出的内容 / 此版本本身没有内容，仅包含补丁 | 「本体」是开发词。 |
| `crates/miyin-core/src/update/apply.rs:164` | 更新包里没有找到 exe，可能下错了文件 | 更新包中未找到可执行文件，可能下载了错误的包 | 「exe」「下错了」口语/技术混合。 |
| `crates/miyin-core/src/update/check.rs:416` | 校验失败：期望 {expected}，实际 {actual}。文件已删除，请重试 | 校验失败：期望 {expected}，实际 {actual}。文件已删除，请重新下载。 | 「请重试」不明确重试什么。 |
| `crates/miyin-core/src/update/net.rs:211,244` | 没有可用的下载地址 / 所有下载地址都失败了：{} | 没有可用的下载地址 / 所有下载地址均失败：{} | 「都失败了」口语。 |
| `crates/miyin-core/src/library/mod.rs:478` | 找不到「{wanted}」这张地图，请自己挑一张作为启动入口 | 找不到地图「{wanted}」，请选择一张地图作为启动入口 | 「这张」「自己挑一张」口语。 |
| `crates/miyin-core/src/library/mod.rs:1013` | 找不到地图文件：{map} | 找不到地图文件：{map} | 保持。 |
| `crates/miyin-core/src/library/mod.rs:1446` | 未知的战役槽位：{slug} | 未知的战役：{slug} | 「槽位」是内部数据结构名。 |
| `crates/miyin-core/src/library/mod.rs:1460` | 官方目录尚不存在，启用时会自动创建：{} | 原版目录尚不存在，启用时将自动创建：{} | 「官方」与「原版」术语不统一（见 §11）。 |
| `crates/miyin-core/src/library/mod.rs:1194` | 数据目录就在程序旁边 | 数据目录位于程序所在目录 | 「就在…旁边」口语。 |
| `crates/miyin-core/src/library/mod.rs:1207` | 数据目录用 {}（程序旁边还有一份旧的 data/，没动它） | 当前使用数据目录 {}（程序旁仍有旧 data/，未改动） | 「旁边」「没动它」口语。 |
| `crates/miyin-core/src/saves.rs:202` | 找不到存档备份：{name} | 找不到存档备份：{name} | 保持。 |
| `crates/miyin-core/src/saves.rs:283` | 「{to}」里已经有同名存档 {file}，没有覆盖 | 「{to}」中已存在同名存档 {file}，未覆盖 | 「里」「没有覆盖」口语；需补「请改名或删除后重试」。 |
| `crates/miyin-core/src/saves.rs:261` | 存档文件名不合法：{file} | 存档文件名无效：{file} | 「不合法」有法律意味。 |
| `crates/miyin-core/src/saves.rs:313` | 未知的归属：{key} | 未知的存档归属：{key} | 缺中心语。 |
| `crates/miyin-core/src/library/activation.rs:201` | 「{}」的模组自动挂载失败：{error} | 「{}」的模组自动启用失败：{error} | 「挂载」+「自动」叠用；通知里用户不知道「挂载」是什么。 |
| `crates/miyin-core/src/library/activation.rs:252` | 该版本里没有可用的地图或模组 | 该版本中没有可用的地图或模组 | 「里」口语。 |
| `crates/miyin-core/src/tools/sc2diff.rs:52` | 还没装 SC2Diff —— 到「设置 → 可选工具」里装一下就能用这个功能 | 尚未安装 SC2Diff。请到「设置 → 可选工具」安装后使用此功能。 | 「还没装」「装一下就能用」口语；破折号连接两个句子。 |

## 11. 术语统一建议（战役 / 模组 / 补丁的「启用」体系）

### 11.1 现状：一个概念，五套说法

本轮在 `ui/src/react/**` 的界面字符串里统计（已排除注释、测试、房间展台）：

| 说法 | 次数 | 语义（代码里的实际含义） |
| --- | --- | --- |
| 启用 | 38 | ① 某版本成为当前版本并写入游戏目录；② 某模组加入该版本的启用集合 |
| 挂载 / 挂上 / 挂 | 16 | 同上 ②，但只出现在模组/补丁语境 |
| 停用 / 关闭 / 卸下 | 25 | 撤销 ① 或 ②；模组页「关闭」与右键菜单「停用」同义 |
| 装进游戏目录 / 装进 / 铺进 / 装上 | 20 | 写盘动作（复制文件到游戏目录） |
| 撤回 / 撤销 / 移除 / 删除 | 23 | 撤销写盘 / 从库删除 / 从游戏目录删除，三件事混用 |
| 收编 / 收进库 | 4 | 把游戏目录里已有的战役导入战役库 |
| 写进 | 1 | 写盘动作（预演对话框） |

**判定问题的关键**：代码里「启用」实际是两件事，文案把它们说成一件：

1. **选择**：索引里把某个版本标成当前版本、把某些模组加进该版本的启用集合（`ActiveManifest` / `mounted_mods`）；
2. **写入**：`activate` / `mods::sync` 把文件复制进游戏目录（AGENTS §14.11 的兜底那一段）。

两件事在界面上都用「启用 / 挂载 / 装进游戏目录」描述，于是出现「点了启用没反应」「勾了模组却提示一个都没挂载」这类反馈（相关 issue 反馈已记录）。

补充（2026-10-29 仓库 AGENTS §14.12 新增）：模组还有两个状态必须分开 —— `mounted`（用户配置里勾了）与 `in_game`（真的铺进了 `<游戏>/Mods/`）。规范原话是「界面上『已勾选』与『已生效』是两个状态；`in_game` 为 false 时不要写成『已启用』」。本表的术语建议按这条为准：**列表勾选框写「勾选 / 取消勾选」，写入结果写「已生效 / 未生效」**，不要再用「挂载 / 已启用」混着说。

### 11.2 推荐用词与替换规则

| 场景 | 推荐 | 不要用 | 替换规则 |
| --- | --- | --- | --- |
| 版本成为当前版本 | **启用 / 停用** | 切换、打开、装上、装进游戏目录、卸下 | 按钮与状态一律「启用 / 已启用 / 停用 / 已停用」。切回原版是「切换到原版战役」。 |
| 某版本的模组集合 | **勾选 / 取消勾选**（配置）；**已生效 / 未生效**（写入结果） | 挂载、挂上、挂、取消挂载、已启用 | 依 AGENTS §14.12：两个状态不能合成一个词。现有「挂载模组」「已挂 N / N」「已挂载 N 个模组」改为「已勾选 N / N」；写入状态单独标注。 |
| 补丁叠加 | **启用 / 停用**（保留「优先级」） | 挂载补丁、挂上、移出 | 「挂载补丁（N）」→「启用补丁（N）」；右上角「移出」→「停用」。 |
| 写盘这一步（只在预演/技术说明里出现） | **写入游戏目录** | 铺盘、铺进、装进去、装好、写进、挪进、盖上 | 用户可见的按钮/通知只给结果词（已启用/已停用）；文件级细节留在预演对话框正文，并统一用「写入 / 移除」。 |
| 撤销写盘 | **移除**（「从游戏目录移除」） | 撤回、撤掉、卸下、还原 | 「撤回」只保留在「撤销上一次操作」的语义里，且当前界面没有真正的撤销功能，建议整体不用。 |
| 从库删除 | **删除**（「从战役库删除」/「从模组库删除」） | 移出、移除、卸、卸载 | 删除是永久动作，不能与停用共用动词。 |
| 把游戏目录里的战役收进库 | **导入**（「从游戏目录导入」） | 收编、收进库、收进来 | 与「导入战役包」同一动词族。 |
| 包入库的动作 | **导入** | 安装 | 建议「导入整合包」「已导入」；「安装」只留给可选工具（SC2Diff 等）。**此项改变现有认知，需拍板（见 §13）**。 |
| 原版 vs 官方 | **原版战役** | 官方战役、官方目录 | 「官方」只在强调暴雪出品时用；目录名（`Maps/Campaign`）可写「原版战役目录」。 |
| 游戏安装位置 | **游戏目录**（首次提及时写「星际争霸 II 安装目录」） | 安装根目录、安装目录、游戏根目录 | 界面标题统一「游戏目录」；错误串与对话框标题用全称。 |
| 提示语标点 | 中文全角，按钮开对话框带「…」 | 半角括号、`->`、反引号、竖线 | 界面里不出现 Markdown 反引号与箭头（SavesView:190 已踩）。 |

### 11.3 三条判定原则

1. **一个动作一个动词**：启用 ≠ 写入 ≠ 删除 ≠ 导入。写盘是启用的**后果**，不是按钮名。
2. **内部词不出界面**：挂载、槽位、条目、元数据、快照、Bake/activate、data/backup、data/saves、localStorage、Tauri、diff —— 需要时用「包信息」「存档库」「配置文件」等用户词替代。
3. **状态与动作分开**：按钮显示「已启用/已停用」（状态），title 说明点击后做什么（动作），通知说「已启用/已停用」（结果）。当前 CustomCampaignPanel:271 正是把三者混在一起。

## 12. 优先修复 Top 20

排序依据：① 指代会失效（方位/颜色）；② 同一屏内自相矛盾；③ 事实错误；④ 训人/自嘲语气；⑤ 术语混乱导致理解成本。

1. `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:252-253` ——「点右边那个绿色的可以停用卸下」。用方位+颜色指按钮，改版、窄窗、换主题就失效；应写「点「已启用」按钮」。
2. `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:128` ——「已停用，装进去的文件都撤回了」。全句四个术语（停用/装进去/撤回），用户无法对应到按钮。
3. `ui/src/react/views/mods/ModsView.tsx:165,243,247` —— 同一页「全部关闭 / 点击关闭 / 已关闭」与右键菜单「停用」并存；一个动作三种叫法。
4. `src-tauri/src/lib.rs:255`（本轮新改）——「挂载清单是空的…把它们重新打开」。「挂载清单」是索引字段名，「重新打开」易被当成重复点击；建议「未启用任何模组，请到「模组」页重新启用」。
5. `ui/src/react/views/campaigns/ImportDialog.tsx:364-365` ——「你把它改到了…确认前请想一下」。质问 + 训人；改为陈述后果。
6. `ui/src/react/views/install/InstallView.tsx:216` ——「一个都没装上」。失败通知最需要说清「失败原因 + 下一步」，这里只有情绪。
7. `ui/src/react/app/AppShell.tsx:168-169` ——「认不出来会让你选，不会瞎猜」。自嘲表述，且「不会瞎猜」反而让用户担心别处会猜。
8. `ui/src/react/views/campaigns/CustomCampaignPanel.tsx:421` ——「而你一个都没勾」。第二人称「你」+ 责备语气。
9. `ui/src/react/views/saves/SavesView.tsx:190-191` —— 界面直出 Markdown 反引号（`虚空之遗 …`），并写「备份与切换还在做」暴露开发进度。
10. `ui/src/react/views/settings/SettingsView.tsx:126` ——「软件同级的 data 目录」与 AGENTS §14.1 事实不符（实际在游戏安装目录同级）。事实错误优先改。
11. `ui/src/react/views/settings/SettingsView.tsx:151-152` ——「所有写入都限制在 Maps / Mods 等目录之内」与 §10.5 已放开的写盘闸门不符。
12. `ui/src/react/views/custom/CustomView.tsx:180-181` ——「不装进游戏目录…直接启动游戏就能玩」与同页面板「装进游戏目录才能直接启动」正面矛盾。
13. `ui/src/react/views/merge/MergeDialog.tsx:47-49` ——「都想当战役库…你说了算」。口语 + 把架构冲突说成拟人争吵。
14. `ui/src/react/views/mods/ModSheets.tsx:56-59` ——「它跟原版是两码事」。「原版」在战役语境指官方战役，在模组语境指未修改版本，同一个词两种指代。
15. `ui/src/react/views/campaigns/SlotMenu.tsx:572` ——「用压缩包把这一版换掉（删旧的、装新的…）」。title 里最口语的一处，且「删旧的」会让人以为包内旧文件会被删。
16. `ui/src/react/views/campaigns/SlotMenu.tsx:802-803` ——「删了就得重新导入」。威胁式口吻；应写「删除后需要重新导入」。
17. `src-tauri/src/lib.rs:2213-2215` ——「启动器遇到了一个没处理的问题…把它一起发给我们」。崩溃对话框是用户最慌的一刻，措辞应最稳。
18. `src-tauri/src/lib.rs:968` ——「到「设置」里重新指定游戏目录试试」。错误提示给「试试」等于没给确定性；且「重新指定」与按钮「重新选择」不一致。
19. `ui/src/react/views/campaigns/ImportDialog.tsx:487-491` ——「多半进游戏后找不到关卡，玩不了」。三处口语叠加，且「多半」弱化了警告。
20. `ui/src/react/views/mods/ModsView.tsx:113-118`（并行新增）——「模组由启动器在启动时自动挂载，非开发者请勿手动调整…」。把用户分「开发者/非开发者」；「自动挂载」是内部模型词（AGENTS §14.12），界面应说「启动时自动同步/启用」，并与列表里的「已勾选 / 已生效」对上。

次级（Top 30 边缘，改动成本同样低）：`ActivationConflictDialog.tsx:72`「等 N 个」、`startup.ts:102`「界面没有挂载出来」、`ToolsPanel.tsx:134-135`「想装的话点上面的」、`CheatsView.tsx:91`「换个关键词试试」、`SlotCard.tsx:15`「已启用：名字」。

## 13. 需要用户拍板的取舍（会改变语义或已有认知）

| # | 议题 | 选项 | 影响 / 建议 |
| --- | --- | --- | --- |
| 1 | 自制战役到底「装不装进游戏目录」 | A. 统一说「启用后写入游戏目录，由启动器启动」；B. 保留「不装进游戏目录」的说法并改实现 | 现状文案自相矛盾（CustomView:180 vs CustomCampaignPanel:253）；代码实际走 `activate` 写入 `Maps/CustomCampaigns`，**推断 A 是事实**，但这是产品口径，需确认。 |
| 2 | 「安装整合包」还是「导入整合包」 | A. 全站统一「导入」；B. 保留「安装」（包=安装，库=导入） | 影响入口按钮、通知、InstallView 标题与教程；老用户已形成的「安装包」认知会被改。 |
| 3 | 模组语境是否保留「挂载」 | A. 改「启用」；B. 保留「挂载模组」作为专有说法 | 仓库文档、包作者指南、社区教程大量用「挂载」；改成「启用」后搜索与教程对不上，但界面终于一致。建议 A，并在 FAQ 里保留「挂载（即启用）」一次。 |
| 4 | 「原版」一词同时用于战役与存档组 | A. 存档组改名「原始存档」；B. 保留 | 「原版战役」是主线概念；存档隔离里「原版」指开启隔离时的那份存档，两个「原版」在同一设置页相邻出现。 |
| 5 | 「启用」是否要拆成「选择」与「应用」两个词 | A. 保持一个词（推荐，用户不需要知道两阶段）；B. 界面上拆开 | AGENTS §14.12 已要求界面区分「已勾选」（`mounted`）与「已生效」（`in_game`），所以模组语境必须拆；版本语境可以仍用一个「启用」。除非产品打算把「应用到游戏目录」做成显式第二步。 |
| 6 | 错误提示是否继续暴露内部路径与技术细节 | A. 保留 `data/backup`、`data/saves`、zip/HTTP 细节；B. 用户提示只给「发生了什么 + 能做什么」，细节进日志 | 保留利于用户自助与反馈定位，但普通用户读不懂；建议 B + 「查看日志」入口（当前已有内嵌终端可复用）。 |
| 7 | 「已取消」作为错误串返回 | A. 改协议（返回结构化 `cancelled`）；B. 只改文案并同步 `useMods.ts` 的判断 | 现在前端靠 `text.includes("取消")` 识别（useMods.ts:173/190），**改文案会静默破坏取消逻辑**。若只改文案必须同时改判断，属行为改动，需批准。 |
| 8 | 识别可信度三层措辞 | A. 确定 / 较可靠 / 推测；B. 保留「猜的」 | 「猜的」自嘲但确实在提醒用户别信；换成「推测」更专业，警示度略降。建议 A，并在「推测」旁保留图标/颜色提示。 |
| 9 | 「更新」一词多义 | A. 区分「更新启动器」「替换已安装版本」「检查更新」；B. 保持现状 | 现状「更新战役包」「更新这一版」「检查更新」「安装并重启」四种含义；A 更准确但按钮变长。 |
| 10 | 用户设置页里的开发者内容 | A. 收敛措辞（去掉 AGENTS 引用、Tauri、localStorage、生产界面）；B. 原样保留 | 该区块默认折叠且需主动开启（`开发者模式`），但一旦开启，普通用户也会看到内部规范引用。建议 A（纯文案，不改功能）。 |
| 11 | 存档「隔离 / 分组 / 快照」三个词 | A. 统一为「分组」；B. 保留三词 | 「按战役隔离存档」是功能名，「组」「快照」是数据形态，用户会把「组」当成文件夹。建议 A + 首次开启时一句说明。 |
| 12 | 标点与省略号规范 | A. 统一全角、按钮开对话框带「…」；B. 逐处判断 | A 会产生一批纯标点 diff；收益是界面细节一致。可放到术语统一之后单独一轮。 |

## 14. 附录

### A. 范围与排除

- **已覆盖**：`ui/src/react/**`（用户侧组件、页面、状态 hook 中的界面字符串）、`ui/src/api/errors.ts`、`src-tauri/src/lib.rs`（命令返回串、rfd 原生对话框、崩溃/WebView2 对话框）、`crates/miyin-core/src/**` 中会进入界面的错误串（`Error` Display、`PackageRejected`、`LibraryNote`）。
- **未覆盖（按任务约定排除）**：`ui/src/ide/**` 开发者页；`ui/src/react/views/room/**`（开发者房间与 toast-lab 展台，含「候选 A/B/C」等演示文案）；代码注释；`docs/**`；CLI 输出（`crates/miyin-core/src/tools/` 的 stdout）；`ui/src/react/**/*.test.*`；CSS 文件；`ui/src/api/bridge.ts` 里的浏览器演示模式假数据（其中 `guidance` 字段会被界面显示，但真实运行时该串来自 Rust 后端；演示模式下的假数据本轮未逐条审查）。
- **推断项**：凡「推断」字样，均表示无法仅凭代码确认产品意图或实际渲染（例如 HTML 转义、是否被其他组件覆盖）。

### B. 方法

1. 先 `grep '\p{Han}'` 列出所有含中文的行；2. 用「行首 // * /* {/*」过滤代码注释；3. 逐文件读上下文确认该行是 JSX 文本 / 字符串常量 / title / aria-label / notify；4. Rust 侧先抓 `#[error(...)]`、`PackageRejected`、`Err("...")`、rfd 的 `set_title` / `set_description`，再回读上下文取完整句。

### C. 统计

- 本表共 **308 条**（§1–§10），按模块见各节表头。
- 术语出现频次见 §11.1。
- 其中「事实错误 / 自相矛盾 / 指代会失效」类问题 4 条（§12 第 1、10、11、12 条）。

### D. 快照（行号基准）

审查时工作区含未提交改动：`src-tauri/src/lib.rs`、`crates/miyin-core/src/**`（部分文件因格式化整体位移 1 行，`lib.rs` 后半位移 8 行）、`ui/src/react/views/mods/{ModsView.tsx,mods.css}`、`ui/src/react/views/campaigns/CustomCampaignPanel.tsx`。
文件内容哈希（sha256 前 12 位，用于确认行号对应版本）：

| 文件 | sha256[:12] |
| --- | --- |
| src-tauri/src/lib.rs | `217eb071028c` |
| crates/miyin-core/src/error.rs | `fce794a2af15` |
| crates/miyin-core/src/library/mod.rs | `1388be6a804c` |
| crates/miyin-core/src/library/activation.rs | `dace46d31cdb` |
| crates/miyin-core/src/library/install.rs | `56bc29dd4f65` |
| crates/miyin-core/src/library/mods.rs | `bf5403b45df5` |
| crates/miyin-core/src/saves.rs | `38c6fe1afb3a` |
| ui/src/react/views/campaigns/CampaignsView.tsx | `3d031db1012d` |
| ui/src/react/views/campaigns/CustomCampaignPanel.tsx | `9c477e7f1113` |
| ui/src/react/views/mods/ModsView.tsx | `0c1b3ed484db` |
| ui/src/react/views/settings/SettingsView.tsx | `327ba390fa7b` |

> 上游若继续修改上述文件，行号会偏移；请以「原文」搜索定位。

### E. 未收录但建议后续单独过一遍

- `ui/src/api/bridge.ts` 的演示模式假数据（`guidance`、`message`、`name` 等字段会在演示模式直接显示）。
- `ui/src/react/views/room/**` 与 `toast-lab/**` 的演示文案（当前定位为内部展台，不算用户文案，但会出现在开发者房间）。
- `crates/miyin-core/src/dev/**`（开发者页后端）的提示串。
- `crates/miyin-core/src/library/tests.rs` 中的测试断言文本（不影响用户）。
