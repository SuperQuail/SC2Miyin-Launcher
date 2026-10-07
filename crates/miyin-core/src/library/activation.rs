//! 启用 / 停用：把库里的某个版本铺进游戏目录，并能精确回滚。
//!
//! # 安全约定（参考实现在这里翻过车）
//!
//! - **只删除我们放进去的文件**：清单之外的任何东西都不碰。
//! - **官方文件先挪走再还原**：不覆盖、不删除官方战役地图。
//! - **绝不对官方目录做递归删除**：全程只做单文件操作。
//!
//! 这样即使中途出错，最坏情况是"多留了几个文件"，而不是"官方战役没了"。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;

use super::compose::{self, Layer};
use super::{Library, require_slot};

/// 一个被我们放进游戏目录的文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedFile {
    pub path: PathBuf,
}

/// 一个被我们挪走以便腾位置的原文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFile {
    /// 它原本所在的位置。
    pub original: PathBuf,
    /// 我们把它挪到了哪里。
    pub backup: PathBuf,
}

/// 激活清单：记录我们动过什么，以便精确回滚。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivationState {
    /// 当前启用的槽位；None 表示没启用任何东西。
    pub slot: Option<String>,
    /// 当前启用的版本 id。
    pub variant: Option<String>,
    /// 我们放进游戏目录的文件。
    pub placed: Vec<PlacedFile>,
    /// 我们挪走的原文件。
    pub backups: Vec<BackupFile>,
}

/// 读取激活清单；文件缺失或损坏都当作「没启用任何东西」。
pub fn load_state(library: &Library) -> ActivationState {
    std::fs::read_to_string(library.active_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// 写回激活清单。
fn save_state(library: &Library, state: &ActivationState) -> Result<()> {
    std::fs::create_dir_all(library.root())?;
    let text = serde_json::to_string_pretty(state)
        .map_err(|error| Error::Parse(format!("激活清单序列化失败：{error}")))?;
    std::fs::write(library.active_path(), text)?;
    Ok(())
}

/// 撤下当前启用的一切：删掉我们放进去的文件，还原被挪走的原文件。
pub fn deactivate(library: &Library, installation: &Installation) -> Result<Vec<String>> {
    let state = load_state(library);
    let mut warnings = Vec::new();

    if state.placed.is_empty() && state.backups.is_empty() {
        save_state(library, &ActivationState::default())?;
        return Ok(warnings);
    }

    for placed in &state.placed {
        match allowed_target(installation, &placed.path) {
            Ok(resolved) => {
                if resolved.is_file() && std::fs::remove_file(&resolved).is_err() {
                    warnings.push(format!("无法删除：{}", resolved.display()));
                }
            }
            Err(_) => warnings.push(format!("跳过越界路径：{}", placed.path.display())),
        }
    }

    for backup in &state.backups {
        if backup.backup.is_file() && !backup.original.exists() {
            if let Some(parent) = backup.original.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::rename(&backup.backup, &backup.original).is_err() {
                warnings.push(format!("无法还原：{}", backup.original.display()));
            }
        }
    }

    save_state(library, &ActivationState::default())?;
    Ok(warnings)
}

pub fn activate(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    variant_id: Option<&str>,
) -> Result<Vec<String>> {
    let kind = require_slot(slot_slug)?;

    // 先撤下上一个启用的版本，再铺新的
    let mut warnings = deactivate(library, installation)?;
    let mut index = library.index();

    let Some(variant_id) = variant_id else {
        if let Some(slot) = index.slots.get_mut(slot_slug) {
            slot.active = None;
        }
        library.save_index(&index)?;
        return Ok(warnings);
    };

    let slot = index.slots.get(slot_slug).cloned().unwrap_or_default();
    let variant = slot
        .variants
        .iter()
        .find(|variant| variant.id == variant_id)
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

    let variant_dir = library.slot_dir(slot_slug).join(&variant.id);
    if !variant_dir.is_dir() {
        return Err(Error::CampaignNotFound(variant.id.clone()));
    }

    // 地图进 Maps/Campaign[/子目录]，模组进 Mods。
    // 目标子目录以**版本自己声明的**为准（进化包 -> swarm/evolution），槽位只作兜底。
    let sub = variant
        .target_sub
        .clone()
        .or_else(|| kind.sub_directory().map(str::to_string));

    // 分层合成：战役本体 + 这个战役上启用的补丁（按优先级叠加）
    let composition = compose::compose(library, slot_slug, &variant, sub.as_deref());
    if composition.files.is_empty() {
        return Err(Error::PackageRejected(
            "该版本里没有可用的地图或模组".to_string(),
        ));
    }

    let mut state = ActivationState {
        slot: Some(slot_slug.to_string()),
        variant: Some(variant.id.clone()),
        placed: Vec::new(),
        backups: Vec::new(),
    };
    let backup_root = library.backup_dir().join(slot_slug);

    for item in &composition.files {
        let relative = item.target.replace('/', std::path::MAIN_SEPARATOR_STR);
        let target = allowed_target(installation, &installation.root.join(&relative))?;

        // 已存在（多半是官方文件，也可能是被更高优先级的层盖住）：先挪进备份区
        if target.exists() {
            let backup = backup_root.join(&relative);
            if let Some(parent) = backup.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if backup.exists() {
                let _ = std::fs::remove_dir_all(&backup);
                let _ = std::fs::remove_file(&backup);
            }
            std::fs::rename(&target, &backup)?;
            state.backups.push(BackupFile {
                original: target.clone(),
                backup,
            });
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        copy_entry(&item.source, &target)?;
        state.placed.push(PlacedFile { path: target });
    }

    // 让用户知道谁盖了谁
    for item in &composition.overridden {
        if matches!(item.layer, Layer::Campaign) {
            continue;
        }
        warnings.push(format!(
            "{} 覆盖了更低优先级的同名内容：{}",
            item.layer.label(),
            item.target
        ));
    }

    save_state(library, &state)?;

    if let Some(slot) = index.slots.get_mut(slot_slug) {
        slot.active = Some(variant.id.clone());
    }
    library.save_index(&index)?;

    Ok(warnings)
}

/// 复制一个载荷：**解开的目录树整体复制**，单文件直接复制。
///
/// 真实包里 `.SC2Map` / `.SC2Mod` 两种形态都有，落盘时必须保持原形态。
fn copy_entry(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        std::fs::copy(source, target)?;
        return Ok(());
    }

    for entry in WalkDir::new(source)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry.path().strip_prefix(source).unwrap_or(entry.path());
        let destination = target.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &destination)?;
        }
    }

    Ok(())
}

/// 校验目标位于官方战役目录或模组目录之内。
fn allowed_target(installation: &Installation, path: &Path) -> Result<PathBuf> {
    safety::ensure_within(&installation.campaign_maps_root, path)
        .or_else(|_| safety::ensure_within(&installation.mods_root, path))
}
