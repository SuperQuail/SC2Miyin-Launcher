//! 槽位视图：索引里的槽位状态与给界面看的快照。

use serde::{Deserialize, Serialize};

use super::model::Variant;

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
    /// **装在游戏目录里**的版本 id（按激活清单算）。
    ///
    /// 和 `active` 不是一回事：`active` 只是「最后选中的那个」，这个才是「真的装着」。
    /// **自制战役可以同时装着好几部**，所以这里可能有多条。
    pub enabled_variants: Vec<String>,
    /// 原版是不是装着的那一份（= 这个槽位没有任何版本装着）。
    pub vanilla_enabled: bool,
    /// 一句提示（例如官方目录缺失）。
    pub notice: Option<String>,
}
