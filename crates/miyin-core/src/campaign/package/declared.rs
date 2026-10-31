//! 阶段 2：包里声明了什么（CCM metadata.txt / 弥音标准 metadata.json）。
//!
//! 只负责解析与校验元数据；推导（内容根、载荷、归属、目录名）都在别处，
//! 所以这里读完就能丢，除了 [Declared] 这一份「作者原话」。

use crate::campaign::contents;
use crate::campaign::metadata::{
    CampaignType, CcmMetadata, OverrideRule, PackageKind, StandardMetadata, decode_text,
};
use crate::campaign::{CampaignFormat, HealthIssue};

use super::extract::read_entry;
use super::paths::parent_prefix;
use super::types::{Entry, MIYIN_FORMAT_VERSION};

/// 包里声明了什么（元数据阶段的结果）。
///
/// 只装「作者写了什么」；推导出来的东西（内容根、载荷、归属、目录名）都不在这里。
pub(super) struct Declared {
    pub(super) format: CampaignFormat,
    /// 元数据所在目录；镜像包会被换成空串（见 resolve_content_root）。
    pub(super) content_root: String,
    pub(super) name: Option<String>,
    pub(super) author: Option<String>,
    pub(super) version: Option<String>,
    pub(super) description: Option<String>,
    /// 声明的主地图，反斜杠已归一成斜杠。
    pub(super) main_map: Option<String>,
    pub(super) doc: Option<String>,
    /// 声明依赖的模组，作者原样写的那份（还没归一化）。
    pub(super) mods_raw: Vec<String>,
    pub(super) modid: Option<String>,
    pub(super) cover: Option<String>,
    pub(super) tags: Vec<String>,
    pub(super) id: Option<String>,
    pub(super) kind: PackageKind,
    pub(super) requires: Vec<String>,
    pub(super) priority: Option<i64>,
    pub(super) overrides: Vec<OverrideRule>,
}

/// 把空字符串与纯空白视为「没有值」。
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// 阶段 2：元数据定位与解析。
///
/// 三选一：包根 metadata.json（弥音标准）-> 最浅的 metadata.txt / patch.txt（CCM）
/// -> 都没有，按无元数据包处理（仍然可装，只是没有作者与版本）。
pub(super) fn read_declarations(
    archive: &mut contents::Contents,
    entries: &[Entry],
    issues: &mut Vec<HealthIssue>,
) -> (Declared, CampaignType) {
    let standard_entry = entries.iter().find(|entry| {
        entry.relative.as_os_str() == "metadata.json"
            || entry.relative.as_os_str() == "Metadata.json"
    });

    // metadata.txt 是 CCM 的；patch.txt 是我们给**没有战役元数据的补丁**准备的同格式文件
    // （现实里的补丁往往只有一个说明.txt，作者想声明依赖就得有个地方写）。
    let ccm_entry = entries
        .iter()
        .filter(|entry| {
            entry.relative.file_name().is_some_and(|name| {
                name.eq_ignore_ascii_case("metadata.txt") || name.eq_ignore_ascii_case("patch.txt")
            })
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
    let mut declared_main_map: Option<String> = None;
    let mut declared_doc: Option<String> = None;
    let mut declared_mods_raw: Vec<String> = Vec::new();
    let mut declared_mod: Option<String> = None;
    let mut declared_cover = None;
    let mut declared_overrides = Vec::new();
    let mut declared_tags: Vec<String> = Vec::new();
    let mut declared_id: Option<String> = None;
    let mut declared_kind = PackageKind::Campaign;
    let mut declared_requires: Vec<String> = Vec::new();
    let mut declared_priority: Option<i64> = None;
    let mut content_root = String::new();
    let mut format = CampaignFormat::Plain;

    if let Some(entry) = standard_entry {
        format = CampaignFormat::Standard;
        content_root = parent_prefix(&entry.relative);
        if let Some(text) = read_entry(archive, entry.index) {
            match StandardMetadata::parse(&decode_text(&text)) {
                Ok(meta) => {
                    // 先借走扩展信息，后面几个字段会被移出
                    declared_main_map = meta.main_map_path();
                    declared_doc = meta.doc_path();
                    declared_mods_raw = meta.mods();
                    declared_mod = meta.modid().map(str::to_string);
                    declared_cover = clean(meta.cover_path().map(str::to_owned));
                    declared_overrides = meta
                        .miyin
                        .as_ref()
                        .map(|extensions| extensions.overrides.clone())
                        .unwrap_or_default();
                    declared_tags = meta.tags();
                    declared_id = clean(meta.id().map(str::to_owned));
                    declared_kind = meta.package_kind();
                    declared_requires = meta.patch_requires();
                    declared_priority = meta.patch_priority();
                    let extension_format =
                        meta.miyin.as_ref().and_then(|extensions| extensions.format);

                    name = clean(meta.name);
                    author = clean(meta.author);
                    version = clean(meta.version);
                    description = clean(meta.description);
                    campaign_raw = clean(meta.campaign).unwrap_or_default();

                    // 扩展格式版本比启动器新 -> 明确拒绝，而不是猜着解析
                    if let Some(format) = extension_format
                        && format > MIYIN_FORMAT_VERSION
                    {
                        issues.push(
                        HealthIssue::broken(
                            "FORMAT_TOO_NEW",
                            format!(
                                "该包使用了更新的扩展格式（v{format}），当前启动器只支持到 v{MIYIN_FORMAT_VERSION}"
                            ),
                        )
                        .with_hint("请升级弥音启动器后再导入"),
                    );
                    }

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
        if let Some(text) = read_entry(archive, entry.index) {
            let meta = CcmMetadata::parse(&decode_text(&text));
            name = clean(meta.title);
            author = clean(meta.author);
            version = clean(meta.version);
            description = clean(meta.description);
            campaign_raw = clean(meta.campaign).unwrap_or_default();
            declared_main_map = clean(meta.main_map.clone());
            declared_doc = clean(meta.doc.clone());
            declared_mods_raw = meta.mods.clone();
            declared_mod = clean(meta.modid.clone());
            declared_cover = clean(meta.cover);
            declared_tags = meta.tags.clone();
            declared_id = clean(meta.id.clone());
            declared_kind = meta
                .kind
                .as_deref()
                .map(PackageKind::parse)
                .unwrap_or(PackageKind::Campaign);
            declared_requires = meta.requires.clone();
            declared_priority = meta.priority;
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

    // 主地图：CCM 写 mainmap=，我们的 JSON 写 main_map
    let main_map = declared_main_map
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('\\', "/"));

    (
        Declared {
            format,
            content_root,
            name,
            author,
            version,
            description,
            main_map,
            doc: declared_doc,
            mods_raw: declared_mods_raw,
            modid: declared_mod,
            cover: declared_cover,
            tags: declared_tags,
            id: declared_id,
            kind: declared_kind,
            requires: declared_requires,
            priority: declared_priority,
            overrides: declared_overrides,
        },
        campaign_type,
    )
}
