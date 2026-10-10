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
    import_with(library, package, slot_slug, mode, None)
}

/// 用压缩包**替换指定的那一版** —— "更新"走这条路。
///
/// 和 [`import`] 只差一点：目标由调用方点明，**不看包里的名字对不对得上**。
/// 玩家手里的更新包经常被改过标题（"XX 1.2 汉化版"），按名字/id 找根本找不到，
/// 那样"更新"就变成了"再装一份"。
///
/// 沿用被替换那一版的目录名，所以挂在它身上的补丁绑定不受影响。
pub fn replace_from_package(
    library: &Library,
    package: &Path,
    slot_slug: &str,
    target_id: &str,
) -> Result<Variant> {
    import_with(
        library,
        package,
        slot_slug,
        ImportMode::Overwrite,
        Some(target_id),
    )
}

fn import_with(
    library: &Library,
    package: &Path,
    slot_slug: &str,
    mode: ImportMode,
    force_target: Option<&str>,
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

    // 覆盖更新时沿用已有版本的目录名 —— 这样挂在它身上的补丁绑定不受影响。
    // 指定了目标就听调用方的（"更新"用），否则按包里的身份去找。
    let overwrite_target = match force_target {
        Some(id) => existing.variants.iter().find(|item| item.id == id).cloned(),
        None if mode == ImportMode::Overwrite => {
            find_existing(&existing.variants, inspection.id.as_deref(), &display_name).cloned()
        }
        None => None,
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
        // 用预检算好的 —— 它按**载荷**数（解开的目录树也算一个），
        // 而不是 extract_to 那个按文件扩展名数的（目录树里全是 xml，
        // 那样会得到「0 张地图」）。
        map_count: inspection.map_count,
        mod_count: inspection.mod_count,
        size_bytes: stats.bytes,
        main_map: inspection.main_map.clone(),
        // **替换时保住用户勾过的那份清单** —— 更新完不该让他重新挑一遍模组。
        // 没有可继承的（全新导入）才是 None：不写清单 = 还没配过 = 全挂。
        mounted_mods: overwrite_target
            .as_ref()
            .and_then(|old| old.mounted_mods.clone()),
        declared_mods: inspection.declared_mods.clone(),
        doc: resolve_doc(&target, inspection.doc.as_deref()),
        target_sub,
        cover,
        tags: inspection.tags.clone(),
        registration_id: inspection.id.clone(),
        kind: inspection.kind,
        payloads: inspection.payloads.clone(),
        overrides: inspection.overrides.clone(),
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
        // 正在启用：必须先撤下来，否则会把还铺在游戏目录里的文件抽走。
        //
        // **只撤这个槽位**（activate 传 None 表示「这个槽位什么都不启用」）。
        // 早先这里调的是 deactivate，而统一安装引擎之后 deactivate 的含义变成了
        // 「撤掉**所有**战役」—— 那样删 A 战役的一个版本会把 B 战役也一起卸掉。
        //
        // activate 会自己保存索引，所以这里要把本地那份重新读一遍。
        super::activate(library, installation, slot_slug, None)?;
        index = library.index();
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

/// 按**包自己的身份**在库里找它该替换的那一版（主页面「更新战役包…」用）。
///
/// 认不出来返回 None —— 那就是库里还没有这一版，装成新的。
pub fn find_target_for_package(
    library: &Library,
    slot_slug: &str,
    package: &Path,
) -> Result<Option<Variant>> {
    require_slot(slot_slug)?;
    let inspection = package::inspect(package)?;
    let display_name = inspection
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            package
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let index = library.index();
    let existing = index.slots.get(slot_slug).cloned().unwrap_or_default();
    Ok(find_existing(&existing.variants, inspection.id.as_deref(), &display_name).cloned())
}

/// 换版本的结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Replaced {
    pub variant: Variant,
    /// 新包里**多出来**的模组（挂载键）。已经按默认挂上了，界面拿它问一句要不要留。
    pub added_mods: Vec<String>,
}

/// 换掉一个版本：**删旧的、装新的**，再把该跟着走的东西带过去。
///
/// 这就是「更新」的本质 —— 我们没有 diff，原地改文件做不到，
/// 能做的只有「旧的清掉、新的装进来」，同时保证**别的记录不断**。
///
/// 要**带过去**的三样（都记在版本自己身上，删了版本就没了）：
///
/// - 是不是**正启用着** —— 带完再应用回去
/// - **模组挂载清单** —— 用户精挑过的那份
/// - **主地图** —— 用户手选的，包里不一定有
///
/// **不用带的**：存档档案与补丁绑定都是**按槽位**挂的
/// （`saves::assign`、`LibraryIndex.bindings`），换版本天然跟着走；
/// 激活清单由 `remove_variant` 开头那次 `activate(slot, None)` 清干净。
///
/// 失败最多停在「旧的已删、新的没装」—— 所以调用方是**更新**这种用户主动的动作，
/// 不是后台悄悄跑的活。
pub fn replace_variant(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    old_id: &str,
    package: &Path,
) -> Result<Replaced> {
    require_slot(slot_slug)?;

    let index = library.index();
    let slot = index.slots.get(slot_slug).cloned().unwrap_or_default();
    let old = slot
        .variants
        .iter()
        .find(|item| item.id == old_id)
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(old_id.to_string()))?;

    let was_active = slot.active.as_deref() == Some(old_id);
    let keep_mods = old.mounted_mods.clone();
    let keep_main = old.main_map.clone();

    // 旧的先走：会先切回原版（把游戏目录擦干净）、删目录、摘索引
    remove_variant(library, installation, slot_slug, old_id)?;

    // 新的进来 —— 用 Rename，id 由内容重新生成
    let mut variant = import(library, package, slot_slug, ImportMode::Rename)?;

    // 挂载清单**只是一份记录**：谁在名单里、谁就能铺；名单里有但包里没有的，铺的时候自然被忽略
    // （落盘只按包里的载荷走）。所以名单可以放心地"老 ∪ 新"。
    let old_keys = match &keep_mods {
        Some(list) => list.clone(),
        // 老记录没配过 = 全挂：那就拿旧包实际带的那些当"老名单"
        None => super::effective_mounted_mods(&old),
    };
    let new_keys = super::effective_mounted_mods(&variant);
    let added_mods: Vec<String> = new_keys
        .iter()
        .filter(|key| !old_keys.contains(key))
        .cloned()
        .collect();
    let mut merged = old_keys.clone();
    for key in &added_mods {
        if !merged.contains(key) {
            merged.push(key.clone());
        }
    }

    // 把记下的那几样贴回新版本身上（新包自己声明了的就不覆盖）
    let mut index = library.index();
    let mut updated = None;
    if let Some(entry) = index
        .slots
        .get_mut(slot_slug)
        .and_then(|item| item.variants.iter_mut().find(|item| item.id == variant.id))
    {
        entry.mounted_mods = Some(merged.clone());
        if entry.main_map.is_none() {
            entry.main_map = keep_main;
        }
        updated = Some(entry.clone());
    }
    if let Some(entry) = updated {
        variant = entry;
        library.save_index(&index)?;
    }

    // **跟着这一版来的模组记录要跟着改指**：它们只存元数据，内容就在版本目录里，
    // 记录不跟着改就会指向一个已经没了的目录（列表里还在，铺下去却没东西）。
    super::mods::repoint_campaign(library.root(), slot_slug, old_id, &variant.id)?;

    // 原来是应用着的，装完再应用回去
    if was_active {
        super::activate(library, installation, slot_slug, Some(&variant.id))?;
    }

    variant.mounted_mods = Some(merged);
    Ok(Replaced {
        variant,
        added_mods,
    })
}

/// 允许用户修改的元数据字段；`None` 表示这一项不动。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantChanges {
    pub name: Option<String>,
    pub author: Option<String>,
    pub registration_id: Option<String>,
    /// 版本号。填进来的会先过格式校验，不合规直接拒。
    pub version: Option<String>,
    pub description: Option<String>,
    /// 包内**声明为依赖**的模组键；`None` 表示这一项不动。
    ///
    /// 空数组是有效值 —— 表示「作者改主意了，一个都不依赖」。
    #[serde(default)]
    pub declared_mods: Option<Vec<String>>,
}

impl VariantChanges {
    /// 是否什么都没改。
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.author.is_none()
            && self.registration_id.is_none()
            && self.version.is_none()
            && self.description.is_none()
            && self.declared_mods.is_none()
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
    if let Some(version) = changes.version {
        let trimmed = version.trim();
        if trimmed.is_empty() {
            // 空 = 清掉版本号（有些包本来就没写）
            variant.version = None;
        } else {
            // **格式校验放后端** —— 界面那份提示是给人看的，这里才是闸门
            if let Some(problem) = super::naming::version_error(trimmed) {
                return Err(Error::PackageRejected(problem));
            }
            variant.version = Some(super::naming::normalize_version(trimmed));
        }
    }
    if let Some(description) = changes.description {
        let trimmed = description.trim();
        variant.description = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(declared) = changes.declared_mods {
        // 只留这个版本里真实存在的模组 —— 免得界面上传来一个手改的键，
        // 之后导出 / 核对时对不上
        let known: Vec<String> = variant
            .payloads
            .iter()
            .filter_map(|payload| super::mod_identity(payload).map(|found| found.key))
            .collect();

        variant.declared_mods = declared
            .into_iter()
            .filter(|key| known.contains(key))
            .collect();
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

/// 决定版本的说明文档：包自报的优先，其次按文件名特征找。
fn resolve_doc(root: &Path, declared: Option<&str>) -> Option<String> {
    if let Some(declared) = declared
        && let Some(found) = relative_file(root, declared)
    {
        return Some(found);
    }
    find_doc(root)
}

/// 按文件名特征在版本目录里找说明文档。
///
/// 认这些名字（不分大小写）：说明 / readme / manual / doc / 攻略 / guide。
/// 都没有时退回根目录下的第一份 PDF。
fn find_doc(root: &Path) -> Option<String> {
    const STEMS: &[&str] = &["说明", "readme", "manual", "doc", "guide", "攻略"];

    let mut fallback: Option<PathBuf> = None;

    for entry in WalkDir::new(root)
        .max_depth(2)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            continue;
        }

        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if STEMS.iter().any(|wanted| stem.contains(wanted)) {
            return relative_to(root, path);
        }
        if fallback.is_none() {
            fallback = Some(path.to_path_buf());
        }
    }

    fallback.and_then(|path| relative_to(root, &path))
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
pub fn find_cover(root: &Path) -> Option<String> {
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
