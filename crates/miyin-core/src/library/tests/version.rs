//! 版本比较、导入冲突、覆盖更新与重命名后导入。

use super::fixture::*;
use crate::library::{
    ImportMode, VariantChanges, VersionRelation, compare_versions, conflict_for, import,
    update_variant,
};

#[test]
fn version_comparison_is_lenient() {
    use std::cmp::Ordering;

    assert_eq!(compare_versions("1.4.2", "1.4.10"), Some(Ordering::Less));
    assert_eq!(compare_versions("1.5.0", "1.4.2"), Some(Ordering::Greater));
    assert_eq!(compare_versions("v0.53", "0.53"), Some(Ordering::Equal));
    assert_eq!(compare_versions("1.32", "1.3"), Some(Ordering::Greater));

    // 认不出来就返回 None，绝不瞎猜
    assert_eq!(compare_versions("正式版", "1.0"), None);
    assert_eq!(compare_versions("", "1.0"), None);
    assert_eq!(compare_versions("abc", "def"), None);
}

#[test]
fn conflict_detection_matches_by_id_then_name() {
    let fixture = fixture();
    let package = build_zip(
        fixture.work.path(),
        "golden.zip",
        &[
            (
                "metadata.txt",
                "title=黄金之遗\nid=HTXL.golden\ncampaign=Lotv\nversion=1.32\n",
            ),
            ("paiur01.SC2Map", "stub"),
        ],
    );
    let variant = import(&fixture.library, &package, "lotv", ImportMode::Rename).expect("import");
    assert_eq!(variant.registration_id.as_deref(), Some("HTXL.golden"));

    // 同一个注册 ID -> 命中，且能比较版本
    let conflict = conflict_for(
        &fixture.library,
        "lotv",
        Some("htxl.GOLDEN"),
        "改了个名字",
        Some("1.40"),
    )
    .expect("应当命中同 ID 的已有版本");
    assert!(conflict.same_id);
    assert_eq!(conflict.existing_id, variant.id);
    assert_eq!(conflict.relation, VersionRelation::Newer);

    // 没有 ID 时按名字兜底
    let by_name = conflict_for(&fixture.library, "lotv", None, "黄金之遗", Some("1.30"))
        .expect("应当按名字命中");
    assert!(!by_name.same_id);
    assert_eq!(by_name.relation, VersionRelation::Older);

    // 都不匹配 -> 没有冲突
    assert!(conflict_for(&fixture.library, "lotv", None, "别的战役", Some("9.9")).is_none());
}

#[test]
fn rename_mode_keeps_both_versions() {
    let fixture = fixture();
    let first = import(
        &fixture.library,
        &package(&fixture, "a.zip", "重生", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("first");
    let second = import(
        &fixture.library,
        &package(&fixture, "b.zip", "重生", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("second");

    assert_ne!(first.id, second.id);
    assert_eq!(
        slot(&fixture, "wol").variants.len(),
        2,
        "重命名模式应当两者并存"
    );
}

#[test]
fn overwrite_mode_replaces_in_place_and_keeps_identity() {
    let fixture = fixture();
    let old = build_zip(
        fixture.work.path(),
        "old.zip",
        &[
            (
                "metadata.txt",
                "title=重生\nid=creator.reborn\ncampaign=WOL\nversion=1.0\n",
            ),
            ("maps/old.SC2Map", "stub"),
        ],
    );
    let first = import(&fixture.library, &old, "wol", ImportMode::Rename).expect("first");
    assert_eq!(first.version.as_deref(), Some("1.0"));

    let new = build_zip(
        fixture.work.path(),
        "new.zip",
        &[
            (
                "metadata.txt",
                "title=重生\nid=creator.reborn\ncampaign=WOL\nversion=2.0\n",
            ),
            ("maps/new.SC2Map", "stub"),
        ],
    );
    let second = import(&fixture.library, &new, "wol", ImportMode::Overwrite).expect("overwrite");

    // 覆盖更新沿用同一个目录名，因此挂在它身上的补丁绑定不受影响
    assert_eq!(second.id, first.id);
    assert_eq!(second.version.as_deref(), Some("2.0"));
    assert_eq!(
        slot(&fixture, "wol").variants.len(),
        1,
        "覆盖更新不应留下旧记录"
    );

    let installed = fixture.library.slot_dir("wol").join(&second.id);
    assert!(installed.join("metadata.txt").is_file());
    assert!(
        !installed.join("maps").join("old.SC2Map").exists(),
        "旧版本内容应当被换掉"
    );
    assert!(installed.join("maps").join("new.SC2Map").is_file());
}

#[test]
fn editing_metadata_keeps_content_and_directory() {
    let fixture = fixture();
    let variant = import(
        &fixture.library,
        &package(&fixture, "a.zip", "原名", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import");

    let updated = update_variant(
        &fixture.library,
        "wol",
        &variant.id,
        VariantChanges {
            name: Some("改过的名字".to_string()),
            author: Some("某位作者".to_string()),
            registration_id: Some("someone.reborn".to_string()),
            version: Some("1.2.3".to_string()),
            description: Some("新描述".to_string()),
            declared_mods: None,
        },
    )
    .expect("update");

    assert_eq!(updated.name, "改过的名字");
    assert_eq!(updated.author.as_deref(), Some("某位作者"));
    assert_eq!(updated.registration_id.as_deref(), Some("someone.reborn"));
    // 版本号能改，而且存的是规范形式
    assert_eq!(updated.version.as_deref(), Some("1.2.3"));
    assert_eq!(updated.description.as_deref(), Some("新描述"));

    // 改名不动目录：补丁绑定的锚点必须稳定
    assert_eq!(updated.id, variant.id);
    let dir = fixture.library.slot_dir("wol").join(&updated.id);
    assert!(dir.join("metadata.txt").is_file(), "包内容不应被动过");

    // 改动要落盘
    let reloaded = slot(&fixture, "wol");
    assert_eq!(reloaded.variants[0].name, "改过的名字");

    // 空名字要被拒绝，而不是写进去
    let rejected = update_variant(
        &fixture.library,
        "wol",
        &variant.id,
        VariantChanges {
            name: Some("   ".to_string()),
            ..VariantChanges::default()
        },
    );
    assert!(rejected.is_err(), "空战役名应当被拒绝");

    // 作者留空 = 清空，界面上会显示成未知作者
    let cleared = update_variant(
        &fixture.library,
        "wol",
        &variant.id,
        VariantChanges {
            author: Some(String::new()),
            ..VariantChanges::default()
        },
    )
    .expect("clear author");
    assert_eq!(cleared.author, None);
}
