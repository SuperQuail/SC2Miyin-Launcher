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
//! ├── active.json       # 激活清单：我们往游戏目录放了什么、挪走了什么
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
use crate::error::{Error, Result};
use crate::sc2::Installation;

pub mod activation;
pub mod store;

#[cfg(test)]
mod tests;

pub use activation::{ActivationState, activate, deactivate};
pub use store::{import, remove_variant};

/// 索引文件的格式版本，便于以后迁移。
const INDEX_VERSION: u32 = 1;

/// 库内的一个战役版本，即「某个玩家做的某一版」。
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 库索引。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryIndex {
    pub version: u32,
    #[serde(default)]
    pub slots: BTreeMap<String, CampaignSlot>,
}

impl Default for LibraryIndex {
    fn default() -> Self {
        Self {
            version: INDEX_VERSION,
            slots: BTreeMap::new(),
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

    /// 激活清单文件。
    pub fn active_path(&self) -> PathBuf {
        self.root.join("active.json")
    }

    /// 各版本的实际存放目录。
    pub fn campaigns_dir(&self) -> PathBuf {
        self.root.join("campaigns")
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
    /// 文件不存在或损坏时返回空索引而不是报错 —— 索引损坏不该让整个启动器打不开。
    pub fn index(&self) -> LibraryIndex {
        std::fs::read_to_string(self.index_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// 写回索引。
    pub fn save_index(&self, index: &LibraryIndex) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let text = serde_json::to_string_pretty(index)
            .map_err(|error| Error::Parse(format!("索引序列化失败：{error}")))?;
        std::fs::write(self.index_path(), text)?;
        Ok(())
    }

    /// 按**官方发布顺序**生成槽位快照。
    pub fn slots(&self, installation: Option<&Installation>) -> Vec<SlotView> {
        let index = self.index();

        // 主菜单只列四大战役（进化归虫群之心、序章归虚空之遗）
        CampaignType::MAIN
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
}

/// 默认的库位置：**可执行文件同级的 data 目录**（绿色版，随软件走）。
pub fn default_root(executable: &Path) -> PathBuf {
    executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("data")
}

/// 校验槽位标识，返回对应的资料片。
pub fn require_slot(slug: &str) -> Result<CampaignType> {
    CampaignType::from_slug(slug)
        .filter(CampaignType::is_main)
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
