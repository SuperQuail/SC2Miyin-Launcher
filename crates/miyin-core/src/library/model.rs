//! 战役库的数据模型：版本、补丁、槽位、索引与给界面看的行。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::campaign::CampaignFormat;
use crate::campaign::metadata::PackageKind;
use crate::campaign::package::Payload;

use super::slot::CampaignSlot;

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
    /// 用户配置里挂着它（勾选框状态）。
    pub mounted: bool,
    /// 真的铺在 `<游戏>/Mods/` 里（按安装清单算）。
    pub in_game: bool,
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
    /// **用户配置**里挂着它（勾选框的状态），不代表文件已经铺进游戏目录。
    pub mounted: bool,
    /// **真的铺在 `<游戏>/Mods/` 里**（按安装清单算）。
    ///
    /// 版本没启用、或启用后才勾上的模组，这里是 `false` —— 界面要能区分
    /// 「配置挂了」和「已经生效」，不要拿 `mounted` 当这个用。
    pub in_game: bool,
    /// 这个模组由几个文件组成 —— `Alenger` 那种文件夹会有十几个。
    pub parts: usize,
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
