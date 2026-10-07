//! 已安装战役的扫描与核对。
//!
//! 扫描 `Maps/CustomCampaigns` 下的每个子目录，读取元数据、统计内容，
//! 并给出**核对结论**（哪些能跑、哪些有问题、问题在哪）。
//!
//! # 相比参考实现修正了什么
//!
//! - 遇到非目录条目时 `continue` 而不是 `return`：参考实现下
//!   第一个杂物文件（`desktop.ini`）就会让整个扫描提前结束，列表变空。
//! - 战役库不存在时返回空列表，而不是抛 `ENOENT` 让整个界面报错。
//! - 递归查找元数据，而不是"只看第一个子目录"。
//! - 主动检查启用所需的资料片目录是否存在（参考实现在启用时才 `cp` 失败）。
//! - 元数据按 UTF-8 解码失败时退回 GBK，中文包不再乱码。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::campaign::metadata::{CampaignType, CcmMetadata, StandardMetadata, decode_text};
use crate::campaign::sanitize::sanitize_dir_name;
use crate::campaign::{Campaign, CampaignFormat, HealthIssue};
use crate::safety;
use crate::sc2::Installation;

/// 扫描深度上限：元数据与封面一般不会埋得更深，限制深度可避免在
/// 超大目录树上浪费时间，也避免误把地图内部的资源当成元数据。
const MAX_METADATA_DEPTH: usize = 3;

/// 封面图的候选文件名（不含扩展名）。
const COVER_STEMS: &[&str] = &["cover", "preview", "banner", "poster", "封面"];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// 扫描全部已安装战役。
///
/// `enabled_ids` 是当前已启用的战役标识集合（由激活模块维护）。
/// 本函数**永不失败**：读不动的地方会被跳过，最坏情况返回空列表。
pub fn scan(installation: &Installation, enabled_ids: &HashSet<String>) -> Vec<Campaign> {
    let Ok(root) = safety::resolve(&installation.custom_campaigns_root) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };

    let mut campaigns: Vec<Campaign> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // 必须是 continue：参考实现这里写成 return，一个杂物文件就让列表整个消失
        if !path.is_dir() {
            continue;
        }

        let id = entry.file_name().to_string_lossy().into_owned();
        if is_internal_entry(&id) {
            continue;
        }

        // 单个战役读不动就跳过，不能影响其它战役的展示
        let Ok(mut campaign) = inspect_dir(&path) else {
            continue;
        };
        campaign.enabled = enabled_ids.contains(&campaign.id);
        audit_against_installation(installation, &mut campaign);
        campaign.recompute_health();
        campaigns.push(campaign);
    }

    campaigns.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    campaigns
}

/// 检查单个战役目录（不做与安装环境相关的核对）。
pub fn inspect_dir(dir: &Path) -> crate::error::Result<Campaign> {
    let id = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut issues: Vec<HealthIssue> = Vec::new();

    // 目录名本身是否规范
    if sanitize_dir_name(&id).as_deref() != Some(id.as_str()) {
        issues.push(
            HealthIssue::warning("UNUSUAL_DIR_NAME", format!("战役目录名不规范：{id}"))
                .with_hint("建议重新安装以生成规范目录名，避免后续操作出现路径问题"),
        );
    }

    let (format, name, author, version, description, campaign_raw) =
        read_metadata(dir, &mut issues);

    let campaign_type = CampaignType::parse(&campaign_raw);
    if format != CampaignFormat::Plain && !campaign_type.is_actionable() {
        issues.push(
            HealthIssue::warning(
                "UNKNOWN_CAMPAIGN",
                format!("元数据中的资料片取值无法识别（campaign = \"{campaign_raw}\"）"),
            )
            .with_hint("该战役无法被自动启用，只能手工把地图放进对应目录"),
        );
    }

    let (map_count, mod_count, size_bytes) = measure(dir);

    if map_count == 0 && mod_count == 0 {
        issues.push(
            HealthIssue::broken("NO_CONTENT", "目录内没有 .SC2Map 地图或 .SC2Mod 模组文件")
                .with_hint("可能是安装中断，建议删除后重新安装"),
        );
    }

    let display_name = name.clone().unwrap_or_else(|| id.clone());

    Ok(Campaign {
        id,
        name: display_name,
        author,
        version,
        description,
        cover: find_cover(dir).map(|path| path.to_string_lossy().into_owned()),
        path: dir.to_path_buf(),
        format,
        campaign_type,
        enabled: false,
        health: crate::campaign::HealthLevel::Ok,
        issues,
        map_count: Some(map_count),
        size_bytes: Some(size_bytes),
    })
}

/// 与安装环境相关的核对（需要知道游戏目录布局）。
fn audit_against_installation(installation: &Installation, campaign: &mut Campaign) {
    if !campaign.campaign_type.is_actionable() {
        return;
    }
    let Some(sub) = campaign.campaign_type.sub_directory() else {
        // 自由之翼直接放在 Maps/Campaign 根下，若目录缺失同样要提示
        if !installation.campaign_maps_root.is_dir() {
            campaign.issues.push(
                HealthIssue::warning("MISSING_CAMPAIGN_DIR", "官方战役目录 Maps/Campaign 不存在")
                    .with_hint("启用时会自动创建该目录"),
            );
        }
        return;
    };

    let target = installation.campaign_maps_root.join(sub);
    if !target.is_dir() {
        campaign.issues.push(
            HealthIssue::warning(
                "MISSING_CAMPAIGN_DIR",
                format!("官方战役目录 Maps/Campaign/{sub} 不存在"),
            )
            .with_hint("启用时会自动创建该目录，无需手工处理"),
        );
    }
}

/// 读取元数据，返回 (格式, 名称, 作者, 版本, 描述, 资料片原值)。
fn read_metadata(
    dir: &Path,
    issues: &mut Vec<HealthIssue>,
) -> (
    CampaignFormat,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
) {
    // 1) 包根的 metadata.json 视为标准包
    let standard_path = dir.join("metadata.json");
    if standard_path.is_file() {
        if let Ok(bytes) = std::fs::read(&standard_path) {
            match StandardMetadata::parse(&decode_text(&bytes)) {
                Ok(meta) => {
                    return (
                        CampaignFormat::Standard,
                        clean(meta.name),
                        clean(meta.author),
                        clean(meta.version),
                        clean(meta.description),
                        clean(meta.campaign).unwrap_or_default(),
                    );
                }
                Err(error) => issues.push(HealthIssue::warning(
                    "BAD_METADATA",
                    format!("metadata.json 解析失败：{error}"),
                )),
            }
        }
    }

    // 2) 递归找 metadata.txt（取层级最浅的那个）
    if let Some(path) = find_deepest_first(dir, "metadata.txt") {
        if let Ok(bytes) = std::fs::read(&path) {
            let meta = CcmMetadata::parse(&decode_text(&bytes));
            if !meta.is_empty() {
                return (
                    CampaignFormat::Ccm,
                    clean(meta.title),
                    clean(meta.author),
                    clean(meta.version),
                    clean(meta.description),
                    clean(meta.campaign).unwrap_or_default(),
                );
            }
            issues.push(HealthIssue::warning(
                "EMPTY_METADATA",
                "metadata.txt 存在但没有解析出任何字段",
            ));
        }
    }

    // 3) 兜底：递归找 metadata.json（有些包会把标准元数据埋进子目录）
    if let Some(path) = find_deepest_first(dir, "metadata.json") {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(meta) = StandardMetadata::parse(&decode_text(&bytes)) {
                return (
                    CampaignFormat::Standard,
                    clean(meta.name),
                    clean(meta.author),
                    clean(meta.version),
                    clean(meta.description),
                    clean(meta.campaign).unwrap_or_default(),
                );
            }
        }
    }

    issues.push(
        HealthIssue::warning("NO_METADATA", "目录内没有可用的元数据文件")
            .with_hint("战役名将直接使用目录名；补一个 metadata.txt 可以获得完整信息"),
    );

    (CampaignFormat::Plain, None, None, None, None, String::new())
}

/// 在目录内查找指定文件名，优先返回层级最浅的。
fn find_deepest_first(dir: &Path, file_name: &str) -> Option<PathBuf> {
    let mut best: Option<(usize, PathBuf)> = None;
    for entry in WalkDir::new(dir)
        .max_depth(MAX_METADATA_DEPTH)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if !entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(file_name)
        {
            continue;
        }
        let depth = entry.depth();
        if best
            .as_ref()
            .is_none_or(|(best_depth, _)| depth < *best_depth)
        {
            best = Some((depth, entry.path().to_path_buf()));
        }
    }
    best.map(|(_, path)| path)
}

/// 统计地图数、模组数与总字节数。
fn measure(dir: &Path) -> (usize, usize, u64) {
    let mut maps = 0usize;
    let mut mods = 0usize;
    let mut bytes = 0u64;

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        bytes = bytes.saturating_add(entry.metadata().map(|meta| meta.len()).unwrap_or(0));
        match extension_of(entry.path()).as_str() {
            "sc2map" => maps += 1,
            "sc2mod" => mods += 1,
            _ => {}
        }
    }

    (maps, mods, bytes)
}

/// 找一个封面图。
fn find_cover(dir: &Path) -> Option<PathBuf> {
    let mut fallback: Option<PathBuf> = None;

    for entry in WalkDir::new(dir)
        .max_depth(MAX_METADATA_DEPTH)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !IMAGE_EXTENSIONS.contains(&extension_of(path).as_str()) {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if COVER_STEMS.iter().any(|candidate| stem == *candidate) {
            return Some(path.to_path_buf());
        }
        if fallback.is_none() && entry.depth() == 1 {
            fallback = Some(path.to_path_buf());
        }
    }

    fallback
}

/// 公开的封面查找入口（供应用层把封面读成 data URL）。
pub fn find_cover_for(dir: &Path) -> Option<PathBuf> {
    find_cover(dir)
}

/// 是否为本程序内部的目录（暂存、备份、隐藏目录）。
fn is_internal_entry(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("~$")
}

/// 小写扩展名。
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// 把空字符串与纯空白视为「没有值」。
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sc2::DiscoverySource;
    use std::collections::HashSet;

    const MARKER: &str = "StarCraft II.exe";

    fn setup() -> (tempfile::TempDir, Installation) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join(MARKER), b"stub").expect("marker");
        std::fs::write(
            root.join(".build.info"),
            "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
        )
        .expect("build info");
        let installation = Installation::from_root(root, DiscoverySource::Manual).expect("安装");
        (dir, installation)
    }

    fn write_campaign(
        installation: &Installation,
        dir_name: &str,
        metadata: &str,
        maps: usize,
    ) -> PathBuf {
        let dir = installation.custom_campaigns_root.join(dir_name);
        std::fs::create_dir_all(dir.join("maps")).expect("mkdir");
        std::fs::write(dir.join("metadata.txt"), metadata).expect("metadata");
        for index in 0..maps {
            std::fs::write(dir.join("maps").join(format!("{index:02}.SC2Map")), b"stub")
                .expect("map");
        }
        dir
    }

    #[test]
    fn scans_installed_campaigns() {
        let (_dir, installation) = setup();
        write_campaign(
            &installation,
            "Reborn",
            "title=Reborn\nauthor=Creator\ncampaign=WOL\nversion=1.0\n",
            2,
        );
        write_campaign(&installation, "Swarmy", "title=Swarmy\ncampaign=HOTS\n", 1);

        let campaigns = scan(&installation, &HashSet::new());
        assert_eq!(campaigns.len(), 2);
        let reborn = campaigns.iter().find(|c| c.id == "Reborn").expect("Reborn");
        assert_eq!(reborn.name, "Reborn");
        assert_eq!(reborn.author.as_deref(), Some("Creator"));
        assert_eq!(reborn.map_count, Some(2));
        assert_eq!(reborn.campaign_type, CampaignType::Wol);
        assert_eq!(reborn.format, CampaignFormat::Ccm);
    }

    #[test]
    fn scan_ignores_stray_files_instead_of_aborting() {
        let (_dir, installation) = setup();
        std::fs::create_dir_all(&installation.custom_campaigns_root).expect("root");
        // 参考实现遇到第一个非目录条目就整个返回，列表会变空
        std::fs::write(installation.custom_campaigns_root.join("desktop.ini"), b"x")
            .expect("stray");
        std::fs::write(installation.custom_campaigns_root.join("readme.md"), b"x").expect("stray");
        write_campaign(&installation, "Reborn", "title=Reborn\ncampaign=WOL\n", 1);

        let campaigns = scan(&installation, &HashSet::new());
        assert_eq!(campaigns.len(), 1, "杂物文件不应中断扫描");
    }

    #[test]
    fn scan_returns_empty_when_library_missing() {
        let (_dir, installation) = setup();
        assert!(!installation.custom_campaigns_root.exists());
        assert!(
            scan(&installation, &HashSet::new()).is_empty(),
            "库不存在应返回空列表而不是报错"
        );
    }

    #[test]
    fn finds_metadata_in_nested_subdirectory() {
        let (_dir, installation) = setup();
        let dir = installation.custom_campaigns_root.join("Nested");
        std::fs::create_dir_all(dir.join("inner").join("deep")).expect("mkdir");
        std::fs::write(
            dir.join("inner").join("metadata.txt"),
            "title=深处战役\ncampaign=LOTV\n",
        )
        .expect("meta");
        std::fs::write(dir.join("inner").join("deep").join("a.SC2Map"), b"stub").expect("map");

        let campaigns = scan(&installation, &HashSet::new());
        assert_eq!(campaigns.len(), 1);
        assert_eq!(campaigns[0].name, "深处战役");
        assert_eq!(campaigns[0].campaign_type, CampaignType::Lotv);
    }

    #[test]
    fn flags_campaign_without_content() {
        let (_dir, installation) = setup();
        let dir = installation.custom_campaigns_root.join("Broken");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("metadata.txt"), "title=Broken\ncampaign=WOL\n").expect("meta");

        let campaigns = scan(&installation, &HashSet::new());
        assert_eq!(campaigns[0].health, crate::campaign::HealthLevel::Broken);
        assert!(
            campaigns[0]
                .issues
                .iter()
                .any(|issue| issue.code == "NO_CONTENT")
        );
    }

    #[test]
    fn marks_campaign_as_enabled_from_state() {
        let (_dir, installation) = setup();
        write_campaign(&installation, "Reborn", "title=Reborn\ncampaign=WOL\n", 1);

        let mut enabled = HashSet::new();
        enabled.insert("Reborn".to_string());
        let campaigns = scan(&installation, &enabled);
        assert!(campaigns[0].enabled);
    }
}
