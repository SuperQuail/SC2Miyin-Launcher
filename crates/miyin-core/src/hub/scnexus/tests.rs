//! hub::scnexus 的单元测试：扫描（含 CCM 过滤、active、坏输入）与转换。
//!
//! 夹具全部在 tempfile 里造 —— 不碰真实游戏目录，也不用真实战役包（那是游戏版权内容）。

use super::*;

/// 造一个 SCNexus 标准库战役目录（metadata.json + 目录本身）。
fn library_campaign(root: &Path, dir: &str, metadata: &serde_json::Value) -> PathBuf {
    let path = root.join(dir);
    std::fs::create_dir_all(&path).expect("建目录");
    std::fs::write(
        path.join("metadata.json"),
        serde_json::to_string_pretty(metadata).expect("序列化"),
    )
    .expect("写 metadata.json");
    path
}

/// 写一个文件，父目录不存在就建。
fn write_file(path: &Path, contents: &[u8]) {
    std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建父目录");
    std::fs::write(path, contents).expect("写文件");
}

/// 一份「标准库战役」的元数据：mods 是对象数组（我们自己的 StandardMetadata 认不了）。
fn sample_metadata(name: &str) -> serde_json::Value {
    serde_json::json!({
        "snid": 4321,
        "name": name,
        "description": "一段说明",
        "version": "1.2.3",
        "author": "某作者",
        "type": "Campaign",
        "campaign": "WOL",
        "maps_directory": "Rebel",
        "mods_directory": "Base",
        "maps": [{ "name": "L1.SC2Map" }],
        "mods": [
            { "name": "base.SC2Mod" },
            { "name": "extra.SC2Mod", "upstream": true }
        ],
        "luancher": { "map_name": "L1.SC2Map" }
    })
}

/// 造一个带内容的战役目录，并返回扫描出来的那一条。
fn sample_campaign(root: &Path) -> ScnexusCampaign {
    let dir = library_campaign(root, "Rebel Yell", &sample_metadata("Rebel Yell"));
    write_file(
        &dir.join("Maps").join("Rebel").join("L1.SC2Map"),
        b"map-bytes",
    );
    write_file(
        &dir.join("Maps").join("Campaign").join("S1.SC2Map"),
        b"swarm",
    );
    write_file(&dir.join("Mods").join("Base").join("base.SC2Mod"), b"mod");
    write_file(&dir.join("Mods").join("Base").join("extra.SC2Mod"), b"mod2");

    let mut list = scan(root, None, None, None);
    assert_eq!(list.len(), 1, "样例目录里应该正好一部战役");
    list.remove(0)
}

/// 元数据文件的路径字符串（store 里记的就是它）。
fn metadata_path_of(dir: &Path) -> String {
    dir.join("metadata.json").to_string_lossy().to_string()
}

#[test]
fn scans_library_campaign_with_object_shaped_mods() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("SCNexusCampaign");
    let one = sample_campaign(&root);

    assert_eq!(one.id, "4321", "snid 优先当稳定 id");
    assert_eq!(one.name, "Rebel Yell");
    assert_eq!(one.author.as_deref(), Some("某作者"));
    assert_eq!(one.version.as_deref(), Some("1.2.3"));
    assert_eq!(one.description.as_deref(), Some("一段说明"));
    assert_eq!(one.campaign, "wol");
    assert_eq!(one.source, Source::Library);
    assert!(!one.active);
    assert_eq!(one.maps, 2, "两张地图（含 Maps/Campaign 那张）");
    assert_eq!(one.mods, 2, "mods 是对象数组，也要数得到");
    assert!(one.size_bytes > 0);
    assert_eq!(PathBuf::from(&one.path), root.join("Rebel Yell"));
}

#[test]
fn marks_active_from_active_store_and_hashes_id_without_snid() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("SCNexusCampaign");
    let dir = library_campaign(
        &root,
        "Active One",
        &serde_json::json!({ "name": "Active One", "type": "Campaign", "campaign": "NCO" }),
    );

    let store = tmp.path().join("store-campaign-active.json");
    write_file(
        &store,
        serde_json::json!({
            "ININTIALIZED": true,
            "CAMPAIGN_SET": {
                "NCO": {
                    "name": "Active One",
                    "campaign": "NCO",
                    "localinfo": { "metadata_path": metadata_path_of(&dir) }
                }
            }
        })
        .to_string()
        .as_bytes(),
    );

    let list = scan(&root, None, None, Some(&store));
    assert_eq!(list.len(), 1);
    assert!(list[0].active, "active store 里点名的要标成启用中");
    assert_eq!(list[0].campaign, "nova", "NCO 映射到我们的 nova 槽位");
    assert_eq!(list[0].id.len(), 16, "没有 snid 就用路径哈希");
    assert_eq!(list[0].author, None);
}

#[test]
fn ccm_entries_come_from_the_campaign_store_only() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let game = tmp.path();
    let ccm_root = game.join("Maps").join("CustomCampaigns");

    let managed = ccm_root.join("Managed");
    write_file(
        &managed.join("metadata.txt"),
        "title=托管战役\ncampaign=HOTS".as_bytes(),
    );
    write_file(&managed.join("Managed01.SC2Map"), b"map");

    // 目录里躺着、但 store 没点名的 —— 不冒充「枢纽托管」
    let stranger = ccm_root.join("Stranger");
    write_file(&stranger.join("metadata.txt"), "title=陌生人".as_bytes());
    write_file(&stranger.join("Other.SC2Map"), b"map");

    let store = game.join("store-campaign.json");
    write_file(
        &store,
        serde_json::json!({
            "CAMPAIGN_LIST_SET": {
                "HOTS": [{
                    "name": "托管战役",
                    "type": "Campaign",
                    "campaign": "HOTS",
                    "version": "1.0",
                    "localinfo": {
                        "metadata_path": managed.join("metadata.txt").to_string_lossy(),
                    }
                }]
            }
        })
        .to_string()
        .as_bytes(),
    );

    let list = scan(
        &game.join("NoSuchLibrary"),
        Some(&ccm_root),
        Some(&store),
        None,
    );
    assert_eq!(list.len(), 1, "只列 store 里点名的那一个");
    assert_eq!(list[0].source, Source::Ccm);
    assert_eq!(list[0].campaign, "hots");
    assert_eq!(list[0].name, "托管战役");
    assert_eq!(list[0].maps, 1);
    assert!(!list.iter().any(|item| item.name == "陌生人"));
}

#[test]
fn converts_into_a_library_variant() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("SCNexusCampaign");
    let campaign = sample_campaign(&root);
    let library = Library::new(tmp.path().join("miyin_launcher_data"));

    let converted = convert(&library, &campaign, false).expect("转换");

    assert_eq!(converted.slot, "wol");
    assert_eq!(converted.variant.name, "Rebel Yell");
    assert_eq!(converted.variant.author.as_deref(), Some("某作者"));
    assert_eq!(converted.variant.version.as_deref(), Some("1.2.3"));
    assert_eq!(converted.variant.map_count, 2, "地图要跟着搬过来");
    assert_eq!(converted.variant.mod_count, 1, "一个文件夹算一个模组");
    assert_eq!(
        converted.variant.main_map.as_deref(),
        Some("Maps/Rebel/L1.SC2Map"),
        "枢纽声明的启动地图要落到正确的地图路径上"
    );
    assert!(
        converted
            .warnings
            .iter()
            .any(|line| line.contains("上游依赖")),
        "外部依赖要提示：{:?}",
        converted.warnings
    );

    // 库里真的落地了
    let variant_dir = library.slot_dir("wol").join(&converted.variant.id);
    assert!(variant_dir.join("metadata.json").is_file());
    assert!(
        variant_dir
            .join("Maps")
            .join("Rebel")
            .join("L1.SC2Map")
            .is_file()
    );

    // 源目录没要求删就留着
    assert!(PathBuf::from(&campaign.path).is_dir());

    // 临时目录要收干净
    let leftovers: Vec<String> = std::fs::read_dir(library.root())
        .expect("读库根")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".hub-staging"))
        .collect();
    assert!(leftovers.is_empty(), "暂存目录没删：{leftovers:?}");
}

#[test]
fn convert_removes_source_only_when_asked_and_not_active() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("SCNexusCampaign");
    let campaign = sample_campaign(&root);
    let source = PathBuf::from(&campaign.path);
    let library = Library::new(tmp.path().join("data"));

    // 没要求删：源目录要留着
    let kept = convert(&library, &campaign, false).expect("转换");
    assert!(source.is_dir());
    assert!(
        !kept.warnings.iter().any(|line| line.contains("没有删")),
        "{:?}",
        kept.warnings
    );

    // 要删、但枢纽那边正启用着：不删并说明
    let active = ScnexusCampaign {
        active: true,
        ..campaign.clone()
    };
    let skipped = convert(&library, &active, true).expect("转换");
    assert!(source.is_dir(), "启用中的源目录不能删");
    assert!(
        skipped.warnings.iter().any(|line| line.contains("正启用")),
        "{:?}",
        skipped.warnings
    );
    assert_eq!(skipped.slot, "wol");

    // 要删、也不再启用：删掉
    let removed = convert(&library, &campaign, true).expect("转换");
    assert!(!source.exists(), "源目录应该删掉了");
    assert!(
        !removed.warnings.iter().any(|line| line.contains("删")),
        "不该有删除相关的提示：{:?}",
        removed.warnings
    );
}

#[test]
fn bad_input_never_panics() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("nope");

    // 什么都没有：空表，不是错误
    assert!(scan(&missing, Some(&missing), Some(&missing), Some(&missing)).is_empty());

    // 坏 JSON 的 store + 坏 metadata 的目录：跳过
    let broken = tmp.path().join("store-campaign.json");
    write_file(&broken, b"{ not json");
    let root = tmp.path().join("SCNexusCampaign");
    write_file(&root.join("Bad").join("metadata.json"), b"[1, 2, 3]");
    assert!(scan(&root, None, Some(&broken), Some(&broken)).is_empty());

    // 只有 Customize（模组，不是战役）：不列
    library_campaign(
        &root,
        "Just a Mod",
        &serde_json::json!({ "name": "Just a Mod", "type": "Customize" }),
    );
    assert!(scan(&root, None, None, None).is_empty());

    // 认不出资料片的：不列（枢纽只支持官方四部）
    library_campaign(
        &root,
        "Unknown",
        &serde_json::json!({ "name": "Unknown", "type": "Campaign", "campaign": "???" }),
    );
    assert!(scan(&root, None, None, None).is_empty());
}
