//! 把库里的版本导出成**CCM 也能直接读**的战役包。
//!
//! 目录形状（就是真实 CCM 包 `黄金之遗(CCM).zip` 的样子）：
//!
//! `@text
//! <战役名>.zip
//! ├── metadata.txt          <- CCM 读这个（键=值）
//! ├── paiur01.SC2Map        <- 平铺的地图
//! ├── HTXL.SC2Mod           <- 平铺的模组
//! └── Miyin/                <- 我们的额外数据，CCM 不读也不冲突
//!     ├── metadata.json
//!     ├── cover.png
//!     └── patches/<id>/…    <- 附加模式下的补丁原件
//! `@
//!
//! 关键点：**我们的数据全部塞在 `Miyin/` 目录里**，所以同一份包既能被 CCM 读，
//! 也能被弥音启动器读回（还能把补丁一起带走）。

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::campaign::package::PayloadTarget;
use crate::error::{Error, Result};
use crate::library::compose::{self, Layer};
use crate::library::{Library, Patch, Variant, now_seconds};

/// 我们自己的数据放这个目录，避免和 CCM 抢位置。
pub const MIYIN_DIR: &str = "Miyin";

/// 导出选项。
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// 目标 zip 路径。
    pub destination: PathBuf,
    /// 把启用中的补丁**合并进顶层**（导出物就是"打完补丁的样子"）。
    ///
    /// 无论开不开，补丁原件都会放进 `Miyin/patches/`，导入方可以继续单独开关。
    pub merge_patches: bool,
}

/// 导出结果，界面用来告诉用户"都带了什么"。
#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub path: PathBuf,
    /// 顶层写进去的文件数。
    pub files: usize,
    pub maps: usize,
    pub mods: usize,
    /// **目录树形态**的载荷名 —— CCM 大概率读不了，界面要提示。
    pub expanded: Vec<String>,
    /// 一起带走的补丁名。
    pub patches: Vec<String>,
}

/// 把某个版本导出成 CCM 兼容包。
pub fn export(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    options: &ExportOptions,
) -> Result<ExportReport> {
    let kind = crate::library::require_slot(slot_slug)?;
    let sub = variant
        .target_sub
        .clone()
        .or_else(|| kind.sub_directory().map(str::to_string));

    let composition = compose::compose(library, slot_slug, variant, sub.as_deref());
    if composition.files.is_empty() {
        return Err(Error::PackageRejected(
            "这个版本里没有可导出的内容".to_string(),
        ));
    }

    // 附加模式：顶层只放战役本体，补丁原件丢进 Miyin/patches/
    // 合成模式：顶层就是合成结果
    let top: Vec<&compose::ComposedFile> = composition
        .files
        .iter()
        .filter(|file| options.merge_patches || matches!(file.layer, Layer::Campaign))
        .collect();

    if top.is_empty() {
        return Err(Error::PackageRejected(
            "这个版本本体里没有内容，只有补丁".to_string(),
        ));
    }

    if let Some(parent) = options.destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&options.destination)?;
    let mut zip = zip::ZipWriter::new(file);
    let zip_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut report = ExportReport {
        path: options.destination.clone(),
        files: 0,
        maps: 0,
        mods: 0,
        expanded: Vec::new(),
        patches: composition
            .patch_layers()
            .into_iter()
            .filter_map(|layer| match layer {
                Layer::Patch { name, .. } => Some(name),
                Layer::Campaign => None,
            })
            .collect(),
    };

    // ---- 顶层：CCM 读得懂的那部分 ----
    write_text(
        &mut zip,
        "metadata.txt",
        &ccm_metadata(variant),
        zip_options,
    )?;

    let mut used: Vec<String> = vec!["metadata.txt".to_string()];
    for item in &top {
        let raw = item.target.rsplit('/').next().unwrap_or(&item.target);
        let name = crate::campaign::sanitize::sanitize_dir_name(raw)
            .ok_or_else(|| Error::PackageRejected(format!("文件名不可用：{}", item.target)))?;
        let name = unique_name(&mut used, &name);

        if item.expanded {
            // 解开的目录树：CCM 多半读不了，但内容不能丢
            report.expanded.push(name.clone());
        }

        if name.to_ascii_lowercase().ends_with(".sc2mod") {
            report.mods += 1;
        } else {
            report.maps += 1;
        }

        add_path(
            &mut zip,
            &item.source,
            &name,
            zip_options,
            &mut report.files,
        )?;
    }

    // ---- Miyin/：我们自己的数据 ----
    write_text(
        &mut zip,
        &format!("{MIYIN_DIR}/metadata.json"),
        &miyin_metadata(slot_slug, variant),
        zip_options,
    )?;
    report.files += 1;

    if let Some(cover) = &variant.cover {
        let source = library.slot_dir(slot_slug).join(&variant.id).join(cover);
        if source.is_file() {
            let name = source
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "cover.png".to_string());
            add_path(
                &mut zip,
                &source,
                &format!("{MIYIN_DIR}/{name}"),
                zip_options,
                &mut report.files,
            )?;
        }
    }

    write_text(
        &mut zip,
        &format!("{MIYIN_DIR}/bindings.json"),
        &bindings_json(library, slot_slug),
        zip_options,
    )?;
    report.files += 1;

    // 补丁原件：两种模式都带上，导入方可以继续单独开关
    for (_, patch) in compose::bindings_of(library, slot_slug) {
        let root = library.patch_dir(&patch.id);
        if !root.is_dir() {
            continue;
        }
        let prefix = format!("{MIYIN_DIR}/patches/{}", patch.id);
        add_tree(&mut zip, &root, &prefix, zip_options, &mut report.files)?;
    }

    zip.finish()
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;

    Ok(report)
}

/// CCM 读的那份元数据（键=值）。
fn ccm_metadata(variant: &Variant) -> String {
    let mut out = String::new();
    out.push_str(&format!("title={}\n", one_line(&variant.name)));
    if let Some(description) = &variant.description {
        out.push_str(&format!("desc={}\n", one_line(description)));
    }
    if let Some(author) = &variant.author {
        out.push_str(&format!("author={}\n", one_line(author)));
    }
    if let Some(campaign) = variant.target_sub.as_deref().and_then(sub_to_campaign) {
        out.push_str(&format!("campaign={campaign}\n"));
    }
    if let Some(version) = &variant.version {
        out.push_str(&format!("version={}\n", one_line(version)));
    }
    if let Some(id) = &variant.registration_id {
        out.push_str(&format!("id={}\n", one_line(id)));
    }
    if !variant.tags.is_empty() {
        out.push_str(&format!("tags={}\n", one_line(&variant.tags.join(", "))));
    }
    out
}

/// 我们自己的那份（JSON，含弥音扩展）。
fn miyin_metadata(slot_slug: &str, variant: &Variant) -> String {
    let payloads: Vec<serde_json::Value> = variant
        .payloads
        .iter()
        .map(|payload| {
            serde_json::json!({
                "source": payload.source,
                "expanded": payload.expanded,
                "target": match &payload.target {
                    PayloadTarget::Mirror { path } => serde_json::json!({ "kind": "mirror", "path": path }),
                    PayloadTarget::Mod { name } => serde_json::json!({ "kind": "mod", "name": name }),
                    PayloadTarget::Map { name } => serde_json::json!({ "kind": "map", "name": name }),
                },
            })
        })
        .collect();

    let document = serde_json::json!({
        "name": variant.name,
        "description": variant.description,
        "author": variant.author,
        "version": variant.version,
        "campaign": variant.target_sub,
        "slot": slot_slug,
        "exported_at": now_seconds(),
        "exported_by": "MiYin Launcher",
        "miyin": {
            "format": crate::campaign::package::MIYIN_FORMAT_VERSION,
            "id": variant.registration_id,
            "tags": variant.tags,
        },
        "payloads": payloads,
        "note": "本目录是弥音启动器的附加数据。CCM 及其他工具可以完全忽略它；启动器读回时会用它还原版本、注册 ID 与载荷落点。",
    });

    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string())
}

/// 补丁挂载关系。
fn bindings_json(library: &Library, slot_slug: &str) -> String {
    let list: Vec<serde_json::Value> = compose::bindings_of(library, slot_slug)
        .into_iter()
        .map(|(binding, patch)| {
            serde_json::json!({
                "id": patch.id,
                "name": patch.name,
                "author": patch.author,
                "version": patch.version,
                "priority": binding.priority,
                "enabled": binding.enabled,
                "requires": patch.requires,
            })
        })
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({ "slot": slot_slug, "patches": list }))
        .unwrap_or_else(|_| "{}".to_string())
}

/// 目标子目录 -> 包内的 campaign 取值。
fn sub_to_campaign(sub: &str) -> Option<&'static str> {
    match sub {
        "swarm" => Some("HOTS"),
        "swarm/evolution" => Some("HOTSEVO"),
        "void" => Some("LOTV"),
        "voidprologue" => Some("LOTVPROLOGUE"),
        "nova" => Some("NCO"),
        _ => None,
    }
}

/// 元数据里的值不能带换行，否则会破坏 `键=值` 结构。
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ").trim().to_string()
}

/// 同名去重：真实包里两张地图撞名的可能性不为零。
fn unique_name(used: &mut Vec<String>, name: &str) -> String {
    if !used.iter().any(|item| item.eq_ignore_ascii_case(name)) {
        used.push(name.to_string());
        return name.to_string();
    }

    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem.to_string(), format!(".{extension}")),
        None => (name.to_string(), String::new()),
    };

    for round in 2..1000 {
        let candidate = format!("{stem} ({round}){extension}");
        if !used
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&candidate))
        {
            used.push(candidate.clone());
            return candidate;
        }
    }

    let fallback = format!("{stem}-{}{extension}", now_seconds());
    used.push(fallback.clone());
    fallback
}

/// 写一个文本条目。
fn write_text<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    name: &str,
    content: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    zip.start_file(name, options)
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

/// 把一个文件或目录树写进 zip。
fn add_path<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    source: &Path,
    name: &str,
    options: zip::write::SimpleFileOptions,
    files: &mut usize,
) -> Result<()> {
    if source.is_dir() {
        return add_tree(zip, source, name, options, files);
    }

    zip.start_file(name, options)
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
    let mut input = std::fs::File::open(source)?;
    std::io::copy(&mut input, zip)?;
    *files += 1;
    Ok(())
}

/// 把一棵目录树写进 zip（保持内部结构）。
fn add_tree<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    root: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
    files: &mut usize,
) -> Result<()> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        let name = format!("{prefix}/{}", relative.to_string_lossy().replace('\\', "/"));

        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), options)
                .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
            continue;
        }

        zip.start_file(&name, options)
            .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
        let mut input = std::fs::File::open(entry.path())?;
        std::io::copy(&mut input, zip)?;
        *files += 1;
    }

    Ok(())
}

/// 单独导出一个补丁。
///
/// 形态贴着现实里的补丁包来：**内容平铺在根目录**（`X.SC2Mod` / `x.SC2Map`），
/// 加一个 `patch.txt` 声明自己是谁、依赖什么，我们的数据照旧放 `Miyin/`。
///
/// 平铺是有讲究的：重新导入时裸的 `.SC2Mod` 会被识别成"落到 `Mods/` 下"，
/// 裸的 `.SC2Map` 会被识别成"落到绑定战役的目录下" —— 这样补丁才能挂到任意战役上。
/// 如果写成 `Mods/X.SC2Mod` 就会被当成镜像路径写死，绑到别的战役就摆错了。
pub fn export_patch(library: &Library, patch: &Patch, destination: &Path) -> Result<ExportReport> {
    let root = library.patch_dir(&patch.id);
    if !root.is_dir() {
        return Err(Error::CampaignNotFound(patch.id.clone()));
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(destination)?;
    let mut zip = zip::ZipWriter::new(file);
    let zip_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut report = ExportReport {
        path: destination.to_path_buf(),
        files: 0,
        maps: 0,
        mods: 0,
        expanded: Vec::new(),
        patches: vec![patch.name.clone()],
    };

    write_text(&mut zip, "patch.txt", &patch_metadata(patch), zip_options)?;

    let mut used: Vec<String> = vec!["patch.txt".to_string()];
    for payload in &patch.payloads {
        let raw = payload.source.rsplit('/').next().unwrap_or(&payload.source);
        let name = crate::campaign::sanitize::sanitize_dir_name(raw)
            .ok_or_else(|| Error::PackageRejected(format!("文件名不可用：{}", payload.source)))?;
        let name = unique_name(&mut used, &name);

        let source = root.join(payload.source.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !source.exists() {
            continue;
        }
        if payload.is_mod {
            report.mods += 1;
        } else {
            report.maps += 1;
        }
        if payload.expanded {
            report.expanded.push(name.clone());
        }

        add_path(&mut zip, &source, &name, zip_options, &mut report.files)?;
    }

    let document = serde_json::json!({
        "name": patch.name,
        "description": patch.description,
        "author": patch.author,
        "version": patch.version,
        "kind": "patch",
        "requires": patch.requires,
        "priority": patch.priority,
        "exported_at": now_seconds(),
        "exported_by": "MiYin Launcher",
        "miyin": {
            "format": crate::campaign::package::MIYIN_FORMAT_VERSION,
            "id": patch.registration_id,
            "kind": "patch",
        },
        "note": "本目录是弥音启动器的附加数据。CCM 及其他工具可以完全忽略它。",
    });
    write_text(
        &mut zip,
        &format!("{MIYIN_DIR}/metadata.json"),
        &serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string()),
        zip_options,
    )?;
    report.files += 1;

    zip.finish()
        .map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;

    Ok(report)
}

/// 补丁的 `patch.txt`（CCM 风格键=值）。
fn patch_metadata(patch: &Patch) -> String {
    let mut out = String::new();
    out.push_str(&format!("name={}\n", patch.name));
    if let Some(description) = &patch.description {
        out.push_str(&format!("desc={}\n", one_line(description)));
    }
    if let Some(author) = &patch.author {
        out.push_str(&format!("author={}\n", one_line(author)));
    }
    if let Some(version) = &patch.version {
        out.push_str(&format!("version={}\n", one_line(version)));
    }
    if let Some(id) = &patch.registration_id {
        out.push_str(&format!("id={}\n", one_line(id)));
    }
    if !patch.requires.is_empty() {
        out.push_str(&format!("requires={}\n", patch.requires.join(", ")));
    }
    out.push_str(&format!("priority={}\n", patch.priority));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ccm_metadata_is_key_value_and_single_line() {
        let variant = Variant {
            id: "a".to_string(),
            name: "黄金之遗".to_string(),
            author: Some("HTXL".to_string()),
            version: Some("1.32".to_string()),
            description: Some("第一行\n第二行".to_string()),
            target_sub: Some("void".to_string()),
            registration_id: Some("HTXL.golden".to_string()),
            tags: vec!["重制".to_string()],
            ..Variant::default()
        };

        let text = ccm_metadata(&variant);
        assert!(text.contains("title=黄金之遗"));
        assert!(text.contains("author=HTXL"));
        assert!(text.contains("campaign=LOTV"));
        assert!(text.contains("id=HTXL.golden"));
        assert!(text.contains("tags=重制"));
        // 值里不能有换行，否则会破坏 键=值 结构
        assert!(text.contains("desc=第一行 第二行"));
    }

    #[test]
    fn sub_directories_map_back_to_campaign_values() {
        assert_eq!(sub_to_campaign("void"), Some("LOTV"));
        assert_eq!(sub_to_campaign("swarm/evolution"), Some("HOTSEVO"));
        assert_eq!(sub_to_campaign("voidprologue"), Some("LOTVPROLOGUE"));
    }

    #[test]
    fn duplicate_names_get_a_suffix() {
        let mut used = Vec::new();
        assert_eq!(unique_name(&mut used, "a.SC2Map"), "a.SC2Map");
        assert_eq!(unique_name(&mut used, "a.SC2Map"), "a (2).SC2Map");
        assert_eq!(unique_name(&mut used, "a.SC2Map"), "a (3).SC2Map");
        // 大小写不敏感：a.SC2Map / a (2) / a (3) 都已被占用，所以跳到 (4)
        assert_eq!(unique_name(&mut used, "A.sc2map"), "A (4).sc2map");
    }
}
