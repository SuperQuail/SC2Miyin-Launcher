//! 星际枢纽兼容包：根 `metadata.json` 满足 SCNexus 标准 schema，内容按 `Maps/**` / `Mods/**` 保留。
//!
//! schema 依据（只读参考仓库，不猜）：
//!
//! - `reference/scnexus/packages/standard/src/metadata/metadata.type.ts`（字段与取值）
//! - `reference/scnexus/packages/app-main/src/modules/customize/compress-process.ts`
//!   （zip 根必须有 metadata.json，且 `manager === "SCNexus"`）
//! - `reference/scnexus/packages/app-main/src/modules/campaign/campaign-util.ts`
//!   （启用时只复制 `Maps/Campaign`、`Maps/<maps_directory>`、`Mods/<mods_directory>`）
//!
//! SCNexus 的 schema 表达不了的东西（封面、说明、覆盖规则、非数字注册 ID、未合并补丁…）
//! 一律进 `losses`，不静默丢。

use std::io::{Seek, Write};

use crate::campaign::metadata::CampaignType;
use crate::error::Result;
use crate::library::{Library, Variant};

use super::{
    ExportLoss, Plan, PlanFile, Written, add_path, find_by_source, loss, main_map_source,
    write_text, zip_error, zip_options,
};

/// SCNexus `metadata.json` 里的一条地图/模组。
#[derive(Debug, Clone)]
pub(super) struct Entry {
    name: String,
    relative_path: Option<String>,
    components: bool,
}

/// 星际枢纽包的计划（metadata.json 的内容 + 布局结论）。
#[derive(Debug, Clone)]
pub(super) struct ScnexusPlan {
    /// 地图所在目录（相对 `Maps/`，如 `Campaign`）——SCNexus 只认一个。
    maps_directory: String,
    /// 模组所在目录（相对 `Mods/`）。空串 = 模组就在 `Mods/` 根下。
    mods_directory: String,
    maps: Vec<Entry>,
    mods: Vec<Entry>,
    /// 数字注册 ID -> `snid`；非数字不写。
    snid: Option<u64>,
    /// `campaign` 字段（WOL / HOTS / LOTV / NCO）。
    campaign: Option<&'static str>,
    /// `luancher.map_name`。
    main_map: Option<String>,
    /// 这个格式能不能承载这份内容（自制战役为 `false`，仍然允许导出但会带 loss）。
    pub(super) supported: bool,
}

impl ScnexusPlan {
    /// 写进 metadata.json 的地图条数。
    pub(super) fn maps(&self) -> &[Entry] {
        &self.maps
    }

    /// 写进 metadata.json 的模组条数。
    pub(super) fn mods(&self) -> &[Entry] {
        &self.mods
    }
}

/// 算星际枢纽包的布局与丢失清单。
pub(super) fn plan(
    library: &Library,
    kind: &CampaignType,
    slot_slug: &str,
    variant: &Variant,
    files: &[PlanFile],
    merge_patches: bool,
    patches: &[String],
) -> (ScnexusPlan, Vec<ExportLoss>) {
    let mut losses = Vec::new();

    // 自制战役不属于 SCNexus 标准 Campaign 包（见 compress-process.ts 的安装路径判断）。
    let custom = kind.is_custom();
    if custom {
        losses.push(loss(
            "custom_campaign",
            "自制战役",
            "星际枢纽标准 Campaign 包只支持官方四大战役的改版；自制战役不会被识别，也不会自动铺进游戏目录",
        ));
    }

    let campaign = scnexus_campaign(slot_slug, variant.target_sub.as_deref());

    let mut snid = None;
    if let Some(id) = variant.registration_id.as_deref() {
        match id.trim().parse::<u64>() {
            Ok(number) => snid = Some(number),
            Err(_) => losses.push(loss(
                "registration_id",
                "注册 ID",
                format!("星际枢纽的 snid 只接受数字，「{id}」不是数字，不会带过去"),
            )),
        }
    }

    if variant.cover.is_some() {
        losses.push(loss(
            "cover",
            "封面图",
            "星际枢纽元数据没有封面字段，封面不会随包带走",
        ));
    }
    if variant.doc.is_some() {
        losses.push(loss(
            "doc",
            "说明文档",
            "星际枢纽元数据没有说明文档字段，说明文档不会随包带走",
        ));
    }
    if !variant.overrides.is_empty() {
        losses.push(loss(
            "overrides",
            "覆盖规则",
            format!(
                "{} 条覆盖规则已经作用在导出的落点上，但规则本身写不进星际枢纽元数据",
                variant.overrides.len()
            ),
        ));
    }
    if variant.mounted_mods.is_some() {
        losses.push(loss(
            "mounted_mods",
            "挂载勾选",
            "星际枢纽不保存模组勾选配置；导入后按全部挂载处理",
        ));
    }
    if !merge_patches && !patches.is_empty() {
        losses.push(loss(
            "patches",
            "未合并的补丁",
            format!(
                "补丁「{}」没有合并进包；勾选「包含补丁」后才会合并",
                patches.join("、")
            ),
        ));
    }

    // 只写安全的包内路径：覆盖规则来自作者，不能让它造出 `..` / 盘符条目。
    let mut safe: Vec<(&PlanFile, String)> = Vec::with_capacity(files.len());
    for file in files {
        match safe_target(&file.target) {
            Some(name) => safe.push((file, name)),
            None => losses.push(loss(
                "unsafe_path",
                "路径不安全",
                format!("「{}」不是安全的包内路径，已跳过", file.target),
            )),
        }
    }

    // ---- 地图：SCNexus 只认一个 maps_directory ----
    let mut map_roots: Vec<String> = Vec::new();
    for (_, name) in &safe {
        if let Some(root) = map_root(name)
            && !map_roots.contains(&root)
        {
            map_roots.push(root);
        }
    }
    // 官方改版的地图一定在 Maps/Campaign 下；没有才退回第一个根（例如自制战役的 CustomCampaigns）。
    let maps_directory = if map_roots.iter().any(|root| root == "Campaign") {
        "Campaign".to_string()
    } else {
        map_roots
            .first()
            .cloned()
            .unwrap_or_else(|| "Campaign".to_string())
    };

    let prefix = format!("Maps/{maps_directory}/");
    let mut maps: Vec<Entry> = Vec::new();
    for (file, name) in &safe {
        let Some(root) = map_root(name) else {
            continue;
        };
        if root != maps_directory {
            losses.push(loss(
                "map_root",
                "地图目录",
                format!("「{name}」不在 maps_directory「{maps_directory}」下，星际枢纽不会铺它"),
            ));
            continue;
        }
        if !is_map_name(name) {
            // Maps/ 下还会有说明、配置文件：SCNexus 整目录复制会带走，不用写进 maps[]
            continue;
        }
        let Some(rest) = name.strip_prefix(&prefix) else {
            continue;
        };
        maps.push(entry(rest, file.expanded));
    }

    // ---- 模组：mods_directory 用空串 ----
    // 地图里写死了 `Mods\X.SC2Mod` 这种相对游戏根的路径，模组必须留在 `Mods/` 原位。
    // `mods_directory = ""` 让 SCNexus 整目录复制 `Mods/`，各模组的子目录用 relative_path 表达。
    let mut mods: Vec<Entry> = Vec::new();
    for (file, name) in &safe {
        if !name.starts_with("Mods/") || !is_mod_name(name) {
            continue;
        }
        let Some(rest) = name.strip_prefix("Mods/") else {
            continue;
        };
        mods.push(entry(rest, file.expanded));
    }

    // 不在 Maps/ 或 Mods/ 下的内容：包里有，但 SCNexus 铺盘时不会碰
    for (_, name) in &safe {
        if !name.starts_with("Maps/") && !name.starts_with("Mods/") {
            losses.push(loss(
                "payload_path",
                "落点不受支持",
                format!("「{name}」不在 Maps/ 或 Mods/ 下，星际枢纽不会铺它"),
            ));
        }
    }

    // ---- 启动地图 ----
    let main_map = match variant.main_map.as_deref() {
        None => None,
        Some(declared) => {
            let found = main_map_source(library, slot_slug, variant).and_then(|source| {
                let file = find_by_source(files, &source)?;
                let name = safe_target(&file.target)?;
                let rest = name.strip_prefix(&prefix)?;
                is_map_name(rest).then(|| rest.to_string())
            });
            if found.is_none() {
                losses.push(loss(
                    "main_map",
                    "主地图",
                    format!(
                        "声明的主地图「{declared}」不在 maps_directory「{maps_directory}」下，luancher.map_name 不会写"
                    ),
                ));
            }
            found
        }
    };

    (
        ScnexusPlan {
            maps_directory,
            mods_directory: String::new(),
            maps,
            mods,
            snid,
            campaign,
            main_map,
            supported: !custom,
        },
        losses,
    )
}

/// 真正写星际枢纽包。
pub(super) fn write<W: Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    variant: &Variant,
    plan: &Plan,
) -> Result<Written> {
    let scnexus = plan.scnexus.as_ref().expect("星际枢纽导出计划");
    let mut files = 0;
    let mut expanded = Vec::new();

    write_text(zip, "metadata.json", &metadata_json(variant, scnexus))?;
    files += 1;

    // 目录条目即使为空也要有：SCNexus 启用时会整目录复制，源目录不存在会失败。
    add_directory(zip, "Maps/Campaign")?;
    if scnexus.maps_directory != "Campaign" {
        add_directory(zip, &format!("Maps/{}", scnexus.maps_directory))?;
    }
    add_directory(zip, "Mods")?;

    // 内容按游戏目录内的落点原样保留（SCNexus 解压后按目录整块复制）
    for file in &plan.files {
        let Some(name) = safe_target(&file.target) else {
            continue;
        };
        if file.expanded {
            expanded.push(name.clone());
        }
        add_path(zip, &file.source, &name, &mut files)?;
    }

    Ok(Written { files, expanded })
}

/// SCNexus 的 `metadata.json`。
fn metadata_json(variant: &Variant, plan: &ScnexusPlan) -> String {
    let maps: Vec<serde_json::Value> = plan
        .maps
        .iter()
        .map(|entry| {
            let mut value = serde_json::json!({ "name": entry.name });
            if let Some(relative) = &entry.relative_path {
                value["relative_path"] = serde_json::json!(relative);
            }
            if entry.components {
                value["components"] = serde_json::json!(true);
            }
            value
        })
        .collect();

    let mods: Vec<serde_json::Value> = plan
        .mods
        .iter()
        .map(|entry| {
            let mut value = serde_json::json!({ "name": entry.name });
            if let Some(relative) = &entry.relative_path {
                value["relative_path"] = serde_json::json!(relative);
            }
            if entry.components {
                value["components"] = serde_json::json!(true);
            }
            value
        })
        .collect();

    let mut document = serde_json::json!({
        "name": variant.name,
        "description": variant.description.clone().unwrap_or_default(),
        "version": variant.version.clone().unwrap_or_default(),
        "author": variant.author.clone().unwrap_or_default(),
        "type": "Campaign",
        "maps_directory": plan.maps_directory,
        "maps": maps,
        "mods_directory": plan.mods_directory,
        "mods": mods,
        "manager": "SCNexus",
        "manager_mode": "standard",
    });

    if !variant.tags.is_empty() {
        document["tags"] = serde_json::json!(variant.tags);
    }
    if let Some(campaign) = plan.campaign {
        document["campaign"] = serde_json::json!(campaign);
    }
    if let Some(snid) = plan.snid {
        document["snid"] = serde_json::json!(snid);
    }
    // 参考实现这个字段就拼成 luancher（少一个 n）——照抄，不然它读不到。
    if let Some(main_map) = &plan.main_map {
        document["luancher"] = serde_json::json!({ "map_name": main_map });
    }

    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string())
}

/// 把游戏目录内的落点转成 zip 内的安全路径；不安全时返回 `None`。
pub(super) fn safe_target(target: &str) -> Option<String> {
    let normalized = target.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in normalized.split('/') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if part == "." || part == ".." || part.contains(':') {
            return None;
        }
        parts.push(part);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// `Maps/` 之后的第一个目录（如 `Campaign`）；不在 `Maps/` 下返回 `None`。
fn map_root(target: &str) -> Option<String> {
    let rest = target.strip_prefix("Maps/")?;
    let first = rest.split('/').next().unwrap_or(rest);
    Some(first.to_string())
}

/// 把 `Maps/<dir>/`（或 `Mods/`）之后的相对路径拆成 relative_path + name。
fn entry(rest: &str, components: bool) -> Entry {
    match rest.rsplit_once('/') {
        Some((relative, name)) => Entry {
            name: name.to_string(),
            relative_path: Some(relative.to_string()),
            components,
        },
        None => Entry {
            name: rest.to_string(),
            relative_path: None,
            components,
        },
    }
}

fn is_map_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".sc2map")
}

fn is_mod_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".sc2mod")
}

/// SCNexus 的 campaign 只有四个取值；进化 / 序章归到父战役。
fn scnexus_campaign(slot_slug: &str, target_sub: Option<&str>) -> Option<&'static str> {
    match target_sub {
        Some("swarm") | Some("swarm/evolution") => Some("HOTS"),
        Some("void") | Some("voidprologue") => Some("LOTV"),
        Some("nova") => Some("NCO"),
        _ => match crate::library::require_slot(slot_slug).ok()?.parent() {
            CampaignType::Wol => Some("WOL"),
            CampaignType::Hots => Some("HOTS"),
            CampaignType::Lotv => Some("LOTV"),
            CampaignType::Nova => Some("NCO"),
            _ => None,
        },
    }
}

/// 写一个目录条目（zip 内以 `/` 结尾）。
fn add_directory<W: Write + Seek>(zip: &mut zip::ZipWriter<W>, name: &str) -> Result<()> {
    zip.add_directory(format!("{}/", name.trim_end_matches('/')), zip_options())
        .map_err(zip_error)
}
