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

/// 启用某个版本；variant_id 传 None 表示切回**原版战役**。
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

    let slot_dir = library.slot_dir(slot_slug);
    let variant_dir = safety::ensure_within(&slot_dir, &slot_dir.join(&variant.id))?;
    if !variant_dir.is_dir() {
        return Err(Error::CampaignNotFound(variant.id.clone()));
    }

    // 地图进 Maps/Campaign[/子目录]，模组进 Mods
    // 目标子目录以**版本自己声明的**为准（进化包 -> swarm/evolution），
    // 槽位只作为兜底。
    let sub = variant
        .target_sub
        .as_deref()
        .or_else(|| kind.sub_directory());

    let maps_target = match sub {
        Some(sub) => installation.campaign_maps_root.join(sub),
        None => installation.campaign_maps_root.clone(),
    };
    let mods_target = installation.mods_root.clone();

    // 参考实现就是漏了这一步：Maps/Campaign/swarm 这类目录在全新安装里并不存在
    std::fs::create_dir_all(&maps_target)?;
    std::fs::create_dir_all(&mods_target)?;

    let payload = collect_payload(&variant_dir);
    if payload.is_empty() {
        return Err(Error::PackageRejected(
            "该版本里没有可用的地图或模组文件".to_string(),
        ));
    }

    let mut state = ActivationState {
        slot: Some(slot_slug.to_string()),
        variant: Some(variant.id.clone()),
        placed: Vec::new(),
        backups: Vec::new(),
    };
    let mut seen: Vec<String> = Vec::new();

    for (source, is_map) in payload {
        let Some(file_name) = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };

        // 不同子目录里的同名文件会互相覆盖：明确提示，而不是悄悄丢一个
        let key = format!("{}:{}", is_map, file_name.to_lowercase());
        if seen.contains(&key) {
            warnings.push(format!("同名文件被跳过：{file_name}"));
            continue;
        }
        seen.push(key);

        let target = if is_map {
            maps_target.join(&file_name)
        } else {
            mods_target.join(&file_name)
        };
        let target = allowed_target(installation, &target)?;

        // 已存在（多半是官方文件）：先挪到备份区，切回原版时原样还原
        if target.exists() {
            let backup = library.backup_dir().join(slot_slug).join(&file_name);
            if let Some(parent) = backup.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(&target, &backup)?;
            state.backups.push(BackupFile {
                original: target.clone(),
                backup,
            });
        }

        std::fs::copy(&source, &target)?;
        state.placed.push(PlacedFile {
            path: target.clone(),
        });
    }

    save_state(library, &state)?;

    if let Some(slot) = index.slots.get_mut(slot_slug) {
        slot.active = Some(variant.id.clone());
    }
    library.save_index(&index)?;

    Ok(warnings)
}

/// 校验目标位于官方战役目录或模组目录之内。
fn allowed_target(installation: &Installation, path: &Path) -> Result<PathBuf> {
    safety::ensure_within(&installation.campaign_maps_root, path)
        .or_else(|_| safety::ensure_within(&installation.mods_root, path))
}

/// 收集版本目录里需要铺进游戏的文件：.SC2Map 与 .SC2Mod。
fn collect_payload(dir: &Path) -> Vec<(PathBuf, bool)> {
    let mut payload: Vec<(PathBuf, bool)> = WalkDir::new(dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            match extension_of(&path).as_str() {
                "sc2map" => Some((path, true)),
                "sc2mod" => Some((path, false)),
                _ => None,
            }
        })
        .collect();

    payload.sort();
    payload
}

/// 小写扩展名。
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}
