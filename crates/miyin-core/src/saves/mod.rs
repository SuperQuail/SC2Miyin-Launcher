//! 存档管理：游戏真正读的存档目录（`Accounts/<账号>/<地域>/Banks`）的备份与还原（issue #20）。
//!
//! **存档是用户数据**，不是游戏本体 —— 所以它不走游戏目录那套写盘闸门
//! （那个闸门只管安装目录里的文件）。但同样守两条：**动之前先备份**、
//! **先给人看**（[`SaveSet`] 就是给人看的那一份）。
//!
//! **存档隔离**：**默认关着**，用户主动打开才生效；打开之后每个战役用自己的那份
//! Banks，切换战役时自动换档。官方战役的**原版**按槽位一组、**每个改版**各一组
//! （同一部改版的各个版本按注册 ID 同组）；自制战役**每部一组**（键是
//! `custom/<身份>`，见 [`isolation_key`]）。
//! 关着的时候启动器一律不碰 Banks；关掉之后已经存下的组都留着。
//!
//! **「一份存档」不是一个目录**：游戏真正读的是
//! `Documents/StarCraft II/Accounts/<账号>/<地域>/Banks`（本机核实，2026-10），
//! `Documents/StarCraft II/Banks` 通常存在但是空的。所以一组 = **所有账号的 Banks
//! 一起**（见 [`SavesRoots`]），组里按 `Accounts/<账号>/<地域>/Banks/…` 的层级存。
//! 带 `_paths` 的函数直接收 [`SavesRoots`]；不带的那几个是等价的老入口
//! （给的 `banks` 旁边有 `Accounts/` 时自动按全部真实根来）。

mod backup;
mod groups;
mod roots;
mod scan;

#[cfg(test)]
mod tests;

pub use self::backup::{
    backup, backup_paths, backups, group_files, move_save_file, move_save_to_key, restore,
    restore_paths, root, snapshot, snapshot_paths,
};
pub use self::groups::{
    assign, ensure_for_slot, ensure_for_slot_paths, group_label, is_custom_key, isolation,
    isolation_key, profile_for, profile_for_original, reconcile_active_binding, save_active,
    save_active_paths, save_into, save_into_paths, set_isolation, set_isolation_paths, switch,
    switch_paths, targets,
};
pub use self::roots::SavesRoots;
pub use self::scan::scan;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 一份存档快照：有哪些文件、多大、最后改动是什么时候。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SaveSet {
    pub files: Vec<SaveFile>,
    pub bytes: u64,
    /// 目录不存在（还没产生过存档）时为 true —— 界面要按「可能缺失」处理
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveFile {
    /// 相对 `documents` 的路径（`/` 分隔）：多账号时带 `Accounts/<账号>/<地域>/Banks/` 前缀，
    /// 老的单目录模式就是相对那个 `Banks` 的平铺文件名。
    pub name: String,
    pub bytes: u64,
}

/// 自动备份（`restore()` 动手前留的那份）的标签。
///
/// 它**不是**用户意义上的存档组 —— 只是「刚才那份存哪儿了」的安全网。
/// 界面必须把它从组列表里摘出去（用 [`BackupEntry::auto`] 判断），
/// 但仍要能在还原列表里看到它。
pub const AUTO_BACKUP: &str = "还原前";

/// 一份已备份的存档。
#[derive(Debug, Clone, Serialize)]
pub struct BackupEntry {
    /// 备份目录名，还原时拿它指认
    pub name: String,
    /// 备份时带的标签（战役 / 用户自己写的备注）
    pub label: String,
    pub bytes: u64,
    pub files: usize,
    /// **自动生成**的备份（`restore()` 的安全网，标签见 [`AUTO_BACKUP`]）。
    ///
    /// 界面用它把这类备份从「存档组」列表里剔掉 —— 组是用户能选的东西，
    /// 自动备份不是（它跟着每次还原长一份）。
    pub auto: bool,
    /// **没有绑定任何战役/版本**的老组（升级前收下的那份，或者用户手动备份的）。
    ///
    /// 这种组的名字说明不了内容（当年是开隔离那一刻把整份 Banks 收下来的，
    /// 用户可能正在玩某个自制战役）—— 界面别再叫它「原版」，显示成
    /// 「未分类（升级前的存档）」，让用户自己去归位。
    pub legacy: bool,
}

/// 隔离状态。存在 `<data>/saves/state.json`。
///
/// **默认关**（2026-10 产品决定）：版本管理还没做完，不替用户动 Banks；用户在设置页
/// 主动打开之后才按战役分开管，换战役不会把上一部的进度冲掉。关掉只是不再自动切，
/// 已经存下的组一个都不删。
///
/// **没有 state.json 就是关着**（`enabled = false`、`active = None`）。用户打开的那一刻
/// 还没归过档，而任何一次切组 / 存回之前都会先 `save_active` —— 也就是先把游戏里这份
/// 收成「原版」。所以打开隔离不会丢进度。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Isolation {
    pub enabled: bool,
    /// 现在游戏里这份 Banks 属于哪个组。没有组名表示还没归过档。
    pub active: Option<String>,
    /// 组 -> 战役槽位。**反查**得到"这个战役用哪份存档"。
    #[serde(default)]
    pub assignments: BTreeMap<String, String>,
    /// 读状态文件时碰到的问题（人话）。**不落盘**：界面拿到就展示，下一个写盘动作会忽略它。
    ///
    /// 现在只有一种：`state.json` 坏了（[isolation] 不再静默当成「关着」）。
    /// 坏文件在下次写盘前会被改名成 `state.json.corrupt-<时间戳>`，不会悄悄被覆盖掉。
    #[serde(default)]
    pub warning: Option<String>,
}

/// 「原版」组的名字。打开隔离时第一份就是它。
pub const ORIGINAL: &str = "原版";

/// 存档页「归属」下拉 / 战役卡片的一个候选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SaveTarget {
    /// 写进 `state.json` 的隔离键。
    pub key: String,
    /// 界面上的说法（官方：`<战役名>（原版）` / `<战役名> · <版本名>`）。
    pub label: String,
    /// 这个战役 / 版本现在**装在游戏目录里**（按激活清单算，不看槽位选中项）。
    ///
    /// 官方改版最多一个为 true；**自制战役可以有好几个 true**。
    pub enabled: bool,
}

/// 一个存档文件（`*.SC2Save`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SaveEntry {
    /// 账号目录名（`Accounts/<账号>` 那一层，通常是数字 id）。
    pub account: String,
    /// 账号下那一层（`<地域>-<账号>`），同一账号可能有多个。
    pub region: String,
    /// 层级：带上 `Saves` 前缀的相对路径（`Saves` / `Saves/Campaign` / `Saves/Unsaved/Campaign` …）。
    pub folder: String,
    /// 文件名，**不含扩展名**。
    pub name: String,
    /// 绝对路径。
    pub path: String,
    pub bytes: u64,
    /// 修改时间（Unix 秒）。
    pub modified: u64,
}

/// 一个 Banks 文件（`*.SC2Bank`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BankEntry {
    pub account: String,
    pub region: String,
    /// 地图 key：**父目录名**（`Banks/<地图 key>/*.SC2Bank`）；直接在 `Banks` 根下就是空串。
    pub map: String,
    /// 文件名，不含扩展名。
    pub name: String,
    pub path: String,
    pub bytes: u64,
    /// 修改时间（Unix 秒）。
    pub modified: u64,
}

/// 扫出来的全部存档。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SaveInventory {
    /// 账号目录名（数字 id），**按最近活跃排序**。
    pub accounts: Vec<String>,
    /// 所有 `*.SC2Save`，按修改时间降序。
    pub saves: Vec<SaveEntry>,
    /// 所有 `*.SC2Bank`，按修改时间降序。
    pub banks: Vec<BankEntry>,
}
