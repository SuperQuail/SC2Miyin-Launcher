//! 战役库：让**同一个战役的多个玩家版本共存**，并可自由切换。
//!
//! # 为什么要独立于游戏目录
//!
//! 游戏目录里的 `Maps/CustomCampaigns` 只能放「当前这一份」，不同玩家的改版会互相覆盖，
//! 装第二个版本就得先删掉第一个。所以库放在**启动器自己的数据目录**里：
//!
//! ```text
//! <启动器目录>/data/
//! ├── library.json      # 索引：每个槽位下有哪些版本、当前启用哪个
//! ├── installed.json    # 安装清单：游戏目录里哪些东西是我们放的、谁放的
//! ├── campaigns/
//! │   └── wol/
//! │       ├── 自由之翼：重生 v1.4.2/
//! │       └── 自由之翼：重生 v1.5.0/
//! └── backup/           # 被挪走的官方文件，切回原版时原样还原
//! ```
//!
//! # 槽位
//!
//! 「槽位」= 一个官方资料片（自由之翼 / 虫群之心 / 进化 / 虚空之遗 / 序章 / 诺娃）。
//! 每个槽位默认是**原版战役**（`active = None`），导入的版本作为可切换的选项。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::campaign::CampaignFormat;
use crate::campaign::metadata::CampaignType;
use crate::campaign::metadata::PackageKind;
use crate::campaign::package::{Payload, PayloadTarget};
use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;

/// 原子写：先写同目录的临时文件，再改名覆盖。
///
/// 直接 fs::write 在断电 / 被强杀 / 杀软截断时会留下半个 JSON，
/// 而那个半个文件下次会被当成**空**索引读进来 —— 整个库就在界面上消失了。
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    // Windows 上 rename 会覆盖已存在的目标（MoveFileEx 语义）
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub mod activation;
pub mod compat;
pub mod compose;
pub mod install;
pub mod known;
pub mod mods;
pub mod naming;

pub use mods::{ModChanges, StandaloneMod};
pub mod export;
pub mod merge;
pub mod patch;
pub mod store;

#[cfg(test)]
mod tests;

pub use activation::{activate, deactivate, preview};
pub use install::{Installed, Item, Manifest, Owner, Plan, Preview, PreviewEntry};
pub use store::{
    Replaced, VariantChanges, find_target_for_package, import, remove_variant, replace_variant,
    update_variant,
};

/// 索引文件的格式版本，便于以后迁移。
const INDEX_VERSION: u32 = 1;

/// 库内的一个战役版本，即「某个玩家做的某一版」。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Variant {
    /// 槽位内唯一 id，同时是它在库中的目录名。
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub format: CampaignFormat,
    /// 导入时间（Unix 秒）。
    pub imported_at: u64,
    /// 来源包的原始文件名（仅作记录）。
    pub source: Option<String>,
    pub map_count: usize,
    pub mod_count: usize,
    pub size_bytes: u64,
    /// 启用时地图应当落到 @@Maps/Campaign@@ 下的哪个子目录。
    ///
    /// 由**包内声明的 campaign** 决定（进化包 -> @@swarm/evolution@@），而不是由槽位决定 ——
    /// 这样「虫群之心」这一个槽位里可以同时放本体包与进化包。
    #[serde(default)]
    pub target_sub: Option<String>,
    /// 包自带的封面图（相对版本目录的路径）；没有就用该战役的官方美术。
    #[serde(default)]
    pub cover: Option<String>,
    /// 包自报的标签。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 注册 ID：补丁靠它引用战役，更新靠它认出"同一个战役的新版本"。
    #[serde(default)]
    pub registration_id: Option<String>,
    /// 包类型：战役本体还是补丁。
    #[serde(default)]
    pub kind: PackageKind,
    /// 载荷清单：哪些地图/模组、各自落到游戏目录的哪里。
    ///
    /// **必须存下来**：镜像包里的地图可能分布在多个子目录（`void` 与 `voidprologue`），
    /// 靠扫目录去猜落点会摆错位置。
    #[serde(default)]
    pub payloads: Vec<Payload>,
    /// 作者声明的覆盖规则；旧索引里没有这个字段，读出来是空的。
    #[serde(default)]
    pub overrides: Vec<crate::campaign::metadata::OverrideRule>,
    /// **主地图**：自制战役的游玩入口，相对版本根目录的路径。
    ///
    /// 有些自制战役有一张总入口地图，打开它就能一路玩到底；也有的只能一张一张打。
    /// 这里存的是**相对路径**（`1. Rebel Yell/Terran01.SC2Map`）——
    /// 只存文件名的话，不同章节里重名的地图会撞车。
    ///
    /// 导入时用包内声明的值填充，之后用户可以在界面上改。
    #[serde(default)]
    pub main_map: Option<String>,
    /// **挂载到游戏目录的模组**，存载荷的 source（相对版本目录的路径）。
    ///
    /// 地图可能依赖包里的模组，模组不铺进 `<游戏>/Mods/` 就打不开；
    /// 但不同战役的模组之间会互相打架，所以让用户自己选挂哪几个。
    ///
    /// - `None` = **还没配过**，按「全挂」处理（老记录也是这种）
    /// - `Some([])` = 用户**明确**一个都不挂
    ///
    /// 用 `Option` 而不是空 `Vec` 就是为了区分这两种情况 ——
    /// 否则早先导入的记录（当时还没有这个字段）会被当成"用户取消了一切"，
    /// 官方战役包里的模组会突然不铺了。
    #[serde(default)]
    pub mounted_mods: Option<Vec<String>>,
    /// **说明文档（PDF）**，相对版本目录的路径。
    ///
    /// 导入时按包内声明解析；没声明就按文件名特征找（说明 / readme / manual…）。
    /// 找不到就是 `None` —— 界面据此决定不显示「说明」入口。
    #[serde(default)]
    pub doc: Option<String>,
    /// 包内**声明为依赖**的模组键（`Mods/` 之后的第一段）。
    ///
    /// 与 `mounted_mods` 不是一回事：这个说的是「作者要求必须有」，
    /// 那个说的是「用户当前挂了哪些」。界面拿它标出哪些是必需的。
    #[serde(default)]
    pub declared_mods: Vec<String>,
}

/// 模组是从哪来的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModOrigin {
    /// 跟着**原版战役**的改版包进来的。
    OfficialCampaign,
    /// 跟着**自制战役**包进来的。
    CustomCampaign,
    /// **单独导入**的，不属于任何战役。
    Standalone,
}

impl ModOrigin {
    /// 界面上的说法。
    pub fn label(&self) -> &'static str {
        match self {
            Self::OfficialCampaign => "原版战役包",
            Self::CustomCampaign => "自制战役包",
            Self::Standalone => "单独导入",
        }
    }
}

/// 库里的一个模组，带着它属于哪个版本的上下文。
#[derive(Debug, Clone, Serialize)]
pub struct LibraryMod {
    /// 所属槽位。
    pub slot: String,
    /// 槽位显示名（「自由之翼」「自制战役」…）。
    pub slot_name: String,
    /// 所属版本。
    pub variant_id: String,
    pub variant_name: String,
    /// 挂载键：载荷的 source，同时是相对版本目录的路径。
    pub path: String,
    /// 显示名（去掉 .SC2Mod 后缀）。
    pub name: String,
    pub mounted: bool,
    /// 这个模组由几个文件组成。
    pub parts: usize,
    /// 从哪来的。
    pub origin: ModOrigin,
    /// 是不是包内**声明为依赖**的模组。
    pub required: bool,
    /// 单独导入的模组才有：库里的 id。界面靠它决定能不能改信息 / 导出 / 删除。
    #[serde(default)]
    pub standalone_id: Option<String>,
    /// **modid**：同一个模组的多个版本靠它归到一起。
    #[serde(default)]
    pub modid: Option<String>,
    /// 版本号。
    #[serde(default)]
    pub version: Option<String>,
    /// **铺进游戏目录时用的名字**（原样保留的那个）。
    ///
    /// 显示出来是有意义的：地图里写的就是这个名字，用户一眼能看出对不对。
    #[serde(default)]
    pub folder: Option<String>,
    /// 铺成文件还是目录。
    #[serde(default)]
    pub kind: Option<String>,
    /// **模组记录的 id** —— 有它就能改信息、导出。
    ///
    /// 独立模组天然有；跟着战役包来的会在第一次编辑时建一条。
    #[serde(default)]
    pub mod_record_id: Option<String>,
    /// 这个模组的内容在哪（独立库 / 某个战役版本）。
    #[serde(default)]
    pub source_kind: String,
}

/// 版本自带的说明文档。
#[derive(Debug, Clone, Serialize)]
pub struct DocInfo {
    /// 相对版本目录的路径。
    pub path: String,
    /// 显示用的文件名。
    pub name: String,
    pub size: u64,
}

/// 版本里的一个模组。
#[derive(Debug, Clone, Serialize)]
pub struct ModEntry {
    /// 挂载键：`Mods/` 之后的第一段（文件夹名，或 .SC2Mod 文件名）。
    pub path: String,
    /// 显示名（去掉 .SC2Mod 后缀）。
    pub name: String,
    /// 是不是已经挂上了。
    pub mounted: bool,
    /// 这个模组由几个文件组成 —— `Alenger` 那种文件夹会有十几个。
    pub parts: usize,
}

/// 这个版本**实际**会铺哪些模组。
///
/// 没配过（`None`）时按「全挂」算 —— 保证老记录和刚导入的包都能正常跑。
pub fn effective_mounted_mods(variant: &Variant) -> Vec<String> {
    if let Some(list) = &variant.mounted_mods {
        return list.clone();
    }

    variant
        .payloads
        .iter()
        .filter_map(|payload| mod_identity(payload).map(|found| found.key))
        .collect()
}

/// 把一堆载荷按模组归并 —— **按文件夹去重**。
///
/// `Alenger/1钢铁.SC2Mod` 和 `Alenger/2贝希摩斯虫群.SC2Mod` 归成一行「Alenger」，
/// 并记下它由几个文件组成。地图载荷会被忽略。
pub fn group_mods(payloads: &[Payload], mounted: &[String]) -> Vec<ModEntry> {
    let mut rows: Vec<ModEntry> = Vec::new();

    for payload in payloads {
        let Some(found) = mod_identity(payload) else {
            continue;
        };

        match rows.iter_mut().find(|row| row.path == found.key) {
            Some(row) => row.parts += 1,
            None => rows.push(ModEntry {
                mounted: mounted.contains(&found.key),
                path: found.key,
                name: found.name,
                parts: 1,
            }),
        }
    }

    rows
}

/// 一个模组的身份。
///
/// **模组按文件夹分，不按单个文件分** —— 这是从真实样本学到的：
///
/// `@text
/// Mods/
/// ├── 3疯批帝国之翼.SC2Mod        单文件         -> 一个模组
/// ├── Alenger/                    普通文件夹     -> 一个模组
/// │   ├── 1钢铁.SC2Mod                            （里面 18 个 .SC2Mod）
/// │   └── …
/// └── kit_liberty_story.SC2Mod/   解开目录树     -> 一个模组
/// `@
///
/// 判定办法：取落点里 `Mods/` 之后**第一段**。
/// `Alenger/1钢铁.SC2Mod` 与 `Alenger/2贝希摩斯虫群.SC2Mod`
/// 是同一个模组「Alenger」的两部分，不是两个模组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModKey {
    /// 挂载键：`Mods/` 之后的第一段（文件夹名，或 .SC2Mod 文件名）。
    pub key: String,
    /// 显示名：去掉 .SC2Mod 后缀。
    pub name: String,
}

/// 认出一个载荷属于哪个模组；地图返回 `None`。
///
/// 两种写法都算模组：
/// - `PayloadTarget::Mod` —— 包内是裸的 `X.SC2Mod`
/// - `PayloadTarget::Mirror` 且落在 `Mods/` 下 —— 包内是游戏目录镜像
///   （真实样本几乎都是这种）
pub fn mod_identity(payload: &Payload) -> Option<ModKey> {
    match &payload.target {
        // 裸的 .SC2Mod：落点是 `Mods/{name}`，名字本身就是键
        // （名字里可能还带子目录，所以拼上前缀交给统一的解析）
        PayloadTarget::Mod { name } => mod_key_of(&format!("Mods/{name}")),
        // 游戏目录镜像：路径已经带 Mods/ 前缀了
        PayloadTarget::Mirror { path } => mod_key_of(path),
        PayloadTarget::Map { .. } => None,
    }
}

/// 把作者写的依赖模组名字归一化成键（**不认识 `Mods/` 前缀也认**）。
///
/// `@text
/// Mods/Alenger/          -> Alenger
/// Mods/Alenger           -> Alenger
/// Alenger                -> Alenger
/// Alenger.SC2Mod         -> Alenger
/// 3疯批帝国之翼.SC2Mod    -> 3疯批帝国之翼
/// `@
///
/// 包作者写依赖时偷懒不写前缀是很常见的，不能因此就当没声明。
pub fn normalize_mod_key(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/').replace('\\', "/");
    let without_prefix = trimmed
        .strip_prefix("Mods/")
        .or_else(|| trimmed.strip_prefix("mods/"))
        .unwrap_or(&trimmed);
    let first = without_prefix.split('/').next().unwrap_or(without_prefix);

    if first.to_ascii_lowercase().ends_with(".sc2mod") {
        first[..first.len() - ".SC2Mod".len()].to_string()
    } else {
        first.to_string()
    }
}

/// 从一条 `Mods/` 下的相对路径推出模组身份。
pub fn mod_key_of(relative: &str) -> Option<ModKey> {
    let normalised = relative.replace('\\', "/");
    let rest = normalised
        .strip_prefix("Mods/")
        .or_else(|| normalised.strip_prefix("mods/"))?;
    let first = rest.split('/').next().filter(|part| !part.is_empty())?;

    // 显示名去掉 .SC2Mod 后缀（大小写不敏感）
    let lower = first.to_ascii_lowercase();
    let name = if lower.ends_with(".sc2mod") {
        first[..first.len() - ".SC2Mod".len()].to_string()
    } else {
        first.to_string()
    };

    Some(ModKey {
        key: first.to_string(),
        name,
    })
}

/// 版本里的一张地图。
#[derive(Debug, Clone, Serialize)]
pub struct MapEntry {
    /// 相对版本根目录的路径，用 `/` 分隔 —— 主地图存的就是这个形式。
    pub path: String,
    /// 显示名（文件名去掉扩展名）。
    pub name: String,
    /// 所属章节：版本根下的第一层目录；地图直接躺在根下时是 `None`。
    pub chapter: Option<String>,
    pub size: u64,
    /// 是不是当前设为主地图的那张。
    pub is_main: bool,
}

/// 主地图的解析结果。
#[derive(Debug, Clone, Serialize)]
pub struct MainMapChoice {
    /// 选中的地图（相对路径）。
    pub path: Option<String>,
    /// 是不是「只有一张地图，替你选了」。
    pub automatic: bool,
    /// 声明了却找不到时的提示 —— **只警告，不阻断**。
    pub warning: Option<String>,
}

impl MainMapChoice {
    /// 没定主地图。
    pub fn unset() -> Self {
        Self {
            path: None,
            automatic: false,
            warning: None,
        }
    }
}

/// 从地图列表里挑出该用哪张作为入口。
///
/// 规则（**不确定就不猜**）：
///
/// | 情况 | 结果 |
/// | --- | --- |
/// | 设了主地图，且**确实存在** | 用它 |
/// | 设了，但找不到 | 给个警告，退回「没定」 |
/// | 是**知名复刻战役**（见 `known`） | 用它表里的入口地图 |
/// | 没设，且整个版本只有一张地图 | 自动用它（省用户一次点击） |
/// | 没设，但包根有一张 **入口地图** | 自动用它（见下） |
/// | 其余 | 没定，让用户自己挑 |
///
/// **打表排在启发式前面** —— 知名复刻的入口是固定的，表比猜可靠。
/// 实测教训：SCMR 那种包里躺着第三方战役的启动器
/// （`Enslavers Redux Campaign Launcher`，在 `8. Enslavers Redux/` 里），
/// 用户在列表里挑很容易挑错那个；表直接点名自己的入口。
///
/// 「入口地图」的判定很窄：**放在包根**（不在章节子目录里）**且名字里带
/// launcher / 启动 / 入口**。真实样本教我们的 —— 复刻战役（SCMR）会在包根放一张
/// `SCMR Campaign Launcher.SC2Map`，那就是它的入口，不认出来用户就得在
/// 138 张地图里自己翻。
///
/// 这里敢猜是因为**猜错很便宜**：界面上一次点击就能改，不像「这个包属于哪部战役」
/// 那种猜错了整个装错地方。
/// 路径比较**忽略大小写与斜杠方向** —— 作者在包里写 `1. Rebel Yell\a.SC2Map`
/// 还是 `1. Rebel Yell/a.SC2Map` 都得认。
pub fn resolve_main_map(maps: &[MapEntry], declared: Option<&str>) -> MainMapChoice {
    let normalize = |value: &str| value.replace('\\', "/").to_lowercase();

    if let Some(wanted) = declared.map(str::trim).filter(|value| !value.is_empty()) {
        let wanted_key = normalize(wanted);
        // 先按完整相对路径比；再退一步只按文件名比（作者可能只写了文件名）
        let hit = maps
            .iter()
            .find(|map| normalize(&map.path) == wanted_key)
            .or_else(|| {
                maps.iter().find(|map| {
                    normalize(&map.name) == wanted_key
                        || normalize(map.path.rsplit('/').next().unwrap_or("")) == wanted_key
                })
            });

        return match hit {
            Some(map) => MainMapChoice {
                path: Some(map.path.clone()),
                automatic: false,
                warning: None,
            },
            None => MainMapChoice {
                path: None,
                automatic: false,
                warning: Some(format!(
                    "找不到「{wanted}」这张地图，请自己挑一张作为启动入口"
                )),
            },
        };
    }

    // **知名复刻战役：打表认它的入口。**
    //
    // 排在启发式前面 —— 这些战役的入口是固定的，表比猜可靠。
    // 而且表能处理启发式处理不了的：一个知名复刻包里往往还躺着**别的**
    // 战役（SCMR 包里就有 Enslavers Redux 的启动器），用户很容易挑错。
    if let Some((path, remake)) = known::find_launcher(maps) {
        let _ = remake;
        return MainMapChoice {
            path: Some(path),
            automatic: true,
            warning: None,
        };
    }

    // 没声明：只有一张的话直接用它，省用户一次点击
    if maps.len() == 1 {
        return MainMapChoice {
            path: Some(maps[0].path.clone()),
            automatic: true,
            warning: None,
        };
    }

    // 没声明，但包根摆了一张「入口地图」—— 多半就是它
    if let Some(map) = maps.iter().find(|map| looks_like_entry_map(map)) {
        return MainMapChoice {
            path: Some(map.path.clone()),
            automatic: true,
            warning: None,
        };
    }

    MainMapChoice::unset()
}

/// 这张地图看起来是不是整部战役的**入口**。
///
/// 两条都满足才算：**在包根**（路径里没有子目录）**且名字里有入口字样**。
/// 条件收得这么窄，是因为放宽一点点就会把普通关卡认成入口。
fn looks_like_entry_map(map: &MapEntry) -> bool {
    let name = map.name.to_lowercase();
    let at_root = !map.path.contains('/') && !map.path.contains('\\');
    at_root
        && (name.contains("launcher")
            || name.contains("启动")
            || name.contains("入口")
            || name.contains("entry"))
}

/// 一个官方资料片槽位。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignSlot {
    /// 已导入的版本，最新的排在前面。
    #[serde(default)]
    pub variants: Vec<Variant>,
    /// 当前启用的版本 id；`None` 表示原版战役。
    #[serde(default)]
    pub active: Option<String>,
}

/// 库里的一条补丁记录。
///
/// 补丁是**覆盖层**：没有自己的地图目录，只往已有战役上叠文件。
/// 叠加规则见 [`crate::library::compose`]。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Patch {
    /// 库内目录名（补丁的本地唯一键）。
    pub id: String,
    /// 显示名。
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// 包声明的注册 ID。
    #[serde(default)]
    pub registration_id: Option<String>,
    /// 包内声明的默认优先级；数值大的后覆盖。
    #[serde(default = "default_priority")]
    pub priority: i64,
    /// 依赖的战役（注册 ID 或名字）；**为空表示只能手动指定**。
    #[serde(default)]
    pub requires: Vec<String>,
    /// 载荷清单。
    #[serde(default)]
    pub payloads: Vec<Payload>,
    /// 作者声明的覆盖规则。
    #[serde(default)]
    pub overrides: Vec<crate::campaign::metadata::OverrideRule>,
    #[serde(default)]
    pub imported_at: u64,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub mod_count: usize,
}

/// 补丁的默认优先级。
fn default_priority() -> i64 {
    crate::library::patch::DEFAULT_PRIORITY
}

/// 某个战役挂的一个补丁。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// 挂的是哪个补丁。
    pub patch_id: String,
    /// 覆盖包内声明的优先级。
    pub priority: i64,
    /// 是否启用；关掉就不参与合成。
    pub enabled: bool,
}

/// 库索引。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryIndex {
    pub version: u32,
    #[serde(default)]
    pub slots: BTreeMap<String, CampaignSlot>,
    /// 库里的补丁。
    #[serde(default)]
    pub patches: BTreeMap<String, Patch>,
    /// 战役槽位 -> 挂在它身上的补丁。
    #[serde(default)]
    pub bindings: BTreeMap<String, Vec<Binding>>,
}

impl Default for LibraryIndex {
    fn default() -> Self {
        Self {
            version: INDEX_VERSION,
            slots: BTreeMap::new(),
            patches: BTreeMap::new(),
            bindings: BTreeMap::new(),
        }
    }
}

/// 供界面展示的槽位快照。
#[derive(Debug, Clone, Serialize)]
pub struct SlotView {
    pub slug: String,
    pub display_name: String,
    /// 启用时地图落到的官方子目录（相对 `Maps/Campaign`）。
    pub sub_directory: Option<String>,
    pub variants: Vec<Variant>,
    /// 当前启用的版本 id；`None` 表示原版战役。
    pub active: Option<String>,
    /// 当前启用版本的名字，方便界面直接显示。
    pub active_name: Option<String>,
    /// 一句提示（例如官方目录缺失）。
    pub notice: Option<String>,
}

/// 战役库。
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    /// 指定库根目录。默认位置见 [`default_root`]。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 库根目录。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 索引文件。
    pub fn index_path(&self) -> PathBuf {
        self.root.join("library.json")
    }

    /// 各版本的实际存放目录。
    pub fn campaigns_dir(&self) -> PathBuf {
        self.root.join("campaigns")
    }

    /// 补丁目录：`<库根>/patches`。
    pub fn patches_dir(&self) -> PathBuf {
        self.root.join("patches")
    }

    /// 某个补丁的内容目录。
    pub fn patch_dir(&self, id: &str) -> PathBuf {
        self.patches_dir().join(id)
    }

    /// 被挪走的官方文件的暂存目录。
    pub fn backup_dir(&self) -> PathBuf {
        self.root.join("backup")
    }

    /// 某个槽位的存放目录。
    pub fn slot_dir(&self, slug: &str) -> PathBuf {
        self.campaigns_dir().join(slug)
    }

    /// 读取索引。
    ///
    /// 文件不存在时返回空索引 —— 还没有库是很正常的事，不该报错。
    /// 但**读坏了要留证据**：直接当空索引的话，紧接着的一次保存
    /// 就会把整个库顶成 {}，用户看到的是一夜之间什么都没了。
    pub fn index(&self) -> LibraryIndex {
        let path = self.index_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return LibraryIndex::default();
        };
        match serde_json::from_str(&text) {
            Ok(index) => index,
            Err(_) => {
                let _ = std::fs::rename(&path, path.with_extension("json.corrupt"));
                LibraryIndex::default()
            }
        }
    }

    /// 写回索引。
    pub fn save_index(&self, index: &LibraryIndex) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let text = serde_json::to_string_pretty(index)
            .map_err(|error| Error::Parse(format!("索引序列化失败：{error}")))?;
        write_atomic(&self.index_path(), &text)
    }

    /// 按**官方发布顺序**生成槽位快照。
    pub fn slots(&self, installation: Option<&Installation>) -> Vec<SlotView> {
        let index = self.index();

        // 主菜单列五个条目：四大原版战役（进化归虫群之心、序章归虚空之遗）+ 自制战役
        CampaignType::MENU
            .iter()
            .map(|kind| {
                let slug = kind.slug();
                let slot = index.slots.get(slug).cloned().unwrap_or_default();
                let active_name = slot
                    .active
                    .as_ref()
                    .and_then(|id| slot.variants.iter().find(|variant| &variant.id == id))
                    .map(|variant| variant.name.clone());

                let has_content = !slot.variants.is_empty() || slot.active.is_some();
                let notice = if has_content {
                    installation.and_then(|installation| notice_for(installation, kind))
                } else {
                    None
                };

                SlotView {
                    slug: slug.to_string(),
                    display_name: kind.display_name(),
                    sub_directory: kind.sub_directory().map(str::to_string),
                    variants: slot.variants,
                    active: slot.active,
                    active_name,
                    notice,
                }
            })
            .collect()
    }

    /// 列出某个版本里的模组，并标出各自挂没挂载。
    ///
    /// **按文件夹去重**：一个模组可能由十几个 `.SC2Mod` 组成，
    /// 界面上一行就够了（见 `mod_identity`）。
    pub fn variant_mods(&self, slot_slug: &str, variant_id: &str) -> Vec<ModEntry> {
        let Some(variant) = self.variant(slot_slug, variant_id) else {
            return Vec::new();
        };

        // **只有当前启用的版本**才算挂载：没启用的版本一个模组都没铺进游戏，
        // 界面上也不该显示成已挂载 —— 否则用户会以为它在生效（issue 反馈的正是这个）。
        let is_active = self
            .index()
            .slots
            .get(slot_slug)
            .is_some_and(|slot| slot.active.as_deref() == Some(variant_id));
        let mounted_list = if is_active {
            effective_mounted_mods(&variant)
        } else {
            Vec::new()
        };
        group_mods(&variant.payloads, &mounted_list)
    }

    /// **全库的模组汇总**：模组管理菜单用。
    ///
    /// 每一行都带着它属于谁 —— 用户看的是「自由之翼 · 重生 v1.4 的 Alenger」，
    /// 而不是一堆孤零零的文件名。
    pub fn all_mods(&self) -> Vec<LibraryMod> {
        let index = self.index();
        let mut rows = Vec::new();

        for (slug, slot) in &index.slots {
            let slot_name = CampaignType::from_slug(slug)
                .map(|kind| kind.display_name())
                .unwrap_or_else(|| slug.clone());
            let is_custom = CampaignType::from_slug(slug).is_some_and(|kind| kind.is_custom());

            for variant in &slot.variants {
                // 同上：没启用的版本不显示成"挂载"
                let is_active = slot.active.as_deref() == Some(variant.id.as_str());
                let mounted_list = if is_active {
                    effective_mounted_mods(variant)
                } else {
                    Vec::new()
                };
                let declared = &variant.declared_mods;

                for entry in group_mods(&variant.payloads, &mounted_list) {
                    // 有模组记录的话**以记录为准** —— 用户可能改过名字和版本，
                    // 改的是记录（覆盖包内声明），不动包本身。
                    let record = crate::library::mods::find_campaign(
                        self.root(),
                        slug,
                        &variant.id,
                        &entry.path,
                    );

                    rows.push(LibraryMod {
                        slot: slug.clone(),
                        slot_name: slot_name.clone(),
                        variant_id: variant.id.clone(),
                        variant_name: variant.name.clone(),
                        origin: if is_custom {
                            ModOrigin::CustomCampaign
                        } else {
                            ModOrigin::OfficialCampaign
                        },
                        required: declared.iter().any(|item| item == &entry.path),
                        path: entry.path,
                        name: record
                            .as_ref()
                            .map(|item| item.name.clone())
                            .unwrap_or(entry.name),
                        mounted: entry.mounted,
                        parts: entry.parts,
                        standalone_id: record.as_ref().map(|item| item.id.clone()),
                        modid: None,
                        version: record
                            .as_ref()
                            .and_then(|item| item.version.clone())
                            .or_else(|| variant.version.clone()),
                        folder: None,
                        kind: None,
                        mod_record_id: record.map(|item| item.id),
                        source_kind: "campaign".to_string(),
                    });
                }
            }
        }

        // 稳定顺序：先按战役，再按版本，最后按名字
        rows.sort_by(|left, right| {
            left.slot_name
                .cmp(&right.slot_name)
                .then_with(|| left.variant_name.cmp(&right.variant_name))
                .then_with(|| left.name.cmp(&right.name))
        });
        rows
    }

    /// 改某个版本的挂载模组清单。
    ///
    /// 传进来的键会被**过滤成这个版本里真实存在的模组** —— 免得界面上传来一个
    /// 手改的路径，白白在激活时失败。
    pub fn set_mounted_mods(
        &self,
        slot_slug: &str,
        variant_id: &str,
        mods: &[String],
    ) -> Result<Variant> {
        let mut index = self.index();
        let slot = index
            .slots
            .get_mut(slot_slug)
            .ok_or_else(|| Error::CampaignNotFound(slot_slug.to_string()))?;
        let variant = slot
            .variants
            .iter_mut()
            .find(|item| item.id == variant_id)
            .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

        let known: Vec<String> = variant
            .payloads
            .iter()
            .filter_map(|payload| mod_identity(payload).map(|found| found.key))
            .collect();

        // 写 Some 而不是空 Vec：这样「一个都不挂」才是用户的意思，
        // 而不是"没配过"
        variant.mounted_mods = Some(
            mods.iter()
                .filter(|wanted| known.contains(wanted))
                .cloned()
                .collect(),
        );

        let updated = variant.clone();
        self.save_index(&index)?;
        Ok(updated)
    }

    /// 改某个版本的主地图；传 `None` 表示清空。
    pub fn set_main_map(
        &self,
        slot_slug: &str,
        variant_id: &str,
        map: Option<&str>,
    ) -> Result<Variant> {
        let mut index = self.index();
        let slot = index
            .slots
            .get_mut(slot_slug)
            .ok_or_else(|| Error::CampaignNotFound(slot_slug.to_string()))?;
        let variant = slot
            .variants
            .iter_mut()
            .find(|item| item.id == variant_id)
            .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

        variant.main_map = map
            .map(|value| value.replace('\\', "/"))
            .filter(|value| !value.trim().is_empty());

        let updated = variant.clone();
        self.save_index(&index)?;
        Ok(updated)
    }

    /// 取一个版本。
    pub fn variant(&self, slot_slug: &str, variant_id: &str) -> Option<Variant> {
        self.index()
            .slots
            .get(slot_slug)?
            .variants
            .iter()
            .find(|item| item.id == variant_id)
            .cloned()
    }

    /// 取某个版本自带的说明文档信息；没有就是 `None`。
    pub fn variant_doc(&self, slot_slug: &str, variant_id: &str) -> Option<DocInfo> {
        let variant = self.variant(slot_slug, variant_id)?;
        let relative = variant.doc?;
        let path = self.slot_dir(slot_slug).join(variant_id).join(&relative);
        if !path.is_file() {
            return None;
        }
        let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| relative.clone());

        Some(DocInfo {
            path: relative,
            name,
            size,
        })
    }

    /// 读出说明文档的字节；路径越界或文件不在都报错。
    ///
    /// 界面拿它交给 PDF 渲染器 —— 走 IPC 传字节而不是让 WebView 去读文件，
    /// 好处是不用放开文件系统访问，也省掉 asset 协议的配置。
    pub fn doc_bytes(&self, slot_slug: &str, variant_id: &str) -> Result<Vec<u8>> {
        let variant = self
            .variant(slot_slug, variant_id)
            .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;
        let relative = variant
            .doc
            .ok_or_else(|| Error::PackageRejected("这个版本没有自带的说明文档".to_string()))?;

        let root = self.slot_dir(slot_slug).join(variant_id);
        let path = safety::ensure_within(&root, &root.join(relative.replace('\\', "/")))?;
        if !path.is_file() {
            return Err(Error::PackageRejected(
                "说明文档在库里的文件已经不在了".to_string(),
            ));
        }
        Ok(std::fs::read(&path)?)
    }

    /// 某张地图在库里的绝对路径；越界或不存在都报错。
    ///
    /// 编辑器启动要用它 —— 自制战役的地图不进游戏目录，得直接把库里的路径递给编辑器。
    pub fn map_path(&self, slot_slug: &str, variant_id: &str, map: &str) -> Result<PathBuf> {
        let root = self.slot_dir(slot_slug).join(variant_id);
        let path = safety::ensure_within(&root, &root.join(map.replace('\\', "/")))?;
        if !path.is_file() {
            return Err(Error::PackageRejected(format!("找不到地图文件：{map}")));
        }
        Ok(path)
    }

    /// 列出某个版本里的所有地图。
    ///
    /// 按**相对路径的自然顺序**排 —— 章节 `1. Rebel Yell` 排在 `2. Overmind` 前面，
    /// `Terran2` 排在 `Terran10` 前面（普通字符串排序会把 10 排到 2 前面）。
    pub fn variant_maps(&self, slot_slug: &str, variant_id: &str) -> Vec<MapEntry> {
        let root = self.slot_dir(slot_slug).join(variant_id);
        if !root.is_dir() {
            return Vec::new();
        }

        let declared = self
            .index()
            .slots
            .get(slot_slug)
            .and_then(|slot| slot.variants.iter().find(|item| item.id == variant_id))
            .and_then(|variant| variant.main_map.clone());

        let mut maps: Vec<MapEntry> = walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sc2map"))
            })
            .filter_map(|entry| {
                let relative = entry.path().strip_prefix(&root).ok()?;
                // 统一用 / 分隔，跨平台且与包内写法一致
                let path = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                let name = entry
                    .path()
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                let chapter = path.split_once('/').map(|(folder, _)| folder.to_string());
                let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);

                Some(MapEntry {
                    path,
                    name,
                    chapter,
                    size,
                    is_main: false,
                })
            })
            .collect();

        maps.sort_by(|left, right| natural_cmp(&left.path, &right.path));

        if let Some(main) = declared {
            for map in &mut maps {
                if map.path.eq_ignore_ascii_case(&main) {
                    map.is_main = true;
                }
            }
        }

        maps
    }
}

/// 默认的库位置：**可执行文件同级的 data 目录**（绿色版，随软件走）。
pub fn default_root(executable: &Path) -> PathBuf {
    executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("data")
}

/// 数据目录名 —— 放在**星际争霸安装目录的同级**。
///
/// 为什么不在 exe 同级：绿色版可以拷好几份、也可以装在不同地方，各自一份 data 就互不相通。
/// 挂在游戏安装目录旁边之后，同一台机器上的所有启动器实例共用同一份战役库。
pub const DATA_DIR_NAME: &str = "miyin_launcher_data";

/// 数据根目录：安装目录的同级；**找不到安装**（还没配过游戏目录）时退回 exe 同级的 `data/`。
///
/// 退回这一条不能省：全新机器上第一次启动时还没有安装目录，程序得能起来。
pub fn data_root(installation_root: Option<&Path>, executable: &Path) -> PathBuf {
    match installation_root.and_then(Path::parent) {
        Some(parent) => parent.join(DATA_DIR_NAME),
        None => default_root(executable),
    }
}

/// 数据根目录的结论：最后用哪个、有没有搬过东西、给人看的一句话。
#[derive(Debug, Clone)]
pub struct DataRootPlan {
    /// 实际要用的数据根。
    pub root: PathBuf,
    /// 旧的 exe 同级 `data/`（存在才给）。
    pub legacy: Option<PathBuf>,
    /// 这次是不是真的把旧数据搬过来了。
    pub migrated: bool,
    /// 给日志 / 界面的一句话说明。
    pub note: String,
}

/// 一个数据根算不算「完整」—— 有 `library.json` 才算。
///
/// 为什么不能只看目录在不在：跨盘搬迁搬到一半断电，新根会留下**半份**目录。
/// 把它当完整库用，界面上就是「库空了」，而完整的旧数据就在旁边（verifier 实测过）。
fn is_complete(root: &Path) -> bool {
    root.is_dir() && root.join("library.json").is_file()
}

/// 搬迁中转目录：先整个搬到这里，成了再改名过去 —— 改名是原子的。
fn partial_root(root: &Path) -> PathBuf {
    let mut name = root.as_os_str().to_os_string();
    name.push(".partial");
    PathBuf::from(name)
}

/// 给「半途而废」的那份找个不重名的落脚处（`<root>.incomplete`、`-2`…）。
fn parked_root(root: &Path) -> PathBuf {
    for n in 0..1000 {
        let mut name = root.as_os_str().to_os_string();
        name.push(if n == 0 {
            ".incomplete".to_string()
        } else {
            format!(".incomplete-{n}")
        });
        let candidate = PathBuf::from(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    let mut name = root.as_os_str().to_os_string();
    name.push(".incomplete-old");
    PathBuf::from(name)
}

/// 目录里有没有东西（空目录不算"半份数据"）。
fn dir_has_entries(root: &Path) -> bool {
    std::fs::read_dir(root)
        .map(|mut it| it.next().is_some())
        .unwrap_or(false)
}

/// 能不能往这个根里写东西。写不了就别选它 —— 不然整个库都是坏的，
/// 而 `C:/Program Files (x86)/…` 恰好是星际争霸的常见安装位置。
fn is_writable(root: &Path) -> bool {
    if std::fs::create_dir_all(root).is_err() {
        return false;
    }
    let probe = root.join(".miyin-write-probe");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// 决定数据根，并在需要时把旧数据搬过来。
///
/// 三种「新根不作数」的情形都要退回去，不能硬用它：
/// 1. 新根路径上是个**文件**；
/// 2. 新根是**空目录 / 半份目录**（没有 `library.json`）—— 通常是上次跨盘搬迁断了；
/// 3. 新根**写不进去**（`Program Files` 底下）。
///
/// 搬迁本身先落到 `<root>.partial`，完成后再**改名**进来 —— 改名是原子的，
/// 所以中途断电只会留下一个 `.partial`，下一次启动会被认出来并重来一次。
pub fn resolve_data_root(installation_root: Option<&Path>, executable: &Path) -> DataRootPlan {
    let root = data_root(installation_root, executable);
    let legacy = default_root(executable);
    let legacy_exists = legacy.is_dir();

    if root == legacy {
        return DataRootPlan {
            root,
            legacy: None,
            migrated: false,
            note: "数据目录就在程序旁边".to_string(),
        };
    }

    let legacy = legacy_exists.then_some(legacy);

    // 新根已经是完整库 —— 用它（旧数据一个字节都不动）。
    // **这一步必须排在碰 .partial 之前**：多实例同时在跑时，别人正在写的中转目录不该被我们搬走。
    if is_complete(&root) {
        return DataRootPlan {
            root: root.clone(),
            note: if legacy.is_some() {
                format!(
                    "数据目录用 {}（程序旁边还有一份旧的 data/，没动它）",
                    root.display()
                )
            } else {
                format!("数据目录 {}", root.display())
            },
            legacy,
            migrated: false,
        };
    }

    // 上次搬到一半留下的中转目录、或者一份半成品目录，先挪开（**保留**）再重来。
    // 只把「有内容的目录」和「挡路的文件」算半份 —— **空目录不算**（那只是还没导入过东西），
    // 否则每次启动都会攒一个 .incomplete。
    let partial = partial_root(&root);
    let mut leftover: Option<PathBuf> = None;

    // 上次搬到一半留下的中转目录：**先接着把它改名叫上来** —— 里面装的可能是真数据
    // （legacy -> .partial 成功了、.partial -> root 那一步失败时，旧目录已经不在，
    // 数据全在中转目录里；当「半份」停掉就等于把用户的库晾在那儿）。
    if partial.is_dir() && crate::platform::move_path(&partial, &root).is_err() {
        let parked = parked_root(&root);
        if crate::platform::move_path(&partial, &parked).is_ok() {
            leftover = Some(parked);
        }
    }

    // 半成品目录（有内容但没有 library.json）也挪开 —— 空目录不算（那只是还没导入过东西）。
    if root.exists() && (!root.is_dir() || (dir_has_entries(&root) && !is_complete(&root))) {
        let parked = parked_root(&root);
        if crate::platform::move_path(&root, &parked).is_ok() {
            leftover = Some(parked);
        }
    }

    // 该有的兜底：新根写不进去，或者根本没有旧数据可搬
    // 该有的兜底：新根写不进去，或者根本没有旧数据可搬
    let note_prefix = leftover
        .as_ref()
        .map(|parked| format!("上次的搬迁没完成，半份留在 {}；", parked.display()))
        .unwrap_or_default();

    if !is_writable(&root) {
        return DataRootPlan {
            note: match &legacy {
                Some(path) => format!(
                    "{note_prefix}{} 写不进去，这次仍用 {}",
                    root.display(),
                    path.display()
                ),
                None => format!(
                    "{note_prefix}{} 写不进去，暂时没有别的地方可放",
                    root.display()
                ),
            },
            root: legacy.unwrap_or(root),
            legacy: None,
            migrated: false,
        };
    }

    let Some(legacy_path) = legacy.clone() else {
        return DataRootPlan {
            note: format!("{note_prefix}数据目录 {}", root.display()),
            root,
            legacy: None,
            migrated: false,
        };
    };

    // 先搬到 .partial，成了再改名进来
    let partial = partial_root(&root);
    let moved = crate::platform::move_path(&legacy_path, &partial)
        .and_then(|()| crate::platform::move_path(&partial, &root));
    match moved {
        Ok(()) => DataRootPlan {
            note: format!(
                "{note_prefix}已把 {} 迁到 {}",
                legacy_path.display(),
                root.display()
            ),
            root,
            legacy: Some(legacy_path),
            migrated: true,
        },
        Err(error) => DataRootPlan {
            note: format!(
                "{note_prefix}迁移数据目录失败（{error}），这次仍用 {}",
                legacy_path.display()
            ),
            root: legacy_path.clone(),
            legacy: Some(legacy_path),
            migrated: false,
        },
    }
}
/// 由包内声明的资料片推断应该导入到哪个槽位。
///
/// 规则本体在 [`CampaignType::main_slot`]，这里只是给调用方一个好找的入口。
pub fn slot_for(kind: &CampaignType) -> Option<&'static str> {
    kind.main_slot()
}

/// 导入时与库里已有版本的冲突。
#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    /// 冲突的已有版本（库内目录名）。
    pub existing_id: String,
    pub existing_name: String,
    pub existing_version: Option<String>,
    pub incoming_version: Option<String>,
    /// 是否命中同一个**注册 ID**（比同名更强的信号）。
    pub same_id: bool,
    /// 新旧版本对比结论。
    pub relation: VersionRelation,
}

/// 新旧版本的对比结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionRelation {
    /// 新包更新。
    Newer,
    /// 版本号相同。
    Same,
    /// 新包更旧。
    Older,
    /// 版本号缺失或格式认不出来。
    Unknown,
}

impl VersionRelation {
    /// 面向用户的说法。
    pub fn label(self) -> &'static str {
        match self {
            Self::Newer => "更新的版本",
            Self::Same => "相同的版本",
            Self::Older => "更旧的版本",
            Self::Unknown => "无法比较版本",
        }
    }
}

/// 导入时遇到已有同名 / 同 ID 版本的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// **重命名后导入**：保留已有版本，新版本另起一个目录名，两者并存。
    #[default]
    Rename,
    /// **覆盖更新**：用新版本替换已有的同 ID（或同名）版本，目录名与已挂的补丁都不受影响。
    Overwrite,
}

/// 宽松版本比较：把两边的数字段抽出来按数值比。
///
/// 只认得"数字点分"这一大类写法（`1.4.2`、`v0.53`、`141版` 都行）；
/// 认不出来时返回 `None`，由调用方决定怎么提示，而不是瞎猜。
pub fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    fn numbers(text: &str) -> Vec<u64> {
        text.split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect()
    }

    let left_numbers = numbers(left);
    let right_numbers = numbers(right);
    if left_numbers.is_empty() || right_numbers.is_empty() {
        return None;
    }

    for (a, b) in left_numbers.iter().zip(right_numbers.iter()) {
        match a.cmp(b) {
            std::cmp::Ordering::Equal => continue,
            other => return Some(other),
        }
    }

    Some(left_numbers.len().cmp(&right_numbers.len()))
}

/// 检查往某个槽位导入时会不会与已有版本冲突。
///
/// 判定顺序：先按**注册 ID** 找，找不到再按**战役名**找 —— 前者是包作者声明的稳定标识，
/// 后者只是兜底（现实里大量包根本没有 ID）。
pub fn conflict_for(
    library: &Library,
    slot_slug: &str,
    incoming_id: Option<&str>,
    incoming_name: &str,
    incoming_version: Option<&str>,
) -> Option<Conflict> {
    let index = library.index();
    let slot = index.slots.get(slot_slug)?;

    let by_id = incoming_id.and_then(|incoming| {
        slot.variants.iter().find(|variant| {
            variant
                .registration_id
                .as_deref()
                .is_some_and(|existing| existing.eq_ignore_ascii_case(incoming))
        })
    });
    let existing = by_id.or_else(|| {
        slot.variants
            .iter()
            .find(|variant| variant.name.eq_ignore_ascii_case(incoming_name))
    })?;

    let relation = match (incoming_version, existing.version.as_deref()) {
        (Some(incoming), Some(existing)) => compare_versions(incoming, existing)
            .map(|ordering| match ordering {
                std::cmp::Ordering::Greater => VersionRelation::Newer,
                std::cmp::Ordering::Equal => VersionRelation::Same,
                std::cmp::Ordering::Less => VersionRelation::Older,
            })
            .unwrap_or(VersionRelation::Unknown),
        _ => VersionRelation::Unknown,
    };

    Some(Conflict {
        existing_id: existing.id.clone(),
        existing_name: existing.name.clone(),
        existing_version: existing.version.clone(),
        incoming_version: incoming_version.map(str::to_string),
        same_id: by_id.is_some(),
        relation,
    })
}

/// 校验槽位标识，返回对应的资料片。
///
/// **自制战役（`custom`）也是合法槽位** —— 以前这里多加了 `is_main()` 过滤，
/// 结果导入自制战役会直接报「未知的战役槽位：custom」。
/// 落盘位置本来就由 `Placement` 分开管（官方改版进 `Maps/Campaign`，
/// 自制战役进 `Maps/CustomCampaigns`），校验这一层不该再判断它属于哪部原版战役。
pub fn require_slot(slug: &str) -> Result<CampaignType> {
    CampaignType::from_slug(slug)
        .ok_or_else(|| Error::PackageRejected(format!("未知的战役槽位：{slug}")))
}

/// 槽位与当前游戏安装之间的提示。
fn notice_for(installation: &Installation, kind: &CampaignType) -> Option<String> {
    let target = match kind.sub_directory() {
        Some(sub) => installation.campaign_maps_root.join(sub),
        None => installation.campaign_maps_root.clone(),
    };

    if target.is_dir() {
        None
    } else {
        Some(format!(
            "官方目录尚不存在，启用时会自动创建：{}",
            target.display()
        ))
    }
}

/// 生成一个进程内唯一的后缀。
pub(crate) fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

/// 当前 Unix 秒。
pub(crate) fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

/// 自然顺序比较：把连续数字当数值比，而不是按字符比。
///
/// 否则 `Terran10` 会排到 `Terran2` 前面。
fn natural_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let mut a = left.chars().peekable();
    let mut b = right.chars().peekable();

    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.is_ascii_digit() && y.is_ascii_digit() {
                    let mut num_a = String::new();
                    while a.peek().is_some_and(char::is_ascii_digit) {
                        num_a.push(a.next().unwrap_or_default());
                    }
                    let mut num_b = String::new();
                    while b.peek().is_some_and(char::is_ascii_digit) {
                        num_b.push(b.next().unwrap_or_default());
                    }
                    let va: u64 = num_a.parse().unwrap_or(0);
                    let vb: u64 = num_b.parse().unwrap_or(0);
                    match va.cmp(&vb) {
                        std::cmp::Ordering::Equal => {}
                        other => return other,
                    }
                } else {
                    a.next();
                    b.next();
                    match x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase()) {
                        std::cmp::Ordering::Equal => {}
                        other => return other,
                    }
                }
            }
        }
    }
}
