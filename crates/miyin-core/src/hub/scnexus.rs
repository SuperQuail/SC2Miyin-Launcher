//! 星际枢纽（SCNexus）托管数据的检测与转换。
//!
//! 「星际枢纽」是另一套 SC2 战役管理器（只读参考：仓库里的 reference/scnexus）。
//! 玩家可能已经在那边装了一堆战役 —— 这里把它们**列出来**，并支持「转为弥音启动器托管」。
//!
//! 两条原则：
//!
//! 1. **只读检测**：[scan] 一个字节都不写，也不动那边的任何文件。
//! 2. **转换复用既有导入**：[convert] 只把内容归一化成我们认识的标准包目录，然后走
//!    library::import —— 不另长一套导入 / 落盘 / 记账逻辑。
//!
//! 依据（「文件:行号」都指 reference/scnexus）：
//!
//! - 存储位置：packages/app-main/src/stores/profile.ts:51-56（SCNexusStorage/store-profile.json）
//! - 战役索引：stores/campaign.ts:26-31（store-campaign.json）、
//!   stores/campaign-active.ts:26-31（store-campaign-active.json）
//! - 标准库布局与扫法：modules/campaign/campaign-manage.ts:20-58
//! - 启用时只铺哪些目录：modules/campaign/campaign-util.ts:37-78
//! - 元数据 schema：packages/standard/src/metadata/metadata.type.ts:5-200
//!
//! # 为什么不复用 campaign::metadata::StandardMetadata
//!
//! SCNexus 的 mods 是**对象数组**（{ name, relative_path, ... }），我们标准格式里是
//! 字符串数组。用我们那份去解析会因类型不匹配整个文件失败（连 name 都读不出来），
//! 所以这里有一份自己的 serde 结构，只取用得上的字段。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::campaign::metadata::{CampaignType, CcmMetadata};
use crate::campaign::package::MIYIN_FORMAT_VERSION;
use crate::error::{Error, Result};
use crate::library::{self, ImportMode, Library, Variant};

/// 内容是从哪种托管目录里发现的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// SCNexus 标准库：<LIBRARY_ROOT>/<目录>/metadata.json
    Library,
    /// SCNexus 托管的自制战役目录（CCM）：<CCM_ROOT>/<目录>/metadata.txt
    Ccm,
}

/// 在 SCNexus 托管目录里发现的一个战役。
#[derive(Debug, Clone, Serialize)]
pub struct ScnexusCampaign {
    /// 稳定 id：元数据里的 snid 优先，缺了就用目录路径的哈希。
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// 资料片归属：wol / hots / lotv / nova（枢纽只认这四种）。
    pub campaign: String,
    pub source: Source,
    /// 战役目录的绝对路径。
    pub path: String,
    /// 是不是枢纽当前启用的那一个。
    pub active: bool,
    pub maps: usize,
    pub mods: usize,
    pub size_bytes: u64,
}

/// 「转为启动器托管」的结果。
#[derive(Debug, Clone, Serialize)]
pub struct Converted {
    /// 归到哪个槽位（wol / hots / lotv / nova）。
    pub slot: String,
    /// 新导入到我们库里的版本。
    pub variant: Variant,
    /// 转换过程中要告诉用户的事（源目录没删成、外部依赖缺失…）。
    pub warnings: Vec<String>,
}

/// SCNexus 的 metadata.json —— 只保留我们用得到的字段。
///
/// 注意 mods 是**对象数组**（见模块头）；这里不用我们的 StandardMetadata。
#[derive(Debug, Clone, Default, Deserialize)]
struct ScnexusMetadata {
    #[serde(default)]
    snid: Option<u64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// 枢纽那边这个键叫 type，值是 Campaign / Customize。
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    campaign: Option<String>,
    /// 启动地图。枢纽把这个键拼成 luancher（原文如此，见 metadata.type.ts:152）。
    #[serde(default)]
    luancher: Option<Launcher>,
    #[serde(default)]
    mods: Vec<ModDeclaration>,
    /// 声明了但没打包进来的依赖（上游模组）。
    #[serde(default)]
    dependencies: Vec<serde_json::Value>,
}

/// 枢纽元数据里的一张地图 / 一个模组的声明。
#[derive(Debug, Clone, Default, Deserialize)]
struct ModDeclaration {
    /// 依赖在上游、包里没有这个文件。
    #[serde(default)]
    upstream: bool,
}

/// 枢纽元数据里的 luancher 字段。
#[derive(Debug, Clone, Default, Deserialize)]
struct Launcher {
    #[serde(default)]
    map_name: Option<String>,
}

/// store-campaign.json：战役索引。
#[derive(Debug, Default, Deserialize)]
struct CampaignStore {
    #[serde(default, rename = "CAMPAIGN_LIST_SET")]
    campaign_list_set: BTreeMap<String, Vec<StoredCampaign>>,
}

/// store-campaign-active.json：当前启用的是哪一部（每个资料片一个）。
#[derive(Debug, Default, Deserialize)]
struct ActiveStore {
    #[serde(default, rename = "CAMPAIGN_SET")]
    campaign_set: BTreeMap<String, Option<StoredCampaign>>,
}

/// store 里的一个战役条目（MetadataInformated：元数据 + 本地信息）。
#[derive(Debug, Default, Deserialize)]
struct StoredCampaign {
    #[serde(default)]
    snid: Option<u64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    campaign: Option<String>,
    #[serde(default)]
    localinfo: LocalInfo,
}

/// store 条目里的本地信息：元数据文件在哪。
#[derive(Debug, Default, Deserialize)]
struct LocalInfo {
    #[serde(default)]
    metadata_path: Option<String>,
}

impl ScnexusMetadata {
    /// type 是不是 Campaign（Customize 是模组，不是战役）。
    fn is_campaign(&self) -> bool {
        self.kind
            .as_deref()
            .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("Campaign"))
    }

    /// 声明了、但没打包进来的依赖有几个。
    fn missing_dependencies(&self) -> usize {
        self.mods.iter().filter(|item| item.upstream).count() + self.dependencies.len()
    }
}

/// 列出 SCNexus 托管的战役。
///
/// 读不到 / 结构不对一律返回空表（**不报错**：机器上没装枢纽是常态）。
///
/// - library_root：枢纽的 PROFILE_CAMPAIGN.LIBRARY_ROOT（默认 <game>/SCNexusCampaign）
/// - ccm_root：枢纽的 PROFILE_CAMPAIGN.CCM_ROOT（默认 <game>/Maps/CustomCampaigns）
/// - campaign_store / active_store：枢纽存储目录里的两份 JSON（找不到就传 None）
pub fn scan(
    library_root: &Path,
    ccm_root: Option<&Path>,
    campaign_store: Option<&Path>,
    active_store: Option<&Path>,
) -> Vec<ScnexusCampaign> {
    let active = active_store.map(active_paths).unwrap_or_default();

    let mut out = scan_library(library_root, &active);
    if let Some(ccm_root) = ccm_root {
        out.extend(scan_ccm(ccm_root, campaign_store, &active));
    }
    out
}

/// 把枢纽里的一部战役转成我们的库版本。
///
/// 做法：归一化到库根下的临时目录（我们标准格式的 metadata.json + 原样内容），
/// 再走 library::import。临时目录无论成败都会收掉。
///
/// remove_source 且这一版在枢纽里**没启用**时才删源目录；删之前还要确认它确实
/// 长在枢纽的标准目录结构里（见 not_removable）—— 删目录不可逆，宁可不删。
pub fn convert(
    library: &Library,
    campaign: &ScnexusCampaign,
    remove_source: bool,
) -> Result<Converted> {
    let source_dir = PathBuf::from(&campaign.path);
    if !source_dir.is_absolute() || !source_dir.is_dir() {
        return Err(Error::PackageRejected(format!(
            "星际枢纽里这一版的目录已经找不到了：{}",
            campaign.path
        )));
    }

    let kind = CampaignType::parse(&campaign.campaign);
    if !kind.is_main() {
        return Err(Error::PackageRejected(format!(
            "认不出这一版归哪部战役（campaign = \"{}\"）",
            campaign.campaign
        )));
    }
    let slot = kind.slug();

    // 目录里的元数据再读一遍：转换要用它的启动地图与外部依赖声明
    let metadata = read_source_metadata(campaign, &source_dir);
    let mut warnings = Vec::new();
    if let Some(metadata) = &metadata {
        let missing = metadata.missing_dependencies();
        if missing > 0 {
            warnings.push(format!(
                "这一版声明了 {missing} 个不在包里的上游依赖，转换只带走了目录里的文件 —— 缺的模组要到「模组」页补上"
            ));
        }
    }

    let variant = stage_and_import(
        library,
        campaign,
        &source_dir,
        metadata.as_ref(),
        slot,
        &mut warnings,
    )?;

    if remove_source {
        if campaign.active {
            warnings.push(
                "这一版在星际枢纽里正启用着，源目录没有删 —— 先去那边停用它，再自己清理"
                    .to_string(),
            );
        } else if let Some(reason) = not_removable(&source_dir, campaign.source) {
            warnings.push(format!("源目录没有删（{reason}）"));
        } else if let Err(error) = std::fs::remove_dir_all(&source_dir) {
            warnings.push(format!("源目录没能删除：{error}"));
        }
    }

    Ok(Converted {
        slot: slot.to_string(),
        variant,
        warnings,
    })
}

/// 归一化 + 导入；临时目录无论如何都收掉。
fn stage_and_import(
    library: &Library,
    campaign: &ScnexusCampaign,
    source_dir: &Path,
    metadata: Option<&ScnexusMetadata>,
    slot: &str,
    warnings: &mut Vec<String>,
) -> Result<Variant> {
    // 暂存放在库根下：和最终落点同一个卷，省一次跨盘大拷贝
    let staging = library
        .root()
        .join(format!(".hub-staging-{}", library::unique_suffix()));

    let result = build_staging(&staging, campaign, source_dir, metadata, warnings)
        .and_then(|()| library::import(library, &staging, slot, ImportMode::Rename));
    let _ = std::fs::remove_dir_all(&staging);
    result
}

/// 把枢纽的内容摆成我们认得的包目录：metadata.json + Maps/** + Mods/**。
fn build_staging(
    staging: &Path,
    campaign: &ScnexusCampaign,
    source_dir: &Path,
    metadata: Option<&ScnexusMetadata>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    std::fs::create_dir_all(staging)?;

    match campaign.source {
        // 标准库：只搬游戏目录那两棵子树 —— 枢纽启用时也只铺这两个（campaign-util.ts:37-78）
        Source::Library => {
            copy_tree(&source_dir.join("Maps"), &staging.join("Maps"))?;
            copy_tree(&source_dir.join("Mods"), &staging.join("Mods"))?;
        }
        // CCM：布局是作者自己定的（常见是扁平的地图 + metadata.txt）—— 整体搬
        Source::Ccm => {
            if copy_content(source_dir, staging, true)? == 0 {
                return Err(Error::PackageRejected(format!(
                    "「{}」的目录里没有能转换的内容（只有说明文件）",
                    campaign.name
                )));
            }
        }
    }

    // 启动地图：能在这份内容里定位到文件才写。认不出来就告诉用户自己挑，
    // 不写一条打不开的路径（resolve_main_map 对不上的路径只会给一句警告）。
    let declared = metadata.and_then(|item| clean(item.luancher.as_ref()?.map_name.clone()));
    let main_map = declared
        .as_deref()
        .and_then(|name| locate_map(staging, name));
    if declared.is_some() && main_map.is_none() {
        warnings.push("没找到它声明的那张启动地图，转换后要自己挑一张作为入口".to_string());
    }

    write_metadata(staging, campaign, metadata, main_map.as_deref())
}

/// 写我们标准格式的 metadata.json —— 导入方（campaign::package）只认这一份。
fn write_metadata(
    staging: &Path,
    campaign: &ScnexusCampaign,
    metadata: Option<&ScnexusMetadata>,
    main_map: Option<&str>,
) -> Result<()> {
    let tags = metadata.map(|item| item.tags.clone()).unwrap_or_default();
    // campaign 写枢纽原本的取值（WOL / HOTS / LOTV / NCO）—— 我们的解析认这些拼写
    let declared = metadata
        .and_then(|item| clean(item.campaign.clone()))
        .unwrap_or_else(|| campaign.campaign.to_uppercase());

    let document = serde_json::json!({
        "type": "Campaign",
        "name": campaign.name.as_str(),
        "description": campaign.description.as_deref(),
        "author": campaign.author.as_deref(),
        "version": campaign.version.as_deref(),
        "campaign": declared.as_str(),
        "miyin": {
            "format": MIYIN_FORMAT_VERSION,
            "id": campaign.id.as_str(),
            "main_map": main_map,
            "tags": tags,
        },
    });

    let text = serde_json::to_string_pretty(&document)?;
    std::fs::write(staging.join("metadata.json"), text)?;
    Ok(())
}

/// 读源目录里的元数据：标准库读 metadata.json，CCM 读 metadata.txt。
fn read_source_metadata(campaign: &ScnexusCampaign, dir: &Path) -> Option<ScnexusMetadata> {
    match campaign.source {
        Source::Library => read_json(&dir.join("metadata.json")),
        Source::Ccm => {
            let text = std::fs::read_to_string(dir.join("metadata.txt")).ok()?;
            let ccm = CcmMetadata::parse(&text);
            Some(ScnexusMetadata {
                campaign: ccm.campaign,
                luancher: ccm.main_map.map(|map_name| Launcher {
                    map_name: Some(map_name),
                }),
                ..ScnexusMetadata::default()
            })
        }
    }
}

/// 扫枢纽的标准战役库：<LIBRARY_ROOT>/<目录>/metadata.json。
fn scan_library(root: &Path, active: &BTreeSet<String>) -> Vec<ScnexusCampaign> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    dirs.sort();

    let mut out = Vec::new();
    for dir in dirs {
        let metadata_path = dir.join("metadata.json");
        let Some(metadata) = read_json::<ScnexusMetadata>(&metadata_path) else {
            continue;
        };
        // 与参考实现的扫法一致：只认 type = Campaign 且资料片是官方那四部
        if !metadata.is_campaign() {
            continue;
        }
        let Some(campaign) = campaign_slug(metadata.campaign.as_deref()) else {
            continue;
        };

        let (size_bytes, maps, mods) = measure(&dir);
        out.push(ScnexusCampaign {
            id: campaign_id(metadata.snid, &dir),
            name: label(metadata.name.as_deref(), &dir),
            author: clean(metadata.author.clone()),
            version: clean(metadata.version.clone()),
            description: clean(metadata.description.clone()),
            campaign: campaign.to_string(),
            source: Source::Library,
            path: dir.to_string_lossy().into_owned(),
            active: active.contains(&path_key(&metadata_path.to_string_lossy())),
            maps,
            mods,
            size_bytes,
        });
    }
    out
}

/// 扫枢纽托管的 CCM 目录：**只认 store-campaign.json 里点名的那几个**。
///
/// 不扫盘猜：CCM 目录里躺着的可能是用户自己放的、别的启动器装的 —— 那种不该
/// 冒充「枢纽托管」。没有 store 就一个都不列。
fn scan_ccm(
    ccm_root: &Path,
    campaign_store: Option<&Path>,
    active: &BTreeSet<String>,
) -> Vec<ScnexusCampaign> {
    let Some(store) = campaign_store.and_then(read_json::<CampaignStore>) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in store.campaign_list_set.values().flatten() {
        let Some(raw) = entry.localinfo.metadata_path.as_deref() else {
            continue;
        };
        let metadata_path = PathBuf::from(raw);
        let Some(dir) = metadata_path.parent() else {
            continue;
        };
        // 只认挂在 CCM 根下的：标准库那批已经由 scan_library 读过一遍
        if !is_under(dir, ccm_root) {
            continue;
        }
        let Some(campaign) = campaign_slug(entry.campaign.as_deref()) else {
            continue;
        };
        if !seen.insert(path_key(&dir.to_string_lossy())) {
            continue;
        }

        let (size_bytes, maps, mods) = measure(dir);
        out.push(ScnexusCampaign {
            id: campaign_id(entry.snid, dir),
            name: label(entry.name.as_deref(), dir),
            author: clean(entry.author.clone()),
            version: clean(entry.version.clone()),
            description: clean(entry.description.clone()),
            campaign: campaign.to_string(),
            source: Source::Ccm,
            path: dir.to_string_lossy().into_owned(),
            active: active.contains(&path_key(raw)),
            maps,
            mods,
            size_bytes,
        });
    }

    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// store-campaign-active.json 里点名的元数据路径（小写、斜杠统一）。
fn active_paths(store: &Path) -> BTreeSet<String> {
    let Some(store) = read_json::<ActiveStore>(store) else {
        return BTreeSet::new();
    };

    store
        .campaign_set
        .into_values()
        .flatten()
        .filter_map(|item| item.localinfo.metadata_path)
        .map(|path| path_key(&path))
        .collect()
}

/// 读一份 JSON；不存在或读坏了都返回 None（检测不该因为一个坏文件整体失败）。
fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// 路径比较用的键：斜杠统一 + 小写（Windows 上 C:\A 与 c:/a 是同一个地方）。
fn path_key(value: &str) -> String {
    value.replace('\\', "/").to_lowercase()
}

/// 去掉首尾空白；空串当「没有值」。
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// 界面上显示的名字：元数据没写就退回目录名。
fn label(name: Option<&str>, dir: &Path) -> String {
    clean(name.map(str::to_string))
        .or_else(|| {
            dir.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "未命名战役".to_string())
}

/// 资料片 -> 我们用的槽位键（wol / hots / lotv / nova）；不是那四部就 None。
fn campaign_slug(raw: Option<&str>) -> Option<&'static str> {
    let kind = CampaignType::parse(raw.unwrap_or_default());
    if kind.is_main() {
        Some(kind.slug())
    } else {
        None
    }
}

/// 稳定 id：snid 优先，缺了就用目录路径的哈希（同一台机器上稳定）。
fn campaign_id(snid: Option<u64>, dir: &Path) -> String {
    match snid {
        Some(value) => value.to_string(),
        None => {
            let digest = Sha256::digest(path_key(&dir.to_string_lossy()).as_bytes());
            let hex = format!("{digest:x}");
            hex[..16].to_string()
        }
    }
}

/// path 是否在 root 之下（忽略大小写与斜杠方向）。
fn is_under(path: &Path, root: &Path) -> bool {
    let path = path_key(&path.to_string_lossy());
    let root = path_key(&root.to_string_lossy());
    let root = root.trim_end_matches('/');
    !root.is_empty() && (path == root || path.starts_with(&format!("{root}/")))
}

/// 目录体积、地图数、模组数。
///
/// 载荷按「路径里第一个 .SC2Map / .SC2Mod 组件」认：解开成目录树的地图也只数一次
/// （与 campaign::package 同一个判据，见 AGENTS §14.5）。
fn measure(root: &Path) -> (u64, usize, usize) {
    let mut bytes = 0u64;
    let mut found: BTreeSet<(String, bool)> = BTreeSet::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_file() {
            bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        }
        if let Ok(relative) = entry.path().strip_prefix(root)
            && let Some(found_root) = payload_root(relative)
        {
            found.insert(found_root);
        }
    }

    let maps = found.iter().filter(|(_, is_mod)| !is_mod).count();
    let mods = found.iter().filter(|(_, is_mod)| *is_mod).count();
    (bytes, maps, mods)
}

/// 相对路径里的载荷根：第一个以 .SC2Map / .SC2Mod 结尾的组件（含它自己）。
fn payload_root(relative: &Path) -> Option<(String, bool)> {
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        let name = component.as_os_str().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        parts.push(name);
        if lower.ends_with(".sc2mod") {
            return Some((parts.join("/"), true));
        }
        if lower.ends_with(".sc2map") {
            return Some((parts.join("/"), false));
        }
    }
    None
}

/// 源目录根上的元数据文件（CCM 侧整体搬内容时要跳过它们）。
const SOURCE_METADATA: [&str; 3] = ["metadata.json", "metadata.txt", "patch.txt"];

/// 把一棵目录树原样复制过去（from 不存在就跳过）。
fn copy_tree(from: &Path, to: &Path) -> Result<usize> {
    copy_content(from, to, false)
}

/// 复制目录内容；skip_root_metadata 时跳过根上的元数据文件。
///
/// 只复制普通文件与目录：符号链接不跟，免得把游戏目录外面的东西搬进来。
fn copy_content(from: &Path, to: &Path, skip_root_metadata: bool) -> Result<usize> {
    if !from.is_dir() {
        return Ok(0);
    }

    let mut copied = 0usize;
    for entry in WalkDir::new(from)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let Ok(relative) = entry.path().strip_prefix(from) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        if skip_root_metadata && is_root_metadata(relative) {
            continue;
        }

        let target = to.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// 这条相对路径是不是根上那一个元数据文件（不是子目录里的同名文件）。
fn is_root_metadata(relative: &Path) -> bool {
    let mut components = relative.components();
    let Some(only) = components.next() else {
        return false;
    };
    if components.next().is_some() {
        return false;
    }
    let name = only.as_os_str().to_string_lossy().to_ascii_lowercase();
    SOURCE_METADATA.contains(&name.as_str())
}

/// 把枢纽的启动地图（luancher.map_name）落到归一化目录里的一张地图上。
///
/// 枢纽那边这个名字**只是文件名**（metadata.type.ts:149-154），所以先按完整相对路径
/// 比、再退回按文件名比；都认不出来就 None（不猜 —— 写一条打不开的路径更糟）。
fn locate_map(root: &Path, wanted: &str) -> Option<String> {
    let wanted_key = path_key(wanted.trim());
    if wanted_key.is_empty() {
        return None;
    }
    let wanted_file = wanted_key
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();

    let mut by_name: Option<String> = None;
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        if !relative.to_ascii_lowercase().ends_with(".sc2map") {
            continue;
        }
        let key = path_key(&relative);
        if key == wanted_key {
            return Some(relative);
        }
        if by_name.is_none()
            && key
                .rsplit('/')
                .next()
                .is_some_and(|name| name == wanted_file)
        {
            by_name = Some(relative);
        }
    }
    by_name
}

/// 这个目录能不能安全地删掉。Some(原因) = 不能删。
///
/// 只删「看起来确实是枢纽自己管的」：名字上必须挂在 SCNexusCampaign（标准库）或
/// CustomCampaigns（它托管的 CCM）之下，而且根上还得有它的元数据文件。
/// 两条都对不上就留着 —— 删目录不可逆，宁可留着让用户自己清。
fn not_removable(dir: &Path, source: Source) -> Option<&'static str> {
    let (metadata_file, expected_parent) = match source {
        Source::Library => ("metadata.json", "scnexuscampaign"),
        Source::Ccm => ("metadata.txt", "customcampaigns"),
    };
    if !dir.join(metadata_file).is_file() {
        return Some("目录里没有它的元数据文件，看着不像枢纽装的那一份");
    }

    let belongs = dir.ancestors().any(|ancestor| {
        ancestor
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_ascii_lowercase() == expected_parent)
    });
    if !belongs {
        return Some("它不在枢纽的标准目录结构里，删掉风险太大");
    }
    None
}

#[cfg(test)]
mod tests;
