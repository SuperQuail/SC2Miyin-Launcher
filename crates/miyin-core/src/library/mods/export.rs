//! 把模组打包导出成 zip（单条 / 多版本）。

use std::path::Path;

use crate::error::{Error, Result};
use crate::safety;

use super::StandaloneMod;
use super::index::{get, mods_root};

/// 把模组打包导出成一个 zip。
pub fn export(data: &Path, id: &str, target: &Path) -> Result<()> {
    let record = get(data, id).ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;
    let root = mods_root(data);
    let dir = safety::ensure_within(&root, &root.join(&record.id))?;
    if !dir.is_dir() {
        return Err(Error::PackageRejected("模组文件已经不在了".to_string()));
    }

    let file = std::fs::File::create(target)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut wrote = 0usize;
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry
            .path()
            .strip_prefix(&dir)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        let name = relative.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), options)?;
        } else {
            zip.start_file(name, options)?;
            let mut input = std::fs::File::open(entry.path())?;
            std::io::copy(&mut input, &mut zip)?;
            wrote += 1;
        }
    }

    zip.finish()?;
    if wrote == 0 {
        let _ = std::fs::remove_file(target);
        return Err(Error::PackageRejected("这个模组目录是空的".to_string()));
    }
    Ok(())
}

/// 导出**选中的若干版本**到一个 zip。
///
/// 为什么要能挑：一个模组攒了十几个版本之后，全打包出去又大又没人要；
/// 而「把 1.0 和 1.3 一起发过去让人对比」是很实际的需求。
///
/// 包里的摆法：
///
/// `@text
/// <名字>/
/// ├── mods.txt          索引：这个包里带了哪些版本
/// ├── 1.0/             各版本的完整内容
/// └── 1.3/
/// `@
pub fn export_many(data: &Path, ids: &[String], target: &Path) -> Result<usize> {
    if ids.is_empty() {
        return Err(Error::PackageRejected("没有选中任何版本".to_string()));
    }

    let root = mods_root(data);
    let records: Vec<StandaloneMod> = ids.iter().filter_map(|id| get(data, id)).collect();

    if records.is_empty() {
        return Err(Error::PackageRejected("选中的模组都不在库里".to_string()));
    }

    // 用第一个的名字当顶层目录 —— 同 modid 的多个版本本来就是一家人
    let title = crate::campaign::sanitize::sanitize_dir_name(&records[0].name)
        .unwrap_or_else(|| "mod".to_string());

    let file = std::fs::File::create(target)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 索引
    let mut index = String::new();
    index.push_str(
        "# 这个模组包里带了以下版本
",
    );
    if let Some(modid) = records[0].modid.as_deref() {
        index.push_str(&format!(
            "modid={modid}
"
        ));
    }
    index.push_str(&format!(
        "title={title}
"
    ));
    for record in &records {
        index.push_str(&format!(
            "version={}    # 原始目录 {}，{} MB
",
            record
                .version
                .clone()
                .unwrap_or_else(|| "未标版本".to_string()),
            record.id,
            record.size_bytes / 1024 / 1024
        ));
    }
    zip.start_file(format!("{title}/mods.txt"), options)?;
    std::io::Write::write_all(&mut zip, index.as_bytes())?;

    let mut written = 0usize;
    for record in &records {
        let dir = safety::ensure_within(&root, &root.join(&record.id))?;
        if !dir.is_dir() {
            continue;
        }

        let label = record.version.clone().unwrap_or_else(|| record.id.clone());
        let label = crate::campaign::sanitize::sanitize_dir_name(&label)
            .unwrap_or_else(|| record.id.clone());

        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let relative = entry
                .path()
                .strip_prefix(&dir)
                .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
            let name = relative.to_string_lossy().replace('\\', "/");
            if name.is_empty() {
                continue;
            }
            let name = format!("{title}/{label}/{name}");

            if entry.file_type().is_dir() {
                zip.add_directory(format!("{name}/"), options)?;
            } else {
                zip.start_file(name, options)?;
                let mut input = std::fs::File::open(entry.path())?;
                std::io::copy(&mut input, &mut zip)?;
                written += 1;
            }
        }
    }

    zip.finish()?;
    if written == 0 {
        let _ = std::fs::remove_file(target);
        return Err(Error::PackageRejected("选中的版本目录都是空的".to_string()));
    }

    Ok(written)
}
