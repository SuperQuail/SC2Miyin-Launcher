//! 补丁的导入、挂载与优先级管理。
//!
//! 补丁是**覆盖层**，不是战役：它没有自己的地图目录，只是往已有战役上叠文件。
//! 合成见 [`crate::library::compose`]。

use std::path::Path;

use serde::Deserialize;

use crate::campaign::metadata::PackageKind;
use crate::campaign::package;
use crate::error::{Error, Result};
use crate::library::{Binding, Library, LibraryIndex, Patch, now_seconds, unique_suffix};
use crate::safety;

/// 补丁的默认优先级；数值大的后覆盖。
pub const DEFAULT_PRIORITY: i64 = 100;

/// 判断一个补丁能不能自动挂到某个战役上。
///
/// - `requires` 为空 = 通用补丁，**只能手动指定**，自动匹配时一律不选
/// - `requires` 非空 = 完全补丁，逐项与战役的注册 ID / 名称比对
pub fn matches_slot(library: &Library, slot_slug: &str, patch: &Patch) -> bool {
    matches_index(&library.index(), slot_slug, patch)
}

/// 同 matches_slot，但直接用手里的索引 —— auto_bind 已经读出来了，
/// 别再为每个 (槽位, 补丁) 组合重读一遍 library.json。
fn matches_index(index: &LibraryIndex, slot_slug: &str, patch: &Patch) -> bool {
    if patch.requires.is_empty() {
        return false;
    }

    let Some(slot) = index.slots.get(slot_slug) else {
        return false;
    };

    patch.requires.iter().any(|requirement| {
        let wanted = requirement.trim();
        slot.variants.iter().any(|variant| {
            variant
                .registration_id
                .as_deref()
                .is_some_and(|id| id.eq_ignore_ascii_case(wanted))
                || variant.name.eq_ignore_ascii_case(wanted)
        })
    })
}

/// 自动挂载：把库里所有**声明了依赖且匹配得上**的补丁挂到对应战役上。
///
/// 已经挂过的不动；`requires` 为空的通用补丁不参与（它们只能手动指定）。
/// 返回新挂上的 `(战役, 补丁名)`。
pub fn auto_bind(library: &Library) -> Result<Vec<(String, String)>> {
    let mut index = library.index();
    let patches: Vec<Patch> = index.patches.values().cloned().collect();
    let slots: Vec<String> = index
        .slots
        .iter()
        .filter(|(_, slot)| {
            slot.variants
                .iter()
                .any(|v| v.registration_id.is_some() || !v.name.is_empty())
        })
        .map(|(slug, _)| slug.clone())
        .collect();

    let mut bound = Vec::new();

    for slot_slug in slots {
        for patch in &patches {
            if !matches_index(&index, &slot_slug, patch) {
                continue;
            }
            let bindings = index.bindings.entry(slot_slug.clone()).or_default();
            if bindings.iter().any(|binding| binding.patch_id == patch.id) {
                continue;
            }
            bindings.push(Binding {
                patch_id: patch.id.clone(),
                priority: patch.priority,
                enabled: true,
            });
            bound.push((slot_slug.clone(), patch.name.clone()));
        }
    }

    if !bound.is_empty() {
        library.save_index(&index)?;
    }

    Ok(bound)
}

/// 把一个补丁包导入库中。
pub fn import_patch(library: &Library, package: &Path) -> Result<Patch> {
    let inspection = package::inspect(package)?;

    if inspection.kind != PackageKind::Patch {
        return Err(Error::PackageRejected(
            "这不是补丁包（包内既有地图又有模组，或声明了 campaign）".to_string(),
        ));
    }
    if inspection.payloads.is_empty() {
        return Err(Error::PackageRejected("补丁包里没有可用的内容".to_string()));
    }

    let patches_dir = library.patches_dir();
    std::fs::create_dir_all(&patches_dir)?;

    let display_name = inspection
        .name
        .clone()
        .or_else(|| inspection.suggested_dir_name.clone())
        .unwrap_or_else(|| "未命名补丁".to_string());

    let mut index = library.index();
    let id = unique_patch_id(&patches_dir, &index, &display_name);

    // 先解到临时目录，成了再原子改名 —— 中途失败不会留下半个补丁
    let staging = safety::ensure_within(
        &patches_dir,
        &patches_dir.join(format!(".tmp-{}", unique_suffix())),
    )?;
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }

    let stats = match package::extract_to(package, &inspection.content_root, &staging) {
        Ok(stats) => stats,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    {
        let target = safety::ensure_within(&patches_dir, &patches_dir.join(&id))?;
        if let Err(error) = std::fs::rename(&staging, &target) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error.into());
        }
    }

    let patch = Patch {
        id: id.clone(),
        name: display_name,
        author: inspection.author.clone(),
        version: inspection.version.clone(),
        description: inspection.description.clone(),
        registration_id: inspection.id.clone(),
        priority: inspection.priority.unwrap_or(DEFAULT_PRIORITY),
        requires: inspection.requires.clone(),
        payloads: inspection.payloads.clone(),
        overrides: inspection.overrides.clone(),
        imported_at: now_seconds(),
        size_bytes: stats.bytes,
        mod_count: stats.mods,
    };

    index.patches.insert(id, patch.clone());
    library.save_index(&index)?;

    Ok(patch)
}

/// 把补丁挂到某个战役上。
pub fn bind(library: &Library, slot_slug: &str, patch_id: &str) -> Result<()> {
    crate::library::require_slot(slot_slug)?;

    let mut index = library.index();
    let patch = index
        .patches
        .get(patch_id)
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(patch_id.to_string()))?;

    let bindings = index.bindings.entry(slot_slug.to_string()).or_default();
    if bindings.iter().any(|binding| binding.patch_id == patch_id) {
        return Ok(());
    }
    bindings.push(Binding {
        patch_id: patch_id.to_string(),
        priority: patch.priority,
        enabled: true,
    });

    library.save_index(&index)
}

/// 解绑（不影响补丁本身，补丁还在库里）。
pub fn unbind(library: &Library, slot_slug: &str, patch_id: &str) -> Result<()> {
    let mut index = library.index();
    if let Some(bindings) = index.bindings.get_mut(slot_slug) {
        bindings.retain(|binding| binding.patch_id != patch_id);
    }
    library.save_index(&index)
}

/// 改挂载设置：启用状态 / 优先级。
pub fn configure_binding(
    library: &Library,
    slot_slug: &str,
    patch_id: &str,
    enabled: Option<bool>,
    priority: Option<i64>,
) -> Result<Binding> {
    let mut index = library.index();
    let bindings = index
        .bindings
        .get_mut(slot_slug)
        .ok_or_else(|| Error::CampaignNotFound(format!("{slot_slug}/{patch_id}")))?;

    let binding = bindings
        .iter_mut()
        .find(|binding| binding.patch_id == patch_id)
        .ok_or_else(|| Error::CampaignNotFound(patch_id.to_string()))?;

    if let Some(enabled) = enabled {
        binding.enabled = enabled;
    }
    if let Some(priority) = priority {
        binding.priority = priority;
    }

    let updated = binding.clone();
    library.save_index(&index)?;
    Ok(updated)
}

/// 允许修改的补丁字段；`None` 表示不动。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchChanges {
    pub name: Option<String>,
    pub author: Option<String>,
    pub registration_id: Option<String>,
    pub description: Option<String>,
    /// 包内声明的默认优先级。只影响**以后**的挂载，
    /// 已经挂到战役上的用挂载记录里的值（那是用户调过的）。
    pub priority: Option<i64>,
}

/// 改一个已导入补丁的元数据。
///
/// 和版本一样：**只改启动器自己记录的元数据，不动包里的原始文件**，
/// 也不动它在各战役上的挂载关系 —— 改名字不会让绑定失效。
pub fn update_patch(library: &Library, patch_id: &str, changes: PatchChanges) -> Result<Patch> {
    let mut index = library.index();
    let patch = index
        .patches
        .get_mut(patch_id)
        .ok_or_else(|| Error::CampaignNotFound(patch_id.to_string()))?;

    if let Some(name) = changes.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(Error::PackageRejected("补丁名不能为空".to_string()));
        }
        patch.name = trimmed.to_string();
    }
    if let Some(author) = changes.author {
        let trimmed = author.trim();
        patch.author = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(id) = changes.registration_id {
        let trimmed = id.trim();
        patch.registration_id = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(description) = changes.description {
        let trimmed = description.trim();
        patch.description = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(priority) = changes.priority {
        patch.priority = priority;
    }

    let updated = patch.clone();
    library.save_index(&index)?;
    Ok(updated)
}

/// 从库里彻底删掉一个补丁（连同它在各战役上的挂载）。
pub fn remove_patch(library: &Library, patch_id: &str) -> Result<()> {
    let mut index = library.index();
    if index.patches.remove(patch_id).is_none() {
        return Err(Error::CampaignNotFound(patch_id.to_string()));
    }
    for bindings in index.bindings.values_mut() {
        bindings.retain(|binding| binding.patch_id != patch_id);
    }
    library.save_index(&index)?;

    let dir = library.patch_dir(patch_id);
    if dir.is_dir() {
        let dir = safety::ensure_within(&library.patches_dir(), &dir)?;
        std::fs::remove_dir_all(dir)?;
    }

    Ok(())
}

/// 在库里给补丁找一个不冲突的目录名。
fn unique_patch_id(dir: &Path, index: &crate::library::LibraryIndex, name: &str) -> String {
    let base = crate::campaign::sanitize::sanitize_dir_name(name)
        .unwrap_or_else(|| format!("patch-{}", unique_suffix()));

    if !index.patches.contains_key(&base) && !dir.join(&base).exists() {
        return base;
    }

    for round in 2..1000 {
        let candidate = format!("{base} #{round}");
        if !index.patches.contains_key(&candidate) && !dir.join(&candidate).exists() {
            return candidate;
        }
    }

    format!("{base}-{}", unique_suffix())
}
