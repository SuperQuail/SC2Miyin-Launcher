//! 独立模组库：**不属于任何战役**的模组。
//!
//! 为什么单独一块：模组不总是跟着战役包来的。玩家之间经常单独传一个
//! `.SC2Mod` 或者一包模组，装进游戏 `Mods/` 就能用。这类模组也得能
//! 导入、启停、改信息、再导出。
//!
//! 目录结构（与战役库并列）：
//!
//! `@text
//! data/
//! ├── mods.json              # 索引：有哪些模组、各是什么信息、启用了没
//! ├── mods/<id>/             # 各模组的完整内容
//! └── mods-active.json       # 我们在游戏 Mods/ 里放了什么（回滚用）
//! `@
//!
//! **模组按文件夹分**：一个模组目录里可能有十几个 `.SC2Mod`
//! （真实样本里的 `Alenger` 就是），整包进整包出。
//!
//! # 模块划分
//!
//! | 文件 | 职责 |
//! | --- | --- |
//! | `index.rs` | `mods.json` 的读写、扫描与记录增删改 |
//! | `import.rs` | 导入流水线：解开来源 -> 暂存算指纹 -> 定位模组 -> 逐条入库 |
//! | `sync.rs` | 铺盘：把启用中的模组铺进 `<游戏>/Mods/`（含 `placed_name`） |
//! | `export.rs` | 打包导出 |
//! | `compare.rs` | 两个版本的文件 / 语义对比 |
//!
//! 公共 API 一律从这里转发 —— 外部只用 `library::mods::X`，不关心它在哪个文件里。

use serde::{Deserialize, Serialize};

mod compare;
mod export;
mod import;
mod index;
mod sync;

#[cfg(test)]
mod tests;

pub use compare::{ModComparison, ModFileDiff, ModSemanticDiff, compare};
pub use export::{export, export_many};
pub use import::{Staged, StagedMod, import, import_with_password, preview, preview_with_password};
pub use index::{
    effective_id, find_campaign, get, list, remember_campaign, remove, repoint_campaign, update,
};
pub use sync::{placed_name, set_enabled, sync};

/// 一次铺盘的结果。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncReport {
    /// 这次实际铺进去的名字。
    pub placed: Vec<String>,
    /// 冲突提示 —— 谁盖了谁。以前是静默覆盖，出了问题查不出来。
    pub warnings: Vec<String>,
}

/// 独立模组铺进游戏目录时的**形态**。
///
/// 真实样本里两种都有：
///
/// `@text
/// Mods/3疯批帝国之翼.SC2Mod      <- 单个文件
/// Mods/Alenger/                  <- 目录，里面 18 个 .SC2Mod
/// Mods/kit_liberty_story.SC2Mod/ <- 目录（名字带后缀的解开形态）
/// `@
///
/// 形态决定了铺过去是「一个文件」还是「一个目录」—— 弄错了地图一样找不到。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModKind {
    /// 单个 `.SC2Mod` 文件。
    File,
    /// 一个目录（连同里面的所有东西）。
    #[default]
    Folder,
}

/// 一个模组的**内容在哪**。
///
/// 不管内容在哪，它都是一条「模组记录」—— 有自己的名字、版本、作者、modid，
/// 可以编辑、可以导出。区别只在内容的落脚点：
///
/// `@text
/// Library   独立导入的，内容在 data/mods/<id>/
/// Campaign  跟着战役包来的，内容在 data/campaigns/<槽位>/<版本>/ 里
/// `@
///
/// 分开记是为了「战役包带来的模组也能改信息」—— 改动存在记录里（覆盖包内的），
/// 不动包本身。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModSource {
    /// 独立导入，内容就在库里。
    #[default]
    Library,
    /// 跟着某个战役版本来的，内容在那个版本的目录里。
    Campaign {
        slot: String,
        variant: String,
        /// 相对版本目录的路径（就是那个模组文件夹）。
        path: String,
    },
}

/// 库里的一个模组记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneMod {
    /// 目录名，也是本地唯一键。
    pub id: String,
    /// 显示名（可以改）。
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// **modid**：包格式里声明的模组身份。
    ///
    /// 判定「这是同一个模组的不同版本，还是另一个模组」全看它：
    /// id 相同 -> 同一个模组的不同版本；id 不同 -> 新模组。
    /// 没声明时退回用显示名当 id（老记录、随手传的包都没有）。
    #[serde(default)]
    pub modid: Option<String>,
    /// 内容的指纹（SHA-256）。用来判定「完全一样，不用再存一份」。
    #[serde(default)]
    pub fingerprint: String,
    /// **铺进游戏目录时的名字，原样保留**。
    ///
    /// 这一点至关重要：地图里声明的是 `Mods\Alenger钢铁.SC2Mod` 这种路径，
    /// 名字对不上游戏就找不到模组，地图直接打不开。
    /// 所以 `Alenger` 就必须是 `Alenger` —— 不能自作主张变成
    /// `Alenger.SC2Mod`（踩过：以前拿显示名拼了个后缀，把这类模组全废了）。
    #[serde(default)]
    pub folder: String,
    /// 铺成文件还是目录。
    #[serde(default)]
    pub kind: ModKind,
    /// 内容在哪（独立导入的 / 跟着战役包来的）。
    #[serde(default)]
    pub source: ModSource,
    /// 启用了没 —— 启用才会铺进游戏目录。
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub imported_at: u64,
    #[serde(default)]
    pub size_bytes: u64,
    /// 目录里有多少个 `.SC2Mod`。
    #[serde(default)]
    pub parts: usize,
}

/// 导入前的预检：这个包是谁、库里有没有同族的。
///
/// 界面拿它决定要不要问一句 —— 「这看起来是 X 的另一个版本，
/// 作为它的新版本，还是当成一个独立改版？」不确认就直接并进去太武断。
#[derive(Debug, Clone, Serialize)]
pub struct ModPreview {
    /// 包里的显示名。
    pub name: String,
    /// 铺进游戏目录时用的名字（原样保留）。
    pub folder: String,
    /// 这一包里有几个模组 —— 真实样本的 `Mods/` 下有 3 个。
    pub mod_count: usize,
    /// 认出来的 modid（包内声明的，或退回名字）。
    pub modid: String,
    /// 内容指纹。
    pub fingerprint: String,
    /// 库里同 modid 的已有版本。
    pub existing: Vec<StandaloneMod>,
    /// 库里是否已有一份**内容完全一样**的。
    pub duplicate: bool,
    /// 归到哪个版本号（按库里已有的推）。
    pub suggested_version: String,
}

/// 导入时怎么处理与已有模组的关系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModImportMode {
    /// 自动：同 modid 就归并成版本（默认）。
    #[default]
    Auto,
    /// **作为新版本**：归到同一个模组下面，可切换。
    Version,
    /// **作为独立改版**：即使 modid 相同也单独显示一个。
    ///
    /// 用于「基于别人的模组改的」那种 —— 跟原版是两码事，混在一个版本列表里
    /// 反而让人以为它们是同一个东西的不同时期。
    Separate,
}

/// 导入一个模组时发生了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModImportAction {
    /// 全新导进来的。
    Added,
    /// 同一个 modid 的新版本。
    NewVersion,
    /// **内容完全一样** —— 没重复存，直接用库里那份。
    Duplicate,
    /// 版本号撞了但内容不同 —— 自动改了个版本号存下来。
    Renamed,
}

/// 导入模组的结果。
#[derive(Debug, Clone, Serialize)]
pub struct ModImport {
    /// 库里最终的那条记录（可能是已有的那份，也可能是新建的）。
    ///
    /// 一个包装了多个模组时，这里是**第一个**；全部在 `records` 里。
    pub record: StandaloneMod,
    /// 这次涉及的所有模组 —— 一个包的 `Mods/` 下可能有好几个。
    #[serde(default)]
    pub records: Vec<StandaloneMod>,
    pub action: ModImportAction,
    /// 库里原来那个（如果有）。
    pub existing: Option<StandaloneMod>,
    /// 界面该怎么说。
    pub message: String,
}

/// 允许用户修改的模组信息；`None` 表示这一项不动。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModChanges {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

impl ModChanges {
    /// 是不是什么都没改。
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.author.is_none()
            && self.version.is_none()
            && self.description.is_none()
    }
}
