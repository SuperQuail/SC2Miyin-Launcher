//! 往库里导入版本、从库里删除版本。

use std::path::{Path, PathBuf};

use crate::campaign::package::{self, MAX_ENTRIES, MAX_UNPACKED_BYTES};
use crate::campaign::sanitize::{sanitize_dir_name, with_suffix};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;
use walkdir::WalkDir;

use super::{ImportMode, Library, Variant, now_seconds, require_slot, unique_suffix};

/// 把一个战役包导入到指定槽位，返回新建的版本记录。
///
/// 同一个战役可以导入多个版本：目录名重复时会自动加序号，
/// 因此「自由之翼：重生 v1.4」与「v1.5」可以并存，随时切换。
pub fn import(
    library: &Library,
    package: &Path,
    slot_slug: &str,
    mode: ImportMode,
) -> Result<Variant> {
    let slot_kind = require_slot(slot_slug)?;

    let inspection = package::inspect(package)?;
    if inspection.has_blocking_issue() {
        let reason = inspection
            .issues
            .iter()
            .find(|issue| issue.level == crate::campaign::HealthLevel::Broken)
            .map(|issue| issue.message.clone())
            .unwrap_or_else(|| "未知原因".to_string());
        return Err(Error::PackageRejected(reason));
    }

    // 显示名优先用元数据里的标题，退回压缩包文件名
    let display_name = inspection
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            package
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "未命名战役".to_string());

    let slot_dir = library.slot_dir(slot_slug);
    std::fs::create_dir_all(&slot_dir)?;

    let mut index = library.index();
    let existing = index.slots.get(slot_slug).cloned().unwrap_or_default();

    // 覆盖更新时沿用已有版本的目录名 —— 这样挂在它身上的补丁绑定不受影响
    let overwrite_target = if mode == ImportMode::Overwrite {
        find_existing(&existing.variants, inspection.id.as_deref(), &display_name).cloned()
    } else {
        None
    };

    let id = match &overwrite_target {
        Some(variant) => variant.id.clone(),
        None => unique_variant_id(&slot_dir, &existing.variants, &display_name)?,
    };

    let target = safety::ensure_within(&slot_dir, &slot_dir.join(&id))?;

    // 先解压到暂存目录，成功后再整体改名 —— 中途失败不会留下半个版本
    let staging = safety::ensure_within(
        &slot_dir,
        &slot_dir.join(format!(".staging-{}", unique_suffix())),
    )?;
    std::fs::create_dir_all(&staging)?;

    let stats = match package::extract_to(package, &inspection.content_root, &staging) {
        Ok(stats) => stats,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    if stats.files == 0 {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::PackageRejected("压缩包内容为空".to_string()));
    }
    if stats.files > MAX_ENTRIES || stats.bytes > MAX_UNPACKED_BYTES {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::PackageRejected(
            "解压后超出体积或条目上限".to_string(),
        ));
    }

    // 覆盖已有版本时先把旧的挪到一边，切换成功后再删 ——
    // 中途失败还能把旧的改回来，不会两头落空
    let mut replaced: Option<PathBuf> = None;
    if target.exists() {
        let parked = safety::ensure_within(
            &slot_dir,
            &slot_dir.join(format!(".replaced-{}", unique_suffix())),
        )?;
        std::fs::rename(&target, &parked)?;
        replaced = Some(parked);
    }

    if let Err(error) = std::fs::rename(&staging, &target) {
        if let Some(parked) = &replaced {
            let _ = std::fs::rename(parked, &target);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error.into());
    }

    if let Some(parked) = replaced {
        let _ = std::fs::remove_dir_all(parked);
    }

    // 启用时地图该落到哪个子目录：以包内声明的资料片为准（进化包 -> swarm/evolution），
    // 声明与所选槽位对不上时退回槽位自己的子目录。
    let declared = inspection.campaign_type.clone();
    let target_sub = if declared.parent() == slot_kind {
        declared.sub_directory().map(str::to_string)
    } else {
        slot_kind.sub_directory().map(str::to_string)
    };

    // 封面：包自报的优先，其次按文件名特征找，都没有就用官方美术
    let cover = resolve_cover(&target, inspection.cover.as_deref());

    let variant = Variant {
        id: id.clone(),
        name: display_name,
        author: inspection.author.clone(),
        version: inspection.version.clone(),
        description: inspection.description.clone(),
        format: inspection.format,
        imported_at: now_seconds(),
        source: package
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        map_count: stats.maps,
        mod_count: stats.mods,
        size_bytes: stats.bytes,
        target_sub,
        cover,
        tags: inspection.tags.clone(),
        registration_id: inspection.id.clone(),
        kind: inspection.kind,
    };

    // 最新的排在最前面；覆盖更新时先摘掉旧记录
    let slot = index.slots.entry(slot_slug.to_string()).or_default();
    if let Some(replaced) = &overwrite_target {
        slot.variants.retain(|variant| variant.id != replaced.id);
    }
    slot.variants.insert(0, variant.clone());
    library.save_index(&index)?;

    Ok(variant)
}

/// 删除库中的一个版本。若它正在启用，会先切回原版战役。
pub fn remove_variant(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    variant_id: &str,
) -> Result<()> {
    require_slot(slot_slug)?;

    let mut index = library.index();
    let slot = index.slots.get(slot_slug).cloned().unwrap_or_default();

    if !slot.variants.iter().any(|variant| variant.id == variant_id) {
        return Err(Error::CampaignNotFound(variant_id.to_string()));
    }

    if slot.active.as_deref() == Some(variant_id) {
        // 正在启用：必须先撤下来，否则会把还铺在游戏目录里的文件抽走
        super::deactivate(library, installation)?;
        if let Some(slot) = index.slots.get_mut(slot_slug) {
            slot.active = None;
        }
    }

    let slot_dir = library.slot_dir(slot_slug);
    let dir = safety::ensure_within(&slot_dir, &slot_dir.join(variant_id))?;
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }

    if let Some(slot) = index.slots.get_mut(slot_slug) {
        slot.variants.retain(|variant| variant.id != variant_id);
    }
    library.save_index(&index)?;

    Ok(())
}

/// 允许用户修改的元数据字段；`None` 表示这一项不动。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantChanges {
    pub name: Option<String>,
    pub author: Option<String>,
    pub registration_id: Option<String>,
    pub description: Option<String>,
}

impl VariantChanges {
    /// 是否什么都没改。
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.author.is_none()
            && self.registration_id.is_none()
            && self.description.is_none()
    }
}

/// 修改一个已导入版本的元数据。
///
/// **只改启动器自己记录的元数据，不动包里的原始文件** ——
/// 这样编辑随时可以改回来，也不会破坏包内容，更不影响 CCM 读取。
///
/// 注意：改名**不会**改版本目录（目录名是补丁绑定的锚点），只改显示名。
pub fn update_variant(
    library: &Library,
    slot_slug: &str,
    variant_id: &str,
    changes: VariantChanges,
) -> Result<Variant> {
    require_slot(slot_slug)?;

    let mut index = library.index();
    let slot = index
        .slots
        .get_mut(slot_slug)
        .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

    let variant = slot
        .variants
        .iter_mut()
        .find(|variant| variant.id == variant_id)
        .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

    if let Some(name) = changes.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(Error::PackageRejected("战役名不能为空".to_string()));
        }
        variant.name = trimmed.to_string();
    }
    if let Some(author) = changes.author {
        let trimmed = author.trim();
        // 空字符串表示"清空作者"，读的时候会显示成未知作者
        variant.author = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(id) = changes.registration_id {
        let trimmed = id.trim();
        variant.registration_id = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(description) = changes.description {
        let trimmed = description.trim();
        variant.description = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }

    let updated = variant.clone();
    library.save_index(&index)?;

    Ok(updated)
}

/// 找出要覆盖的已有版本：先按注册 ID，再按名字。
fn find_existing<'a>(
    variants: &'a [Variant],
    incoming_id: Option<&str>,
    incoming_name: &str,
) -> Option<&'a Variant> {
    let by_id = incoming_id.and_then(|incoming| {
        variants.iter().find(|variant| {
            variant
                .registration_id
                .as_deref()
                .is_some_and(|existing| existing.eq_ignore_ascii_case(incoming))
        })
    });

    by_id.or_else(|| {
        variants
            .iter()
            .find(|variant| variant.name.eq_ignore_ascii_case(incoming_name))
    })
}

/// 在槽位内生成唯一的版本目录名。
fn unique_variant_id(root: &Path, existing: &[Variant], name: &str) -> Result<String> {
    let base = sanitize_dir_name(name)
        .ok_or_else(|| Error::PackageRejected(format!("无法把「{name}」用作版本目录名")))?;

    if is_free(root, existing, &base) {
        return Ok(base);
    }

    for counter in 2..1000u32 {
        let candidate = with_suffix(&base, &counter.to_string())
            .ok_or_else(|| Error::PackageRejected("无法生成版本目录名".to_string()))?;
        if is_free(root, existing, &candidate) {
            return Ok(candidate);
        }
    }

    Err(Error::PackageRejected(
        "同名版本过多，请先清理一些版本".to_string(),
    ))
}

/// 决定版本的封面图：包自报的优先，其次按文件名特征找，都没有就返回 `None`。
fn resolve_cover(root: &Path, declared: Option<&str>) -> Option<String> {
    if let Some(declared) = declared
        && let Some(found) = relative_file(root, declared)
    {
        return Some(found);
    }
    find_cover(root)
}

/// 把包内声明的相对路径解析成版本目录内的相对路径；越界或不存在都返回 `None`。
fn relative_file(root: &Path, declared: &str) -> Option<String> {
    let candidate = root.join(declared.replace('\\', "/"));
    let resolved = safety::ensure_within(root, &candidate).ok()?;
    if !resolved.is_file() {
        return None;
    }
    relative_to(root, &resolved)
}

/// 按文件名特征在版本目录里找封面图。
///
/// 认这些名字（不分大小写、可带任意常见图片扩展名）：cover / preview / banner /
/// poster / 封面。都没有时退回根目录下的第一张图片。
fn find_cover(root: &Path) -> Option<String> {
    const STEMS: &[&str] = &["cover", "preview", "banner", "poster", "封面"];
    const EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

    let mut fallback: Option<PathBuf> = None;

    for entry in WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !EXTS.contains(&extension_of(path).as_str()) {
            continue;
        }

        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if STEMS.contains(&stem.as_str()) {
            return relative_to(root, path);
        }
        if fallback.is_none() && entry.depth() == 1 {
            fallback = Some(path.to_path_buf());
        }
    }

    fallback.and_then(|path| relative_to(root, &path))
}

/// 取相对 root 的路径，统一用 `/` 分隔。
fn relative_to(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

/// 小写扩展名。
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// 目录名是否未被占用。
fn is_free(root: &Path, existing: &[Variant], id: &str) -> bool {
    !existing.iter().any(|variant| variant.id == id) && !root.join(id).exists()
}
