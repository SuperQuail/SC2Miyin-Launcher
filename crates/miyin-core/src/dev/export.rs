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
    /// 封面图（包内路径）。对应约定里的顶层 `cover`。
    #[serde(default)]
    pub cover: Option<String>,
    /// 模组身份（模组包用）。
    #[serde(default)]
    pub modid: Option<String>,
    /// 这个包里的地图依赖哪些模组。
    #[serde(default)]
    pub mods: Vec<String>,
}

/// 导出结果。
#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
    /// **用真正的解析器把自己刚写的包读一遍**，把读到的关键字段带回来。
    ///
    /// 为什么要这么绕：写出去和读回来是两套假设，只测"zip 里有 metadata.json"
    /// 说明不了任何事。让解析器自己说话，界面才好一眼看出哪个字段没落进去。
    pub read_back: Option<ReadBack>,
}

/// 解析器读回来的关键字段。
#[derive(Debug, Clone, Serialize)]
pub struct ReadBack {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub id: Option<String>,
    pub campaign: Option<String>,
    pub tags: Vec<String>,
    pub main_map: Option<String>,
    pub doc: Option<String>,
    pub cover: Option<String>,
    pub payloads: usize,
}

/// 导出时最多这么多个文件 —— 防手滑把整个游戏目录打进去。
pub const MAX_FILES: usize = 20000;

/// 一条要打进包里的东西：包内路径 + 磁盘上的绝对路径。
#[derive(Debug, Clone, Deserialize)]
pub struct ExportFile {
    /// 包内路径（`/` 分隔）
    pub path: String,
    pub abs: String,
    /// 这是一层**目录**。
    ///
    /// 空目录也要能进包 —— 作者可能就是要留一个空壳子（游戏的某些目录约定）。
    #[serde(default)]
    pub is_dir: bool,
}

/// 把文件打成 zip。
///
/// - `files` 里每条都按**原样路径**进包；
/// - 附一份 `metadata.json`；
/// - 目录项自动补齐（解压工具不至于把路径当平铺文件名）。
pub fn export(dest: &Path, meta: &PackageMeta, files: &[ExportFile]) -> Result<ExportReport> {
    if files.is_empty() {
        return Err(Error::PackageRejected(
            "一个都没勾 —— 先把要打进去的东西选上".to_string(),
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
        let entry = item.path.trim_start_matches('/').replace('\\', "/");
        if entry.is_empty() || entry.contains("..") {
            return Err(Error::PackageRejected(format!(
                "包内路径不合法：{}",
                item.path
            )));
        }

        // 目录：只写一个目录项 —— **空目录就是这么进包的**
        if item.is_dir {
            if !source.is_dir() {
                return Err(Error::PackageRejected(format!("找不到目录：{}", item.abs)));
            }
            if !dirs.contains(&entry) {
                zip.add_directory(format!("{entry}/"), options)?;
                dirs.push(entry);
            }
            written += 1;
            continue;
        }

        if !source.is_file() {
            // 勾的东西中途没了 —— 明确报错，别悄悄少一个文件
            return Err(Error::PackageRejected(format!("找不到文件：{}", item.abs)));
        }

        // 补父目录项
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

    // 自检：把自己刚写出来的包交给真正的解析器读一遍
    let read_back = crate::campaign::package::inspect(dest)
        .ok()
        .map(|item| ReadBack {
            name: item.name,
            author: item.author,
            version: item.version,
            description: item.description,
            id: item.id,
            campaign: item.campaign_type.main_slot().map(|slot| slot.to_string()),
            tags: item.tags,
            main_map: item.main_map,
            doc: item.doc,
            cover: item.cover,
            payloads: item.payloads.len(),
        });

    Ok(ExportReport {
        path: dest.to_string_lossy().into_owned(),
        files: written,
        bytes,
        read_back,
    })
}

/// 拼 `metadata.json`。
///
/// 顶层字段走**星际枢纽 / CCM 都认**的那几个名字；弥音自己的东西一律塞进
/// `miyin` 命名空间，别的工具直接忽略。
fn metadata_json(meta: &PackageMeta) -> Result<String> {
    let mut miyin = serde_json::Map::new();
    let mut object = serde_json::Map::new();

    // 空字段一律不写 —— 约定里每个字段都是可选的，
    // 塞一堆 null 进去只会让别的工具以为"作者显式写了空"。
    let put = |target: &mut serde_json::Map<String, serde_json::Value>,
               key: &str,
               value: Option<&String>| {
        if let Some(text) = value.filter(|text| !text.trim().is_empty()) {
            target.insert(key.to_string(), serde_json::Value::from(text.clone()));
        }
    };

    put(&mut object, "name", Some(&meta.name));
    put(&mut object, "author", meta.author.as_ref());
    put(&mut object, "version", meta.version.as_ref());
    put(&mut object, "description", meta.description.as_ref());
    put(&mut object, "campaign", meta.campaign.as_ref());
    put(&mut object, "cover", meta.cover.as_ref());

    put(&mut miyin, "id", meta.id.as_ref());
    put(&mut miyin, "kind", meta.kind.as_ref());
    put(&mut miyin, "doc", meta.doc.as_ref());
    put(&mut miyin, "modid", meta.modid.as_ref());
    if !meta.mods.is_empty() {
        miyin.insert(
            "mods".to_string(),
            serde_json::Value::from(meta.mods.clone()),
        );
    }
    if !meta.tags.is_empty() {
        miyin.insert(
            "tags".to_string(),
            serde_json::Value::from(meta.tags.clone()),
        );
    }

    // `main_map` 是**自制战役**的键（约定 §「自制战役的两个新键」）——
    // 官方战役改版没有"入口地图"这回事，写了只会让别家工具困惑。
    if meta
        .campaign
        .as_ref()
        .is_none_or(|text| text.trim().is_empty())
    {
        put(&mut miyin, "main_map", meta.main_map.as_ref());
    }

    // **声明的格式版本 = 实际用到的最高那一档**（约定 §5）：
    // 多报会让老启动器白白拒绝，少报会让它按老语义解析新字段。
    let mut format = 1;
    if meta.kind.is_some() {
        format = format.max(2);
    }
    miyin.insert("format".to_string(), serde_json::Value::from(format));

    // 键排一下序，生成的文件 diff 起来好看
    object.insert("miyin".to_string(), serde_json::Value::Object(miyin));

    serde_json::to_string_pretty(&serde_json::Value::Object(object))
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
                is_dir: false,
            },
            ExportFile {
                path: "Mods/b.SC2Mod".to_string(),
                abs: mod_file.to_string_lossy().into_owned(),
                is_dir: false,
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
        assert!(
            !text.contains("\"format\": 1\n}") && !text.trim_start().starts_with("{\n  \"format\""),
            "顶层不该有 format —— 版本声明在 miyin.format 里：{text}"
        );
        assert!(
            text.contains("\"format\": 1"),
            "没写补丁字段就声明 1（多报会让老启动器白白拒绝）：{text}"
        );
        assert!(!text.contains(": null"), "空字段不该写成 null：{text}");
        assert!(text.contains("重制"), "标签要带上");
        assert!(
            !text.contains("main_map"),
            "归属官方战役时不该写 main_map —— 那是自制战役的键"
        );

        // 自制战役（不填归属）才写 main_map
        let custom = PackageMeta {
            name: "自制".to_string(),
            main_map: Some("Maps/CustomCampaigns/自制/01.SC2Map".to_string()),
            ..PackageMeta::default()
        };
        let custom_dest = tmp.path().join("自制.zip");
        export(&custom_dest, &custom, &files).expect("导出");
        let archive = std::fs::File::open(&custom_dest).expect("开包");
        let mut zip = zip::ZipArchive::new(archive).expect("读包");
        let mut custom_text = String::new();
        std::io::Read::read_to_string(
            &mut zip.by_name("metadata.json").expect("元数据"),
            &mut custom_text,
        )
        .expect("读");
        assert!(
            custom_text.contains("main_map"),
            "自制战役要写 main_map：{custom_text}"
        );
    }

    /// 声明的格式版本 = **实际用到了哪一档**（约定 §5）。
    #[test]
    fn format_declaration_follows_what_we_write() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let map = tmp.path().join("a.SC2Map");
        std::fs::write(&map, "x").expect("写");
        let files = vec![ExportFile {
            path: "Maps/a.SC2Map".to_string(),
            abs: map.to_string_lossy().into_owned(),
            is_dir: false,
        }];

        let read = |name: &str| {
            let archive = std::fs::File::open(tmp.path().join(name)).expect("开包");
            let mut zip = zip::ZipArchive::new(archive).expect("读包");
            let mut text = String::new();
            std::io::Read::read_to_string(
                &mut zip.by_name("metadata.json").expect("元数据"),
                &mut text,
            )
            .expect("读");
            text
        };

        export(
            &tmp.path().join("基础.zip"),
            &PackageMeta {
                name: "基础".to_string(),
                ..PackageMeta::default()
            },
            &files,
        )
        .expect("导出");
        let base = read("基础.zip");
        assert!(base.contains("\"format\": 1"), "只有基础字段就是 1：{base}");

        export(
            &tmp.path().join("补丁.zip"),
            &PackageMeta {
                name: "补丁".to_string(),
                kind: Some("patch".to_string()),
                ..PackageMeta::default()
            },
            &files,
        )
        .expect("导出");
        let patch = read("补丁.zip");
        assert!(
            patch.contains("\"format\": 2"),
            "用了补丁字段（kind）就要声明 2：{patch}"
        );
    }

    /// **导出 → 用真正的解析器读回来**，每个字段都要对得上。
    ///
    /// 这条测试是用户要求加的：导出的包连作者都读不到，说明"写出去"和"读回来"
    /// 用的是两套假设。只测"zip 里有 metadata.json"是不够的 —— 得让
    /// `package::inspect` 自己说它读到了什么。
    #[test]
    fn export_then_parse_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let map = tmp.path().join("01.SC2Map");
        std::fs::write(&map, b"x").expect("写");
        let cover = tmp.path().join("cover.png");
        std::fs::write(&cover, b"png").expect("写");
        let doc = tmp.path().join("说明.pdf");
        std::fs::write(&doc, b"pdf").expect("写");
        let files = vec![
            ExportFile {
                path: "Maps/CustomCampaigns/示例/01.SC2Map".to_string(),
                abs: map.to_string_lossy().into_owned(),
                is_dir: false,
            },
            // 封面 / 说明书是要真的进包的 —— 光在元数据里写个名字，
            // 导入时 resolve_cover / resolve_doc 找不到文件就会静默丢掉
            ExportFile {
                path: "cover.png".to_string(),
                abs: cover.to_string_lossy().into_owned(),
                is_dir: false,
            },
            ExportFile {
                path: "说明.pdf".to_string(),
                abs: doc.to_string_lossy().into_owned(),
                is_dir: false,
            },
        ];
        let meta = PackageMeta {
            name: "往返测试".to_string(),
            author: Some("作者名".to_string()),
            version: Some("1.2".to_string()),
            description: Some("一句说明".to_string()),
            campaign: None,
            kind: Some("campaign".to_string()),
            id: Some("quail.roundtrip".to_string()),
            tags: vec!["重制".to_string(), "剧情".to_string()],
            main_map: Some("Maps/CustomCampaigns/示例/01.SC2Map".to_string()),
            doc: Some("说明.pdf".to_string()),
            cover: Some("cover.png".to_string()),
            modid: None,
            mods: Vec::new(),
        };
        let dest = tmp.path().join("往返.zip");
        export(&dest, &meta, &files).expect("导出");

        let inspection = crate::campaign::package::inspect(&dest).expect("解析自己导出的包");
        assert_eq!(inspection.name.as_deref(), Some("往返测试"), "名称");
        assert_eq!(inspection.author.as_deref(), Some("作者名"), "作者");
        assert_eq!(inspection.version.as_deref(), Some("1.2"), "版本");
        assert_eq!(inspection.description.as_deref(), Some("一句说明"), "说明");
        assert_eq!(inspection.id.as_deref(), Some("quail.roundtrip"), "注册 ID");
        assert_eq!(
            inspection.tags,
            vec!["重制".to_string(), "剧情".to_string()],
            "标签"
        );
        assert_eq!(
            inspection.main_map.as_deref(),
            Some("Maps/CustomCampaigns/示例/01.SC2Map"),
            "主地图"
        );
        assert_eq!(inspection.doc.as_deref(), Some("说明.pdf"), "说明书");
        assert_eq!(inspection.cover.as_deref(), Some("cover.png"), "封面");
        assert_eq!(inspection.payloads.len(), 1, "载荷");

        // 光解析出来还不算"读得到" —— 用户是**导入之后**在版本卡上看作者。
        // 所以再走一遍真正的导入，看落进库里的那条记录。
        let library_root = tmp.path().join("data");
        let library = crate::library::Library::new(library_root);
        let variant = crate::library::import(
            &library,
            &dest,
            "custom",
            crate::library::ImportMode::Rename,
        )
        .expect("导入自己导出的包");
        assert_eq!(variant.name, "往返测试", "导入后的名称");
        assert_eq!(variant.author.as_deref(), Some("作者名"), "导入后的作者");
        assert_eq!(variant.version.as_deref(), Some("1.2"), "导入后的版本");
        assert_eq!(
            variant.description.as_deref(),
            Some("一句说明"),
            "导入后的说明"
        );
        assert_eq!(
            variant.tags,
            vec!["重制".to_string(), "剧情".to_string()],
            "导入后的标签"
        );
        assert_eq!(
            variant.registration_id.as_deref(),
            Some("quail.roundtrip"),
            "导入后的注册 ID"
        );
        assert_eq!(
            variant.main_map.as_deref(),
            Some("Maps/CustomCampaigns/示例/01.SC2Map"),
            "导入后的主地图"
        );
        assert_eq!(variant.doc.as_deref(), Some("说明.pdf"), "导入后的说明书");
        assert_eq!(variant.cover.as_deref(), Some("cover.png"), "导入后的封面");
    }

    #[test]
    fn refuses_empty_selection_and_missing_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dest = tmp.path().join("空.zip");
        let meta = PackageMeta::default();

        assert!(export(&dest, &meta, &[]).is_err(), "空勾选要拒绝");

        let missing = vec![ExportFile {
            path: "Maps/a.SC2Map".to_string(),
            is_dir: false,
            abs: tmp
                .path()
                .join("没有这个文件")
                .to_string_lossy()
                .into_owned(),
        }];
        assert!(export(&dest, &meta, &missing).is_err(), "文件没了要报错");
    }

    /// 空目录也要能进包 —— 作者可能就是要留一个空壳子。
    #[test]
    fn empty_directories_are_packed() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let empty = tmp.path().join("空的");
        std::fs::create_dir_all(&empty).expect("建目录");

        let files = vec![ExportFile {
            path: "Maps/CustomCampaigns/留个位置".to_string(),
            abs: empty.to_string_lossy().into_owned(),
            is_dir: true,
        }];
        let dest = tmp.path().join("空目录.zip");
        let report = export(&dest, &PackageMeta::default(), &files).expect("导出");
        assert_eq!(report.files, 1);

        let archive = std::fs::File::open(&dest).expect("开包");
        let mut zip = zip::ZipArchive::new(archive).expect("读包");
        let names: Vec<String> = (0..zip.len())
            .map(|index| zip.by_index(index).expect("条目").name().to_string())
            .collect();
        assert!(
            names.contains(&"Maps/CustomCampaigns/留个位置/".to_string()),
            "空目录要以目录项进包，实得：{names:?}"
        );
    }

    #[test]
    fn refuses_paths_that_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let map = touch(tmp.path(), "a.SC2Map", "x");
        let dest = tmp.path().join("坏.zip");
        let files = vec![ExportFile {
            path: "../外面/a.SC2Map".to_string(),
            abs: map.to_string_lossy().into_owned(),
            is_dir: false,
        }];
        assert!(export(&dest, &PackageMeta::default(), &files).is_err());
    }
}
