//! 星际枢纽（SCNexus）托管检测与转换的命令层。
//!
//! 只做「找路径 + 转调 core」：扫描 / 转换的规则都在 miyin_core::hub::scnexus 里
//! （依据见该模块头部的 reference/scnexus 文件:行号）。
//!
//! 命中路径的先后：先按参考实现的真实位置找存储目录，再退回「游戏目录 + 默认子目录」。
//! 依据：reference/scnexus/packages/app-main/src/stores/profile.ts:51-56（electron-store 的
//! cwd 是 userData 下的 SCNexusStorage，文件是 store-profile.json）、
//! modules/profile/profile-store.ts:92-103（profile 写入的是绝对路径；
//! 默认值 <GameRoot>/SCNexusCampaign 与 <GameRoot>/Maps/CustomCampaigns 见 stores/profile.ts:32-43）。

use std::path::{Path, PathBuf};

use miyin_core::hub::scnexus::{self, Converted, ScnexusCampaign};
use serde::Deserialize;
use tauri::State;

use crate::AppState;

/// Electron userData 下的固定目录名（参考实现里的 electron-store cwd）。
const STORAGE_DIR: &str = "SCNexusStorage";
/// 用户配置：游戏根目录与战役库根。
const PROFILE_FILE: &str = "store-profile.json";
/// 已安装战役索引（CCM 侧要靠它认「枢纽托管的那几个」）。
const CAMPAIGN_STORE: &str = "store-campaign.json";
/// 当前启用的是哪一部。
const ACTIVE_STORE: &str = "store-campaign-active.json";
/// 默认的战役库目录名（相对游戏根，见 stores/profile.ts:32-43）。
const DEFAULT_LIBRARY: &str = "SCNexusCampaign";

/// 枢纽的路径集合：检出来的，或按游戏目录兜底的。
struct Roots {
    library_root: PathBuf,
    ccm_root: Option<PathBuf>,
    campaign_store: Option<PathBuf>,
    active_store: Option<PathBuf>,
}

/// store-profile.json 里我们用得到的部分（键名照抄参考实现）。
#[derive(Debug, Default, Deserialize)]
struct Profile {
    #[serde(default, rename = "PROFILE_GAME")]
    game: GameProfile,
    #[serde(default, rename = "PROFILE_CAMPAIGN")]
    campaign: CampaignProfile,
}

#[derive(Debug, Default, Deserialize)]
struct GameProfile {
    #[serde(default, rename = "GAME_ROOT")]
    game_root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CampaignProfile {
    #[serde(default, rename = "LIBRARY_ROOT")]
    library_root: Option<String>,
    /// 自制战役（CCM）目录；标准库战役不在这里。
    #[serde(default, rename = "CCM_ROOT")]
    ccm_root: Option<String>,
}

/// 列出星际枢纽里托管的战役（没有枢纽时是空表，不是错误）。
#[tauri::command(async)]
pub(crate) fn list_scnexus_campaigns(state: State<'_, AppState>) -> Vec<ScnexusCampaign> {
    let Some(roots) = discover_roots(&state) else {
        return Vec::new();
    };
    scan(&roots)
}

/// 把星际枢纽里的一部战役转成启动器托管。
///
/// remove_source 来自确认框里那个「转换后删掉枢纽里的副本」勾选（默认不删）。
#[tauri::command(async)]
pub(crate) fn convert_scnexus_campaign(
    id: String,
    remove_source: bool,
    state: State<'_, AppState>,
) -> Result<Converted, String> {
    let roots = discover_roots(&state)
        .ok_or_else(|| "没找到星际枢纽的战役目录 —— 这台机器上可能没装星际枢纽".to_string())?;

    // 按 id 现扫一次：不信任界面手里的旧快照（那边可能已经卸载 / 改名了）
    let campaign = scan(&roots)
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "星际枢纽里已经没有这一版了，刷新一下再试".to_string())?;

    scnexus::convert(&state.library, &campaign, remove_source).map_err(|error| error.to_string())
}

/// 扫一遍（命令行里两处都用得上）。
fn scan(roots: &Roots) -> Vec<ScnexusCampaign> {
    scnexus::scan(
        &roots.library_root,
        roots.ccm_root.as_deref(),
        roots.campaign_store.as_deref(),
        roots.active_store.as_deref(),
    )
}

/// 找枢纽的战役目录；一条线索都没有时返回 None。
fn discover_roots(state: &State<'_, AppState>) -> Option<Roots> {
    let storage = find_storage();
    let profile = storage
        .as_deref()
        .map(|dir| dir.join(PROFILE_FILE))
        .and_then(read_profile);

    // 游戏根优先用启动器自己认的（与枢纽记的可能不同 —— 用户换过安装位置）
    let game_root = state
        .installation
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|installation| installation.root.clone()))
        .or_else(|| {
            profile
                .as_ref()
                .and_then(|profile| profile.game.game_root.as_deref())
                .map(PathBuf::from)
        });

    let library_root = match profile
        .as_ref()
        .and_then(|profile| profile.campaign.library_root.as_deref())
    {
        Some(raw) => resolve(raw, game_root.as_deref()),
        None => game_root.as_ref().map(|root| root.join(DEFAULT_LIBRARY)),
    }?;
    // 库根都没有就不列 —— 空目录扫出来也是空表，没意义
    if !library_root.is_dir() {
        return None;
    }

    let ccm_root = match profile
        .as_ref()
        .and_then(|profile| profile.campaign.ccm_root.as_deref())
    {
        Some(raw) => resolve(raw, game_root.as_deref()),
        None => game_root
            .as_ref()
            .map(|root| root.join("Maps").join("CustomCampaigns")),
    };

    Some(Roots {
        library_root,
        ccm_root,
        campaign_store: storage.as_ref().map(|dir| dir.join(CAMPAIGN_STORE)),
        active_store: storage.as_ref().map(|dir| dir.join(ACTIVE_STORE)),
    })
}

/// 相对路径按游戏根解析；绝对路径原样用。
fn resolve(raw: &str, game_root: Option<&Path>) -> Option<PathBuf> {
    let path = PathBuf::from(raw.trim());
    if path.as_os_str().is_empty() {
        return None;
    }
    if path.is_absolute() {
        return Some(path);
    }
    game_root.map(|root| root.join(path))
}

/// 读 store-profile.json；读不到 / 坏了都当没有。
fn read_profile(path: PathBuf) -> Option<Profile> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// 找 SCNexusStorage 目录。
///
/// 先试 Electron 给 @scnexus/app-main 算出来的固定位置（%APPDATA%\@scnexus\app-main\
/// SCNexusStorage）；没有就在 %APPDATA% 下深度 ≤3 找一遍 —— 便携版 / 换过包名的情况。
fn find_storage() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from)?;

    let known = appdata.join("@scnexus").join("app-main").join(STORAGE_DIR);
    if known.join(PROFILE_FILE).is_file() {
        return Some(known);
    }

    find_storage_below(&appdata, 0)
}

/// 深度受限的目录搜索（深度 0 = 直接在 dir 下面找 SCNexusStorage）。
fn find_storage_below(dir: &Path, depth: usize) -> Option<PathBuf> {
    let storage = dir.join(STORAGE_DIR);
    if storage.join(PROFILE_FILE).is_file() {
        return Some(storage);
    }
    if depth >= 3 {
        return None;
    }

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        if let Some(found) = find_storage_below(&entry.path(), depth + 1) {
            return Some(found);
        }
    }
    None
}
