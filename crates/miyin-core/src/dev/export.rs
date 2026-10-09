//! 开发者页的**导出**：把勾选的那批文件打成一个压缩包，附上元数据。
//!
//! 和 `library::export`（导出战役库里的版本）不是一回事：
//! 那边导的是库里已有的版本，这边导的是**开发者自己挑的一堆文件** ——
//! 游戏目录里的、别处的都行，按原样打进包里。
//!
//! 包内路径**保持游戏目录里的相对路径**（`Maps/Campaign/void/x.SC2Map`），
//! 也就是"镜像包"形态 —— 导入时 `has_mirror_root` 认得出来，
//! 落点不用重新猜。

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// 包的元数据。写进 `metadata.json` 的顶层 + `miyin` 命名空间。
///
/// 字段都允许空 —— 开发者可以先导一个能用的包出来，细节以后再补。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PackageMeta {
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// 归属哪部官方战役（`wol` / `hots` / `lotv` / `nova`），自制战役留空。
    #[serde(default)]
    pub campaign: Option<String>,
    /// `campaign` 或 `patch`。
    #[serde(default)]
    pub kind: Option<String>,
    /// 注册 ID —— 同一个战役的多个版本靠它认亲。
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 包内路径：自制战役的游玩入口地图。
    #[serde(default)]
    pub main_map: Option<String>,
    /// 说明文档（包内路径）。
    #[serde(default)]
    pub doc: Option<String>,
}

/// 导出结果。
#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

/// 导出时最多这么多个文件 —— 防手滑把整个游戏目录打进去。
pub const MAX_FILES: usize = 20000;

/// 一条要打进包里的东西：包内路径 + 磁盘上的绝对路径。
#[derive(Debug, Clone, Deserialize)]
pub struct ExportFile {
    /// 包内路径（`/` 分隔）
    pub path: String,
    pub abs: String,
}

/// 把文件打成 zip。
///
/// - `files` 里每条都按**原样路径**进包；
/// - 附一份 `metadata.json`；
/// - 目录项自动补齐（解压工具不至于把路径当平铺文件名）。
pub fn export(dest: &Path, meta: &PackageMeta, files: &[ExportFile]) -> Result<ExportReport> {
    if files.is_empty() {
        return Err(Error::PackageRejected(
            "一个文件都没勾 —— 先把要打进去的东西选上".to_string(),
        ));
    }
    if files.len() > MAX_FILES {
        return Err(Error::PackageRejected(format!(
            "文件太多了（{} 个，上限 {MAX_FILES}）—— 确认一下是不是把整个游戏目录勾进去了",
            files.len()
        )));
    }

    let file = std::fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut bytes = 0u64;
    let mut written = 0usize;
    let mut dirs: Vec<String> = Vec::new();

    for item in files {
        let source = PathBuf::from(&item.abs);
        if !source.is_file() {
            // 勾的东西中途没了 —— 明确报错，别悄悄少一个文件
            return Err(Error::PackageRejected(format!("找不到文件：{}", item.abs)));
        }
        let entry = item.path.trim_start_matches('/').replace('\\', "/");
        if entry.is_empty() || entry.contains("..") {
            return Err(Error::PackageRejected(format!(
                "包内路径不合法：{}",
                item.path
            )));
        }

        // 补目录项
        let mut prefix = String::new();
        for part in entry.split('/').take(entry.split('/').count() - 1) {
            prefix.push_str(part);
            prefix.push('/');
            let dir = prefix.trim_end_matches('/').to_string();
            if !dirs.contains(&dir) {
                zip.add_directory(format!("{dir}/"), options)?;
                dirs.push(dir);
            }
        }

        zip.start_file(&entry, options)?;
        let mut input = std::fs::File::open(&source)?;
        bytes += std::io::copy(&mut input, &mut zip)?;
        written += 1;
    }

    zip.start_file("metadata.json", options)?;
    zip.write_all(metadata_json(meta)?.as_bytes())?;
    zip.finish()?;

    Ok(ExportReport {
        path: dest.to_string_lossy().into_owned(),
        files: written,
        bytes,
    })
}

/// 拼 `metadata.json`。
///
/// 顶层字段走**星际枢纽 / CCM 都认**的那几个名字；弥音自己的东西一律塞进
/// `miyin` 命名空间，别的工具直接忽略。
fn metadata_json(meta: &PackageMeta) -> Result<String> {
    let mut miyin = serde_json::Map::new();
    miyin.insert(
        "format".to_string(),
        serde_json::Value::from(crate::campaign::package::MIYIN_FORMAT_VERSION),
    );
    for (key, value) in [("id", meta.id.as_ref()), ("kind", meta.kind.as_ref())] {
        if let Some(value) = value.filter(|text| !text.trim().is_empty()) {
            miyin.insert(key.to_string(), serde_json::Value::from(value.clone()));
        }
    }
    if let Some(main_map) = meta
        .main_map
        .as_ref()
        .filter(|text| !text.trim().is_empty())
    {
        miyin.insert(
            "main_map".to_string(),
            serde_json::Value::from(main_map.clone()),
        );
    }
    if let Some(doc) = meta.doc.as_ref().filter(|text| !text.trim().is_empty()) {
        miyin.insert("doc".to_string(), serde_json::Value::from(doc.clone()));
    }
    if !meta.tags.is_empty() {
        miyin.insert(
            "tags".to_string(),
            serde_json::Value::from(meta.tags.clone()),
        );
    }

    let document = serde_json::json!({
        "format": 2,
        "name": meta.name,
        "author": meta.author,
        "version": meta.version,
        "description": meta.description,
        "campaign": meta.campaign,
        "miyin": miyin,
    });

    serde_json::to_string_pretty(&document)
        .map_err(|error| Error::PackageRejected(format!("元数据写不出来：{error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str, body: &str) -> PathBuf {
        std::fs::create_dir_all(dir).expect("建目录");
        let path = dir.join(name);
        std::fs::write(&path, body).expect("写");
        path
    }

    #[test]
    fn exports_paths_metadata_and_directory_entries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("游戏目录");
        let map = touch(&src, "a.SC2Map", "地图内容");
        let mod_file = touch(&src, "b.SC2Mod", "模组内容");

        let files = vec![
            ExportFile {
                path: "Maps/Campaign/void/a.SC2Map".to_string(),
                abs: map.to_string_lossy().into_owned(),
            },
            ExportFile {
                path: "Mods/b.SC2Mod".to_string(),
                abs: mod_file.to_string_lossy().into_owned(),
            },
        ];
        let meta = PackageMeta {
            name: "我的战役".to_string(),
            author: Some("作者".to_string()),
            version: Some("1.0".to_string()),
            campaign: Some("void".to_string()),
            main_map: Some("Maps/Campaign/void/a.SC2Map".to_string()),
            tags: vec!["重制".to_string()],
            ..PackageMeta::default()
        };

        let dest = tmp.path().join("导出.zip");
        let report = export(&dest, &meta, &files).expect("导出");
        assert_eq!(report.files, 2);
        assert!(dest.is_file());

        // 包内路径、目录项、元数据都要在
        let archive = std::fs::File::open(&dest).expect("开包");
        let mut zip = zip::ZipArchive::new(archive).expect("读包");
        let names: Vec<String> = (0..zip.len())
            .map(|index| zip.by_index(index).expect("条目").name().to_string())
            .collect();
        assert!(names.contains(&"Maps/Campaign/void/a.SC2Map".to_string()));
        assert!(names.contains(&"Mods/b.SC2Mod".to_string()));
        assert!(names.contains(&"metadata.json".to_string()));
        assert!(
            names.iter().any(|name| name.ends_with('/')),
            "目录项要补上，解压工具才不会把路径当文件名"
        );

        let mut text = String::new();
        std::io::Read::read_to_string(
            &mut zip.by_name("metadata.json").expect("元数据"),
            &mut text,
        )
        .expect("读");
        assert!(text.contains("我的战役"));
        assert!(text.contains("main_map"), "主地图要写进 miyin 命名空间");
        assert!(text.contains("重制"), "标签要带上");
    }

    #[test]
    fn refuses_empty_selection_and_missing_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dest = tmp.path().join("空.zip");
        let meta = PackageMeta::default();

        assert!(export(&dest, &meta, &[]).is_err(), "空勾选要拒绝");

        let missing = vec![ExportFile {
            path: "Maps/a.SC2Map".to_string(),
            abs: tmp
                .path()
                .join("没有这个文件")
                .to_string_lossy()
                .into_owned(),
        }];
        assert!(export(&dest, &meta, &missing).is_err(), "文件没了要报错");
    }

    #[test]
    fn refuses_paths_that_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let map = touch(tmp.path(), "a.SC2Map", "x");
        let dest = tmp.path().join("坏.zip");
        let files = vec![ExportFile {
            path: "../外面/a.SC2Map".to_string(),
            abs: map.to_string_lossy().into_owned(),
        }];
        assert!(export(&dest, &PackageMeta::default(), &files).is_err());
    }
}
