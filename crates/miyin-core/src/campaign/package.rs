//! 战役包预检。
//!
//! 在真正解压之前，先把压缩包"读一遍"，回答三个问题：
//!
//! 1. **这是什么格式**（CCM `metadata.txt` / 弥音标准 `metadata.json` / 无元数据）
//! 2. **里面写了什么**（名称、作者、版本、归属资料片、地图数量）
//! 3. **能不能装**（条目名是否越界、是否超过体积与条目数上限、是否根本没有地图）
//!
//! # 相比参考实现修正了什么
//!
//! - **补上了 zip-slip 校验**。参考实现完全依赖第三方库的路径处理行为，
//!   一旦换库或升级就可能把文件写到游戏目录之外。
//! - **解压前先算总字节数与条目数**，挡掉 zip bomb。
//! - **识别元数据所在的内容根**（`MyCampaign/metadata.txt` 这种最常见布局），
//!   解压时剥离该前缀 —— 参考实现处理这种包时必然报 `EPERM/ENOTEMPTY`。
//! - **zip 内文件名编码异常时给出提示**，而不是让用户面对乱码文件夹。

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::campaign::metadata::{CampaignType, CcmMetadata, StandardMetadata, decode_text};
use crate::campaign::sanitize::sanitize_dir_name;
use crate::campaign::{CampaignFormat, HealthIssue};
use crate::error::Result;

/// 允许解压的最大总字节数（防 zip bomb）。
pub const MAX_UNPACKED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// 允许的最大条目数。
pub const MAX_ENTRIES: usize = 100_000;

/// 包内一个条目的摘要。
#[derive(Debug, Clone)]
struct Entry {
    index: usize,
    /// 校验并清理后的相对路径。
    relative: PathBuf,
    is_dir: bool,
}

/// 包预检结果。
#[derive(Debug, Clone, Serialize)]
pub struct PackageInspection {
    pub path: PathBuf,
    /// 是否可以安装（没有任何「无法运行」级别的问题）。
    pub installable: bool,
    pub format: CampaignFormat,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub campaign_type: CampaignType,
    /// 包内实际内容根（元数据所在目录），解压时需剥离；空串表示包根。
    pub content_root: String,
    /// 建议的安装目录名（已安全化）。
    pub suggested_dir_name: Option<String>,
    pub map_count: usize,
    pub mod_count: usize,
    pub entry_count: usize,
    pub unpacked_bytes: u64,
    pub issues: Vec<HealthIssue>,
}

impl PackageInspection {
    /// 解析出的显示名，取不到时退回目录名。
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.suggested_dir_name.clone())
            .unwrap_or_else(|| "未命名战役".to_string())
    }

    /// 是否存在「无法运行」级别的问题。
    pub fn has_blocking_issue(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.level == crate::campaign::HealthLevel::Broken)
    }
}

/// 校验 zip 条目名，并返回清理后的相对路径。
///
/// 返回 `None` 表示该条目**越界或不可用**，必须整体拒绝这个包：
/// 绝对路径、盘符、UNC、以及任何 `..` 组件。
pub fn safe_entry_path(name: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");

    if normalized.starts_with('/') {
        return None;
    }
    // 盘符形式：C:\\foo
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        return None;
    }

    let mut out = PathBuf::new();
    for component in normalized.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            return None;
        }
        // 复用目录名安全化：非法字符替换、保留名拒绝、长度截断
        out.push(sanitize_dir_name(component)?);
    }

    (!out.as_os_str().is_empty()).then_some(out)
}

/// 读取并检查一个战役包。
pub fn inspect(path: &Path) -> Result<PackageInspection> {
    let file = std::fs::File::open(path)?;
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(error) => {
            return Ok(PackageInspection {
                path: path.to_path_buf(),
                installable: false,
                format: CampaignFormat::Unknown,
                name: None,
                author: None,
                version: None,
                description: None,
                campaign_type: CampaignType::Other(String::new()),
                content_root: String::new(),
                suggested_dir_name: None,
                map_count: 0,
                mod_count: 0,
                entry_count: 0,
                unpacked_bytes: 0,
                issues: vec![
                    HealthIssue::broken(
                        "NOT_AN_ARCHIVE",
                        format!("不是有效的 zip 压缩包：{error}"),
                    )
                    .with_hint("请确认下载完整，或重新导出为 .zip"),
                ],
            });
        }
    };

    let mut issues: Vec<HealthIssue> = Vec::new();
    let mut entries: Vec<Entry> = Vec::with_capacity(archive.len());
    let mut unpacked_bytes: u64 = 0;
    let mut unsafe_name: Option<String> = None;
    let mut lossy_names = 0usize;

    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        let raw = file.name().to_string();
        let lossy_name = std::str::from_utf8(file.name_raw()).is_err();
        if lossy_name {
            lossy_names += 1;
        }
        let size = file.size();
        let is_dir = file.is_dir();
        drop(file);

        let Some(relative) = safe_entry_path(&raw) else {
            unsafe_name.get_or_insert(raw);
            continue;
        };

        if !is_dir {
            unpacked_bytes = unpacked_bytes.saturating_add(size);
        }
        entries.push(Entry {
            index,
            relative,
            is_dir,
        });
    }

    // ---- 安全闸门 ----
    if let Some(name) = unsafe_name {
        issues.push(
            HealthIssue::broken(
                "UNSAFE_ENTRY",
                format!("压缩包内存在越界路径条目，已拒绝安装：{name}"),
            )
            .with_hint("该包含有指向压缩包之外的路径（zip-slip），可能来自恶意文件"),
        );
    }

    if entries.len() > MAX_ENTRIES {
        issues.push(HealthIssue::broken(
            "TOO_MANY_ENTRIES",
            format!("压缩包条目过多（{} 个，上限 {MAX_ENTRIES}）", entries.len()),
        ));
    }

    if unpacked_bytes > MAX_UNPACKED_BYTES {
        issues.push(HealthIssue::broken(
            "TOO_LARGE",
            format!(
                "解压后体积过大（{:.1} GB，上限 {:.0} GB）",
                unpacked_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                MAX_UNPACKED_BYTES as f64 / 1024.0 / 1024.0 / 1024.0
            ),
        ));
    }

    if lossy_names > 0 {
        issues.push(HealthIssue::warning(
            "NON_UTF8_NAMES",
            format!("压缩包内有 {lossy_names} 个文件名不是 UTF-8 编码，解压后可能显示为乱码"),
        ));
    }

    // ---- 元数据定位 ----
    let standard_entry = entries.iter().find(|entry| {
        entry.relative.as_os_str() == "metadata.json"
            || entry.relative.as_os_str() == "Metadata.json"
    });

    let ccm_entry = entries
        .iter()
        .filter(|entry| {
            entry
                .relative
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("metadata.txt"))
        })
        .min_by_key(|entry| entry.relative.components().count());

    // 子目录里的 metadata.json 会被忽略：这是参考实现踩过的坑（装了却扫不到）
    if standard_entry.is_none()
        && entries.iter().any(|entry| {
            entry
                .relative
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("metadata.json"))
        })
    {
        issues.push(HealthIssue::warning(
            "NESTED_METADATA_JSON",
            "metadata.json 不在压缩包根目录，已按无元数据包处理",
        ));
    }

    let mut name = None;
    let mut author = None;
    let mut version = None;
    let mut description = None;
    let mut campaign_raw = String::new();
    let mut content_root = String::new();
    let mut format = CampaignFormat::Plain;

    if let Some(entry) = standard_entry {
        format = CampaignFormat::Standard;
        content_root = parent_prefix(&entry.relative);
        if let Some(text) = read_entry(&mut archive, entry.index) {
            match StandardMetadata::parse(&decode_text(&text)) {
                Ok(meta) => {
                    name = clean(meta.name);
                    author = clean(meta.author);
                    version = clean(meta.version);
                    description = clean(meta.description);
                    campaign_raw = clean(meta.campaign).unwrap_or_default();
                    if let Some(kind) = meta.kind.as_deref()
                        && !kind.eq_ignore_ascii_case("campaign")
                    {
                        issues.push(HealthIssue::broken(
                            "NOT_A_CAMPAIGN",
                            format!("该包的 type 是 {kind}，不是战役包"),
                        ));
                    }
                }
                Err(error) => issues.push(
                    HealthIssue::warning(
                        "BAD_METADATA_JSON",
                        format!("metadata.json 解析失败：{error}"),
                    )
                    .with_hint("将退回使用压缩包文件名作为战役名"),
                ),
            }
        }
    } else if let Some(entry) = ccm_entry {
        format = CampaignFormat::Ccm;
        content_root = parent_prefix(&entry.relative);
        if let Some(text) = read_entry(&mut archive, entry.index) {
            let meta = CcmMetadata::parse(&decode_text(&text));
            name = clean(meta.title);
            author = clean(meta.author);
            version = clean(meta.version);
            description = clean(meta.description);
            campaign_raw = clean(meta.campaign).unwrap_or_default();
        }
    } else {
        issues.push(
            HealthIssue::warning(
                "NO_METADATA",
                "包内没有 metadata.txt / metadata.json，将按无元数据包安装",
            )
            .with_hint("仍然可以安装，但启动器无法显示作者与版本信息"),
        );
    }

    let campaign_type = CampaignType::parse(&campaign_raw);
    if format != CampaignFormat::Plain && !campaign_type.is_actionable() {
        issues.push(
            HealthIssue::warning(
                "UNKNOWN_CAMPAIGN",
                format!("无法识别归属资料片（campaign = \"{campaign_raw}\"），安装后无法启用"),
            )
            .with_hint("该战役仍可安装到自制战役目录，供支持手动选择的工具使用"),
        );
    }

    // ---- 内容统计（丢弃内容根前缀） ----
    let mut map_count = 0usize;
    let mut mod_count = 0usize;
    for entry in &entries {
        if entry.is_dir {
            continue;
        }
        let Some(relative) = strip_prefix(&entry.relative, &content_root) else {
            continue;
        };
        match extension_of(&relative).as_str() {
            "sc2map" => map_count += 1,
            "sc2mod" => mod_count += 1,
            _ => {}
        }
    }

    if map_count == 0 && mod_count == 0 {
        issues.push(
            HealthIssue::warning("NO_CONTENT", "包内没有找到 .SC2Map 地图或 .SC2Mod 模组文件")
                .with_hint("这可能不是战役包，或使用了未支持的打包方式"),
        );
    }

    // ---- 目录名 ----
    let zip_stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();

    // 元数据里的名字不可用时（空、纯点、保留名、全是非法字符），
    // 退回使用压缩包文件名 —— 而不是像参考实现那样硬着头皮拿去拼路径。
    if let Some(raw_name) = &name
        && sanitize_dir_name(raw_name).is_none()
    {
        issues.push(
            HealthIssue::warning(
                "UNUSABLE_TITLE",
                format!("元数据中的名称无法用作目录名（{raw_name}），已改用压缩包文件名"),
            )
            .with_hint("请确认这个包是否被损坏或被恶意构造"),
        );
    }

    let suggested_dir_name = sanitize_dir_name(name.as_deref().unwrap_or_default())
        .or_else(|| sanitize_dir_name(&zip_stem));

    if suggested_dir_name.is_none() {
        issues.push(HealthIssue::broken(
            "UNUSABLE_NAME",
            "从包中无法推导出可用的安装目录名",
        ));
    }

    Ok(PackageInspection {
        path: path.to_path_buf(),
        installable: !issues
            .iter()
            .any(|issue| issue.level == crate::campaign::HealthLevel::Broken),
        format,
        name,
        author,
        version,
        description,
        campaign_type,
        content_root,
        suggested_dir_name,
        map_count,
        mod_count,
        entry_count: entries.len(),
        unpacked_bytes,
        issues,
    })
}

/// 收集包内条目（供解压使用）。
pub(crate) fn collect_entries(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Vec<(usize, PathBuf, bool)> {
    let mut out = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        let raw = file.name().to_string();
        let is_dir = file.is_dir();
        drop(file);
        if let Some(relative) = safe_entry_path(&raw) {
            out.push((index, relative, is_dir));
        }
    }
    out
}

/// 读取包内某个条目的全部字节。
pub(crate) fn read_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    index: usize,
) -> Option<Vec<u8>> {
    let mut file = archive.by_index(index).ok()?;
    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

/// 去掉内容根前缀，得到包内相对路径。
pub(crate) fn strip_prefix(path: &Path, prefix: &str) -> Option<PathBuf> {
    if prefix.is_empty() {
        return Some(path.to_path_buf());
    }
    path.strip_prefix(prefix).ok().map(Path::to_path_buf)
}

/// 取相对路径的父目录字符串（统一用 `/` 分隔）。
fn parent_prefix(path: &Path) -> String {
    let Some(parent) = path.parent() else {
        return String::new();
    };
    parent
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 小写扩展名。
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// 把空字符串与纯空白视为「没有值」。
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_traversal_and_absolute_entry_names() {
        assert_eq!(safe_entry_path("../evil.txt"), None);
        assert_eq!(safe_entry_path("a/../../evil.txt"), None);
        assert_eq!(safe_entry_path("/etc/passwd"), None);
        assert_eq!(safe_entry_path("C:\\Windows\\system32"), None);
        assert_eq!(safe_entry_path("\\\\server\\share\\x"), None);
    }

    #[test]
    fn accepts_and_normalizes_normal_entry_names() {
        assert_eq!(
            safe_entry_path("MyCampaign/maps/a.SC2Map"),
            Some(PathBuf::from("MyCampaign/maps/a.SC2Map"))
        );
        assert_eq!(
            safe_entry_path("./MyCampaign//b.txt"),
            Some(PathBuf::from("MyCampaign/b.txt"))
        );
    }

    #[test]
    fn derives_content_root_from_nested_metadata() {
        assert_eq!(
            parent_prefix(Path::new("MyCampaign/metadata.txt")),
            "MyCampaign"
        );
        assert_eq!(parent_prefix(Path::new("metadata.txt")), "");
        assert_eq!(parent_prefix(Path::new("a/b/metadata.txt")), "a/b");
    }

    #[test]
    fn strip_prefix_drops_content_root() {
        let stripped = strip_prefix(Path::new("MyCampaign/maps/a.SC2Map"), "MyCampaign");
        assert_eq!(stripped, Some(PathBuf::from("maps/a.SC2Map")));
    }

    /// 用指定条目构造一个 zip。
    fn build_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).expect("create zip");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (entry_name, content) in files {
            writer.start_file(*entry_name, options).expect("start file");
            writer.write_all(content).expect("write");
        }
        writer.finish().expect("finish");
        path
    }

    #[test]
    fn inspects_ccm_package_with_nested_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(
            dir.path(),
            "reborn.zip",
            &[
                (
                    "Reborn/metadata.txt",
                    "title=Wings of Liberty: Reborn\ndesc=重制版\nauthor=SomeCreator\ncampaign=WOL\nversion=1.4.2\n"
                        .as_bytes(),
                ),
                ("Reborn/maps/01.SC2Map", b"stub"),
                ("Reborn/maps/02.SC2Map", b"stub"),
                ("Reborn/Mods/extra.SC2Mod", b"stub"),
            ],
        );

        let inspection = inspect(&archive).expect("预检应成功");
        assert!(inspection.installable, "问题：{:?}", inspection.issues);
        assert_eq!(inspection.format, CampaignFormat::Ccm);
        assert_eq!(inspection.name.as_deref(), Some("Wings of Liberty: Reborn"));
        assert_eq!(inspection.author.as_deref(), Some("SomeCreator"));
        assert_eq!(inspection.version.as_deref(), Some("1.4.2"));
        assert_eq!(inspection.campaign_type, CampaignType::Wol);
        // 最常见的 CCM 布局：元数据在子目录里，解压时必须剥离这一层
        assert_eq!(inspection.content_root, "Reborn");
        assert_eq!(inspection.map_count, 2);
        assert_eq!(inspection.mod_count, 1);
        assert_eq!(
            inspection.suggested_dir_name.as_deref(),
            Some("Wings of Liberty_ Reborn")
        );
    }

    #[test]
    fn refuses_archive_containing_traversal_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(
            dir.path(),
            "evil.zip",
            &[
                ("metadata.txt", b"title=Evil\ncampaign=WOL\n"),
                ("../../escaped.txt", b"gotcha"),
            ],
        );

        let inspection = inspect(&archive).expect("预检应成功");
        assert!(!inspection.installable, "含越界条目的包必须被拒绝");
        assert!(
            inspection
                .issues
                .iter()
                .any(|issue| issue.code == "UNSAFE_ENTRY")
        );
    }

    #[test]
    fn flags_package_without_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(dir.path(), "Plain Pack.zip", &[("maps/a.SC2Map", b"stub")]);

        let inspection = inspect(&archive).expect("预检应成功");
        assert_eq!(inspection.format, CampaignFormat::Plain);
        assert!(inspection.installable, "无元数据包仍可安装");
        assert!(
            inspection
                .issues
                .iter()
                .any(|issue| issue.code == "NO_METADATA")
        );
        // 没有 title 时退回压缩包文件名
        assert_eq!(inspection.suggested_dir_name.as_deref(), Some("Plain Pack"));
    }

    #[test]
    fn reports_non_archive_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("not-a-zip.zip");
        std::fs::write(&path, b"hello world").expect("write");

        let inspection = inspect(&path).expect("预检应成功");
        assert!(!inspection.installable);
        assert!(
            inspection
                .issues
                .iter()
                .any(|issue| issue.code == "NOT_AN_ARCHIVE")
        );
    }
}
