//! 弥音包：CCM 平铺布局 + 根 `metadata.json`（我们的标准格式）+ `Miyin/` 附加数据。
//!
//! 目录形状（就是真实 CCM 包 `黄金之遗(CCM).zip` 的样子，外加根 metadata.json）：
//!
//! `@text
//! <战役名>.zip
//! ├── metadata.txt          <- CCM 读这个（键=值）
//! ├── metadata.json         <- 我们读这个（标准 schema + miyin 扩展，读回时保真）
//! ├── paiur01.SC2Map        <- 平铺的地图
//! ├── HTXL.SC2Mod           <- 平铺的模组
//! └── Miyin/                <- 我们的附加数据，CCM 不读也不冲突
//!     ├── metadata.json     <- 旧版附加元数据（保留，兼容旧导出物）
//!     ├── cover.png
//!     ├── 说明.pdf
//!     ├── bindings.json
//!     └── patches/<id>/…    <- 附加模式下的补丁原件
//! `@
//!
//! 关键点：地图/模组平铺在顶层（CCM 读得懂），我们自己的数据在 `Miyin/`；
//! 根 `metadata.json` 用我们的标准 schema，启动器读回时按它还原
//! 注册 ID、标签、封面、说明、主地图与覆盖规则。

use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

use crate::campaign::metadata::{CampaignType, PackageKind};
use crate::campaign::package::{MIYIN_FORMAT_VERSION, PayloadTarget};
use crate::error::Result;
use crate::library::{Library, Variant, compose, now_seconds};

use super::{
    ExportLoss, Plan, PlanFile, Written, add_path, add_tree, file_name, find_by_source, loss,
    main_map_source, unique_name, write_text,
};

/// 我们自己的数据放这个目录，避免和 CCM 抢位置。
pub const MIYIN_DIR: &str = "Miyin";

/// 弥音包特有的附加数据计划。
#[derive(Debug, Clone)]
pub(super) struct MiyinPlan {
    /// 封面图（写进 `Miyin/`）。
    cover: Option<ZipExtra>,
    /// 说明文档（写进 `Miyin/`）。
    doc: Option<ZipExtra>,
    /// 主地图在包内的平铺文件名。
    main_map: Option<String>,
}

/// 一个要写进 `Miyin/` 的文件。
#[derive(Debug, Clone)]
pub(super) struct ZipExtra {
    source: PathBuf,
    /// `Miyin/` 里的文件名（已去重）。
    name: String,
}

/// 算弥音包要额外带什么、丢什么。
pub(super) fn plan(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    files: &[PlanFile],
) -> (MiyinPlan, Vec<ExportLoss>) {
    let mut losses = Vec::new();
    let variant_dir = library.slot_dir(slot_slug).join(&variant.id);

    // `Miyin/` 里的名字空间：元数据、绑定、补丁目录先占住
    let mut used: Vec<String> = vec![
        "metadata.json".to_string(),
        "bindings.json".to_string(),
        "patches".to_string(),
    ];

    let cover = extra_file(
        &variant_dir,
        variant.cover.as_deref(),
        "cover",
        "封面图",
        &mut used,
        &mut losses,
    );
    let doc = extra_file(
        &variant_dir,
        variant.doc.as_deref(),
        "doc",
        "说明文档",
        &mut used,
        &mut losses,
    );

    let main_map = match main_map_source(library, slot_slug, variant) {
        None => None,
        Some(source) => match find_by_source(files, &source) {
            Some(file) => Some(file.flat_name.clone()),
            None => {
                if let Some(declared) = variant.main_map.as_deref() {
                    losses.push(loss(
                        "main_map",
                        "主地图",
                        format!("声明的主地图「{declared}」不在导出内容里，导入后需要重新指定"),
                    ));
                }
                None
            }
        },
    };

    (
        MiyinPlan {
            cover,
            doc,
            main_map,
        },
        losses,
    )
}

/// 把一个声明的附加文件收进计划；文件不在库里就记一条丢失。
fn extra_file(
    variant_dir: &Path,
    declared: Option<&str>,
    kind: &str,
    label: &str,
    used: &mut Vec<String>,
    losses: &mut Vec<ExportLoss>,
) -> Option<ZipExtra> {
    let declared = declared?;
    let source = variant_dir.join(declared.replace('\\', "/"));
    if !source.is_file() {
        losses.push(loss(
            kind,
            label,
            format!("「{declared}」在库里的文件已经不在了，不会随包导出"),
        ));
        return None;
    }
    let name = unique_name(used, &file_name(&source));
    Some(ZipExtra { source, name })
}

/// 真正写弥音包。
pub(super) fn write<W: Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    plan: &Plan,
) -> Result<Written> {
    let miyin = plan.miyin.as_ref().expect("弥音导出计划");
    let mut files = 0;
    let mut expanded = Vec::new();

    // CCM 读的那份（键=值，保持原有形态）
    write_text(zip, "metadata.txt", &ccm_metadata(variant))?;
    // 我们自己的标准元数据：启动器读回时靠它还原
    write_text(
        zip,
        "metadata.json",
        &root_metadata(slot_slug, variant, miyin),
    )?;
    files += 1;

    // ---- 顶层：CCM 读得懂的那部分（平铺） ----
    for item in &plan.files {
        if item.expanded {
            expanded.push(item.flat_name.clone());
        }
        add_path(zip, &item.source, &item.flat_name, &mut files)?;
    }

    // ---- Miyin/：我们自己的数据 ----
    write_text(
        zip,
        &format!("{MIYIN_DIR}/metadata.json"),
        &legacy_metadata(slot_slug, variant),
    )?;
    files += 1;

    if let Some(cover) = &miyin.cover {
        add_path(
            zip,
            &cover.source,
            &format!("{MIYIN_DIR}/{}", cover.name),
            &mut files,
        )?;
    }
    if let Some(doc) = &miyin.doc {
        add_path(
            zip,
            &doc.source,
            &format!("{MIYIN_DIR}/{}", doc.name),
            &mut files,
        )?;
    }

    write_text(
        zip,
        &format!("{MIYIN_DIR}/bindings.json"),
        &bindings_json(library, slot_slug),
    )?;
    files += 1;

    // 补丁原件：两种模式都带上，导入方可以继续单独开关
    for (_, patch) in compose::bindings_of(library, slot_slug) {
        let root = library.patch_dir(&patch.id);
        if !root.is_dir() {
            continue;
        }
        let prefix = format!("{MIYIN_DIR}/patches/{}", patch.id);
        add_tree(zip, &root, &prefix, &mut files)?;
    }

    Ok(Written { files, expanded })
}

/// 根 `metadata.json`：我们的标准 schema + `miyin` 扩展。
fn root_metadata(slot_slug: &str, variant: &Variant, miyin: &MiyinPlan) -> String {
    let kind = match variant.kind {
        PackageKind::Campaign => "campaign",
        PackageKind::Patch => "patch",
    };

    let extensions = serde_json::json!({
        "format": MIYIN_FORMAT_VERSION,
        "kind": kind,
        "id": variant.registration_id,
        "main_map": miyin.main_map,
        "tags": variant.tags,
        "cover": miyin.cover.as_ref().map(|file| format!("{MIYIN_DIR}/{}", file.name)),
        "doc": miyin.doc.as_ref().map(|file| format!("{MIYIN_DIR}/{}", file.name)),
        // 覆盖规则要原样带出去，否则「导入 -> 导出 -> 再导入」会丢掉落点
        "overrides": variant.overrides,
        // 作者声明的依赖模组（不是当前勾选状态）
        "mods": variant.declared_mods,
    });

    let document = serde_json::json!({
        "name": variant.name,
        "description": variant.description,
        "version": variant.version,
        "author": variant.author,
        "type": "Campaign",
        "campaign": campaign_value(slot_slug, variant.target_sub.as_deref()),
        "miyin": extensions,
        "exported_at": now_seconds(),
        "exported_by": "MiYin Launcher",
        "note": "根 metadata.json 是弥音标准元数据，导入时优先于 metadata.txt；CCM 及其他工具可以忽略它。",
    });

    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string())
}

/// 旧版附加元数据（保留原字段，兼容旧导出物与手工排查）。
fn legacy_metadata(slot_slug: &str, variant: &Variant) -> String {
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
            "format": MIYIN_FORMAT_VERSION,
            "id": variant.registration_id,
            "tags": variant.tags,
            "overrides": variant.overrides,
        },
        "cover": variant.cover,
        "doc": variant.doc,
        "main_map": variant.main_map,
        "mounted_mods": variant.mounted_mods,
        "payloads": payloads,
        "note": "本目录是弥音启动器的附加数据。CCM 及其他工具可以完全忽略它；启动器读回时会用它还原版本、注册 ID 与载荷落点。",
    });

    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string())
}

/// CCM 读的那份元数据（键=值）。
pub(super) fn ccm_metadata(variant: &Variant) -> String {
    let mut out = String::new();
    out.push_str(&format!("title={}\n", one_line(&variant.name)));
    if let Some(description) = &variant.description {
        out.push_str(&format!("desc={}\n", one_line(description)));
    }
    if let Some(author) = &variant.author {
        out.push_str(&format!("author={}\n", one_line(author)));
    }
    if let Some(campaign) = campaign_value_for_ccm(variant) {
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

/// 包内声明的 campaign 取值（CCM 元数据用；进化/序章保留细分写法）。
fn campaign_value_for_ccm(variant: &Variant) -> Option<&'static str> {
    variant.target_sub.as_deref().and_then(sub_to_campaign)
}

/// 主地图/资料片子目录 -> 包内的 campaign 取值。
pub(super) fn sub_to_campaign(sub: &str) -> Option<&'static str> {
    match sub {
        "swarm" => Some("HOTS"),
        "swarm/evolution" => Some("HOTSEVO"),
        "void" => Some("LOTV"),
        "voidprologue" => Some("LOTVPROLOGUE"),
        "nova" => Some("NCO"),
        _ => None,
    }
}

/// 导出根 metadata.json 里的 campaign 取值；认不出来时退回槽位对应的官方战役。
fn campaign_value(slot_slug: &str, target_sub: Option<&str>) -> Option<&'static str> {
    if let Some(sub) = target_sub
        && let Some(value) = sub_to_campaign(sub)
    {
        return Some(value);
    }
    match crate::library::require_slot(slot_slug).ok()?.parent() {
        CampaignType::Wol => Some("WOL"),
        CampaignType::Hots => Some("HOTS"),
        CampaignType::Lotv => Some("LOTV"),
        CampaignType::Nova => Some("NCO"),
        CampaignType::Custom => Some("CUSTOM"),
        _ => None,
    }
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

/// 元数据里的值不能带换行，否则会破坏 `键=值` 结构。
fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ").trim().to_string()
}
