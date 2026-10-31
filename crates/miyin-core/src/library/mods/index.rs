//! `mods.json` 索引：读写、扫描，以及模组记录的增删改。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::safety;

use super::{ModChanges, ModKind, ModSource, StandaloneMod};

/// 索引文件。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ModIndex {
    #[serde(default)]
    mods: Vec<StandaloneMod>,
}

/// 独立模组库的根目录（`data/mods`）。
pub(super) fn mods_root(data: &Path) -> PathBuf {
    data.join("mods")
}

/// 索引文件路径。
fn index_path(data: &Path) -> PathBuf {
    data.join("mods.json")
}

/// 读索引；文件不在或读坏了都当空处理，不让界面起不来。
pub fn list(data: &Path) -> Vec<StandaloneMod> {
    let mut rows = std::fs::read_to_string(index_path(data))
        .ok()
        .and_then(|text| serde_json::from_str::<ModIndex>(&text).ok())
        .unwrap_or_default()
        .mods;

    rows.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    rows
}

/// 写索引。
pub(super) fn save(data: &Path, mods: &[StandaloneMod]) -> Result<()> {
    std::fs::create_dir_all(data)?;
    let index = ModIndex {
        mods: mods.to_vec(),
    };
    let text = serde_json::to_string_pretty(&index)?;
    std::fs::write(index_path(data), text)?;
    Ok(())
}

/// 取一个模组。
pub fn get(data: &Path, id: &str) -> Option<StandaloneMod> {
    list(data).into_iter().find(|item| item.id == id)
}

/// 一个目录里有多少个 `.SC2Mod`，以及总体积。
pub(super) fn scan_dir(root: &Path) -> (usize, u64) {
    let mut parts = 0usize;
    let mut bytes = 0u64;

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        // 只有 .SC2Mod 本身算「一个部件」；它解开之后的内部文件不算
        if name.ends_with(".sc2mod") {
            parts += 1;
            bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        }
    }

    // 没有裸 .SC2Mod（可能是单文件形态）时，退回按全局体积算
    if parts == 0 {
        bytes = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| entry.metadata().ok())
            .map(|meta| meta.len())
            .sum();
        parts = 1;
    }

    (parts, bytes)
}

/// 算一份内容的指纹（SHA-256）。
///
/// 拿它判定「完全一样」 —— 比逐字节比对省事，也比比名字可靠：
/// 同一个包换个文件名再传一次是很常见的。
pub(super) fn fingerprint_of(root: &Path) -> Result<String> {
    // 用 sha2 而不是 blake3：它已经在依赖里了，不为一个指纹再加一个 crate
    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }

    // 路径排序后再喂 —— 不然遍历顺序一变指纹就变了
    files.sort();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        // 路径也进哈希：同样的内容摆在不同的子目录里，是两个模组
        hasher.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        hasher.update(&std::fs::read(&path)?);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// 这个记录的身份：优先用声明的 modid，没有就退回名字。
pub fn effective_id(record: &StandaloneMod) -> String {
    record
        .modid
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| record.name.clone())
}

/// 找一条「跟着某个战役版本来的」模组记录。
pub fn find_campaign(data: &Path, slot: &str, variant: &str, path: &str) -> Option<StandaloneMod> {
    list(data).into_iter().find(|item| {
        matches!(
            &item.source,
            ModSource::Campaign {
                slot: item_slot,
                variant: item_variant,
                path: item_path,
            } if item_slot == slot && item_variant == variant && item_path == path
        )
    })
}

/// **记住**一个跟着战役包来的模组，好让它也能改信息、也能单独导出。
///
/// 库里没有就建一条（`ModSource::Campaign`，**只存元数据**，内容仍在战役版本
/// 目录里，不复制一份）；有就原样返回。
///
/// 为什么要这一步：战役包带来的模组本来只能看不能改 ——
/// 名字、版本、作者全是包内写死的。现在它们和独立模组一样是一条**模组记录**，
/// 改的是记录（覆盖包内声明），不动包本身。
#[allow(clippy::too_many_arguments)]
pub fn remember_campaign(
    data: &Path,
    slot: &str,
    variant: &str,
    path: &str,
    folder: &str,
    name: &str,
    version: Option<&str>,
    kind: ModKind,
    parts: usize,
) -> Result<StandaloneMod> {
    if let Some(found) = find_campaign(data, slot, variant, path) {
        return Ok(found);
    }

    // 记录 id 只要唯一、稳定就行 —— 用来源拼一个，不用中文当目录名
    let id = format!("campaign-{slot}-{variant}-{}", short_hash(path));

    let record = StandaloneMod {
        id,
        name: name.to_string(),
        author: None,
        version: version.map(str::to_string),
        description: None,
        modid: Some(name.to_string()),
        fingerprint: String::new(),
        folder: folder.to_string(),
        kind,
        source: ModSource::Campaign {
            slot: slot.to_string(),
            variant: variant.to_string(),
            path: path.to_string(),
        },
        // 战役模组的启停由战役那条线管（mounted_mods），这里不掺和
        enabled: false,
        imported_at: crate::library::now_seconds(),
        size_bytes: 0,
        parts,
    };

    let mut mods = list(data);
    mods.push(record.clone());
    save(data, &mods)?;
    Ok(record)
}

/// 路径的短哈希，用来拼一个稳定的记录 id。
fn short_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string()
}

/// 改模组信息。**只改启动器记录的，不动文件**。
pub fn update(data: &Path, id: &str, changes: ModChanges) -> Result<StandaloneMod> {
    let mut mods = list(data);
    let record = mods
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;

    if let Some(name) = changes.name.filter(|value| !value.trim().is_empty()) {
        record.name = name;
    }
    if changes.author.is_some() {
        record.author = changes.author;
    }
    if changes.version.is_some() {
        record.version = changes.version;
    }
    if changes.description.is_some() {
        record.description = changes.description;
    }

    let updated = record.clone();
    save(data, &mods)?;
    Ok(updated)
}

/// 删掉一个模组。
pub fn remove(data: &Path, id: &str) -> Result<()> {
    let root = mods_root(data);
    let dir = safety::ensure_within(&root, &root.join(id))?;
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }

    let mut mods = list(data);
    mods.retain(|item| item.id != id);
    save(data, &mods)?;
    Ok(())
}

/// 换版本时，把指向旧版本的补丁绑定改指到新版本（不这么做补丁会跟着丢）。
pub fn repoint_campaign(data: &Path, slot: &str, from: &str, to: &str) -> Result<usize> {
    let mut rows = list(data);
    let mut moved = 0;
    for row in rows.iter_mut() {
        if let ModSource::Campaign {
            slot: row_slot,
            variant,
            ..
        } = &mut row.source
            && row_slot == slot
            && variant == from
        {
            *variant = to.to_string();
            moved += 1;
        }
    }
    if moved > 0 {
        save(data, &rows)?;
    }
    Ok(moved)
}
