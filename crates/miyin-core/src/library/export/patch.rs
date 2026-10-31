//! 单独导出一个补丁。
//!
//! 形态贴着现实里的补丁包来：**内容平铺在根目录**（`X.SC2Mod` / `x.SC2Map`），
//! 加一个 `patch.txt` 声明自己是谁、依赖什么，我们的数据照旧放 `Miyin/`。
//!
//! 平铺是有讲究的：重新导入时裸的 `.SC2Mod` 会被识别成"落到 `Mods/` 下"，
//! 裸的 `.SC2Map` 会被识别成"落到绑定战役的目录下" —— 这样补丁才能挂到任意战役上。
//! 如果写成 `Mods/X.SC2Mod` 就会被当成镜像路径写死，绑到别的战役就摆错了。

use std::path::Path;

use crate::error::{Error, Result};
use crate::library::{Library, Patch, now_seconds};

use super::{ExportFormat, ExportReport, MIYIN_DIR, add_path, unique_name, write_text, zip_error};

/// 导出结果里的 `files` 计数从 0 开始（`patch.txt` 不算内容文件）。
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

    let mut report = ExportReport {
        path: destination.to_path_buf(),
        files: 0,
        maps: 0,
        mods: 0,
        expanded: Vec::new(),
        patches: vec![patch.name.clone()],
        format: ExportFormat::Miyin,
        losses: Vec::new(),
    };

    write_text(&mut zip, "patch.txt", &patch_metadata(patch))?;

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

        add_path(&mut zip, &source, &name, &mut report.files)?;
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
    )?;
    report.files += 1;

    zip.finish().map_err(zip_error)?;

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

/// 元数据里的值不能带换行，否则会破坏 `键=值` 结构。
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ").trim().to_string()
}
