//! 导出：弥音包保真回读、星际枢纽包的 schema/布局，以及 loss 清单。

use std::path::{Path, PathBuf};

use super::fixture::*;
use crate::library::export::{ExportFormat, ExportOptions, export, preview};
use crate::library::{ImportMode, Library, import, patch};

/// 造一个根 metadata.json（我们的标准格式）的包。
fn standard_package(
    fixture: &Fixture,
    name: &str,
    metadata: &str,
    extra: &[(&str, &str)],
) -> PathBuf {
    let mut files: Vec<(String, String)> =
        vec![("metadata.json".to_string(), metadata.to_string())];
    for (path, content) in extra {
        files.push((path.to_string(), content.to_string()));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();
    build_zip(fixture.work.path(), name, &borrowed)
}

fn read_zip_names(path: &Path) -> Vec<String> {
    let file = std::fs::File::open(path).expect("open zip");
    let mut zip = zip::ZipArchive::new(file).expect("zip");
    (0..zip.len())
        .filter_map(|index| {
            zip.by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .collect()
}

fn read_zip_json(path: &Path, name: &str) -> serde_json::Value {
    let file = std::fs::File::open(path).expect("open zip");
    let mut zip = zip::ZipArchive::new(file).expect("zip");
    let mut text = String::new();
    use std::io::Read;
    zip.by_name(name)
        .expect("entry exists")
        .read_to_string(&mut text)
        .expect("read entry");
    serde_json::from_str(&text).expect("json")
}

fn loss_kinds(losses: &[crate::library::export::ExportLoss]) -> Vec<&str> {
    losses.iter().map(|item| item.kind.as_str()).collect()
}

/// 弥音包：根 metadata.json + metadata.txt + Miyin/ 附加数据；导入回来元数据要保真。
#[test]
fn miyin_export_round_trips_metadata() {
    let fixture = fixture();
    let metadata = r#"{
        "name": "重生之遗",
        "description": "测试用改版",
        "version": "1.2.3",
        "author": "作者甲",
        "type": "Campaign",
        "campaign": "WOL",
        "cover": "cover.png",
        "doc": "manual.pdf",
        "main_map": "a.SC2Map",
        "id": "ABC.reborn",
        "miyin": {
            "format": 3,
            "tags": ["重制", "战役"],
            "mods": ["Kit"],
            "overrides": [{ "from": "extra", "to": "Maps" }]
        }
    }"#;
    let package = standard_package(
        &fixture,
        "reborn.zip",
        metadata,
        &[
            ("a.SC2Map", "地图"),
            ("Kit.SC2Mod", "模组"),
            ("cover.png", "PNG"),
            ("manual.pdf", "PDF"),
        ],
    );
    let variant = import(&fixture.library, &package, "wol", ImportMode::Rename).expect("import");

    let destination = fixture.work.path().join("弥音导出.zip");
    let report = export(
        &fixture.library,
        "wol",
        &variant,
        &ExportOptions {
            destination: destination.clone(),
            merge_patches: false,
            format: ExportFormat::Miyin,
        },
    )
    .expect("export miyin");

    assert_eq!(report.format, ExportFormat::Miyin);
    assert_eq!(report.maps, 1);
    assert_eq!(report.mods, 1);
    assert!(
        report.losses.is_empty(),
        "封面/说明/主地图都在库里，不该有丢失：{:?}",
        report.losses
    );

    let names = read_zip_names(&destination);
    for wanted in [
        "metadata.txt",
        "metadata.json",
        "a.SC2Map",
        "Kit.SC2Mod",
        "Miyin/cover.png",
        "Miyin/manual.pdf",
        "Miyin/metadata.json",
        "Miyin/bindings.json",
    ] {
        assert!(
            names.iter().any(|name| name == wanted),
            "缺少 {wanted}: {names:?}"
        );
    }

    // 导入到一个全新的库里：元数据必须能读回来
    let dir = tempfile::tempdir().expect("临时库");
    let fresh = Library::new(dir.path().join("data"));
    let back = import(&fresh, &destination, "wol", ImportMode::Rename).expect("re-import");

    assert_eq!(back.name, variant.name);
    assert_eq!(back.author, variant.author);
    assert_eq!(back.version, variant.version);
    assert_eq!(back.description, variant.description);
    assert_eq!(back.registration_id, variant.registration_id);
    assert_eq!(back.tags, variant.tags);
    assert_eq!(back.declared_mods, variant.declared_mods);
    assert_eq!(back.overrides, variant.overrides);
    assert_eq!(back.main_map, variant.main_map);
    assert!(
        back.cover
            .as_deref()
            .is_some_and(|cover| cover.ends_with("cover.png")),
        "封面要能读回：{:?}",
        back.cover
    );
    assert!(
        back.doc
            .as_deref()
            .is_some_and(|doc| doc.ends_with("manual.pdf")),
        "说明文档要能读回：{:?}",
        back.doc
    );
    assert_eq!(back.map_count, 1);
    assert_eq!(back.mod_count, 1);
}

/// 星际枢纽包：metadata.json 满足 SCNexus schema，内容按 Maps/ Mods/ 布局。
#[test]
fn scnexus_export_writes_standard_metadata_and_layout() {
    let fixture = fixture();
    let campaign = build_zip(
        fixture.work.path(),
        "golden.zip",
        &[
            (
                "metadata.txt",
                "title=黄金之遗\ncampaign=WOL\nauthor=HTXL\nversion=1.32\n",
            ),
            ("a.SC2Map", "地图A"),
            ("b.SC2Map", "地图B"),
            ("Kit.SC2Mod", "模组"),
        ],
    );
    let variant = import(&fixture.library, &campaign, "wol", ImportMode::Rename).expect("import");
    fixture
        .library
        .set_main_map("wol", &variant.id, Some("a.SC2Map"))
        .expect("main map");
    let variant = fixture
        .library
        .variant("wol", &variant.id)
        .expect("variant");

    let destination = fixture.work.path().join("枢纽导出.zip");
    let report = export(
        &fixture.library,
        "wol",
        &variant,
        &ExportOptions {
            destination: destination.clone(),
            merge_patches: false,
            format: ExportFormat::Scnexus,
        },
    )
    .expect("export scnexus");

    assert_eq!(report.format, ExportFormat::Scnexus);
    assert_eq!(report.maps, 2);
    assert_eq!(report.mods, 1);
    assert!(report.losses.is_empty(), "{:?}", report.losses);

    let names = read_zip_names(&destination);
    for wanted in [
        "metadata.json",
        "Maps/Campaign/a.SC2Map",
        "Maps/Campaign/b.SC2Map",
        "Mods/Kit.SC2Mod",
    ] {
        assert!(
            names.iter().any(|name| name == wanted),
            "缺少 {wanted}: {names:?}"
        );
    }
    assert!(
        !names.iter().any(|name| name.starts_with("Miyin/")),
        "星际枢纽包里不该出现我们的附加目录：{names:?}"
    );

    let metadata = read_zip_json(&destination, "metadata.json");
    assert_eq!(metadata["manager"], "SCNexus");
    assert_eq!(metadata["manager_mode"], "standard");
    assert_eq!(metadata["type"], "Campaign");
    assert_eq!(metadata["campaign"], "WOL");
    assert_eq!(metadata["name"], "黄金之遗");
    assert_eq!(metadata["author"], "HTXL");
    assert_eq!(metadata["version"], "1.32");
    assert_eq!(metadata["maps_directory"], "Campaign");
    assert_eq!(metadata["mods_directory"], "");

    let maps = metadata["maps"].as_array().expect("maps 数组");
    let map_names: Vec<&str> = maps.iter().filter_map(|map| map["name"].as_str()).collect();
    assert_eq!(map_names, vec!["a.SC2Map", "b.SC2Map"]);
    let mods = metadata["mods"].as_array().expect("mods 数组");
    assert_eq!(mods.len(), 1);
    assert_eq!(mods[0]["name"], "Kit.SC2Mod");
    assert_eq!(metadata["luancher"]["map_name"], "a.SC2Map");
}

/// 星际枢纽包表达不了的东西逐条进 losses；预览不写盘，导出也不静默丢内容。
#[test]
fn scnexus_preview_lists_losses_instead_of_dropping_content() {
    let fixture = fixture();
    let metadata = r#"{
        "name": "带附加信息的改版",
        "description": "测试",
        "version": "1.0",
        "author": "作者乙",
        "type": "Campaign",
        "campaign": "WOL",
        "cover": "cover.png",
        "doc": "manual.pdf",
        "id": "XYZ.reborn",
        "miyin": {
            "format": 3,
            "overrides": [{ "from": "x.SC2Map", "to": "Interfaces" }]
        }
    }"#;
    let package = standard_package(
        &fixture,
        "styled.zip",
        metadata,
        &[
            ("x.SC2Map", "地图"),
            ("cover.png", "PNG"),
            ("manual.pdf", "PDF"),
        ],
    );
    let variant = import(&fixture.library, &package, "wol", ImportMode::Rename).expect("import");
    fixture
        .library
        .set_mounted_mods("wol", &variant.id, &[])
        .expect("mounted config");

    // 挂一个补丁（不合并 -> 补丁内容进不了星际枢纽包）
    let patch_zip = build_zip(
        fixture.work.path(),
        "p.zip",
        &[("Mods/Extra.SC2Mod", "补丁内容")],
    );
    let patch = patch::import_patch(&fixture.library, &patch_zip).expect("patch");
    patch::bind(&fixture.library, "wol", &patch.id).expect("bind");

    let variant = fixture
        .library
        .variant("wol", &variant.id)
        .expect("variant");

    let preview = preview(
        &fixture.library,
        "wol",
        &variant,
        false,
        ExportFormat::Scnexus,
    )
    .expect("preview");
    assert!(preview.supported, "官方战役的改版仍受支持，只是带 loss");
    let kinds = loss_kinds(&preview.losses);
    for wanted in [
        "cover",
        "doc",
        "overrides",
        "registration_id",
        "mounted_mods",
        "patches",
        "payload_path",
    ] {
        assert!(
            kinds.contains(&wanted),
            "缺少 loss「{wanted}」：{:?}",
            preview.losses
        );
    }

    let destination = fixture.work.path().join("枢纽-loss.zip");
    let report = export(
        &fixture.library,
        "wol",
        &variant,
        &ExportOptions {
            destination: destination.clone(),
            merge_patches: false,
            format: ExportFormat::Scnexus,
        },
    )
    .expect("export");
    assert_eq!(
        loss_kinds(&report.losses),
        loss_kinds(&preview.losses),
        "导出与预览的 loss 必须一致"
    );

    // 落到 Maps/ 之外的载荷不进 metadata，但**内容仍然在包里**（不许静默删）
    let names = read_zip_names(&destination);
    assert!(
        names.iter().any(|name| name == "Interfaces/x.SC2Map"),
        "内容不能静默丢：{names:?}"
    );
}

/// 自制战役不属于星际枢纽标准 Campaign 包：预览明确标 unsupported，但仍允许导出（带 loss）。
#[test]
fn custom_campaign_is_unsupported_for_scnexus_but_still_exportable() {
    let fixture = fixture();
    let campaign = build_zip(
        fixture.work.path(),
        "custom.zip",
        &[
            (
                "metadata.txt",
                "title=自制战役\ncampaign=custom\nversion=1.0\n",
            ),
            ("Map.SC2Map", "地图"),
        ],
    );
    let variant =
        import(&fixture.library, &campaign, "custom", ImportMode::Rename).expect("import");

    let scnexus = preview(
        &fixture.library,
        "custom",
        &variant,
        false,
        ExportFormat::Scnexus,
    )
    .expect("preview scnexus");
    assert!(!scnexus.supported, "自制战役不是 SCNexus 标准 Campaign 包");
    assert!(loss_kinds(&scnexus.losses).contains(&"custom_campaign"));

    let miyin = preview(
        &fixture.library,
        "custom",
        &variant,
        false,
        ExportFormat::Miyin,
    )
    .expect("preview miyin");
    assert!(miyin.supported);
    assert!(
        !loss_kinds(&miyin.losses).contains(&"custom_campaign"),
        "弥音包没有这种限制：{:?}",
        miyin.losses
    );
}

/// 空内容直接报错，不生成一个打不开的 zip。
///
/// 导入层不允许空包，所以这里直接往索引里放一条没有载荷的记录。
#[test]
fn exporting_an_empty_variant_is_rejected() {
    use crate::library::{CampaignSlot, LibraryIndex};

    let dir = tempfile::tempdir().expect("临时库");
    let library = Library::new(dir.path().join("data"));
    let variant = crate::library::Variant {
        id: "empty".to_string(),
        name: "空包".to_string(),
        ..Default::default()
    };
    let mut index = LibraryIndex::default();
    index.slots.insert(
        "wol".to_string(),
        CampaignSlot {
            variants: vec![variant.clone()],
            active: None,
        },
    );
    library.save_index(&index).expect("save index");

    let error = export(
        &library,
        "wol",
        &variant,
        &ExportOptions {
            destination: dir.path().join("empty-out.zip"),
            merge_patches: false,
            format: ExportFormat::Miyin,
        },
    )
    .expect_err("空版本必须拒绝导出");
    assert!(error.to_string().contains("没有可导出的内容"), "{error}");
}
