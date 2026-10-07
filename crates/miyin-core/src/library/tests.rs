//! 战役库的端到端测试：导入 → 启用 → 切换 → 还原。

use std::path::{Path, PathBuf};

use crate::library::{
    ImportMode, Library, VariantChanges, VersionRelation, activate, compare_versions, conflict_for,
    import, remove_variant, update_variant,
};
use crate::sc2::{DiscoverySource, Installation};

const MARKER: &str = "StarCraft II.exe";

/// 用给定条目构造一个 zip。
fn build_zip(dir: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    use std::io::Write;

    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    for (entry, content) in files {
        writer.start_file(*entry, options).expect("start file");
        writer.write_all(content.as_bytes()).expect("write");
    }
    writer.finish().expect("finish");

    path
}

/// 一套临时环境：假游戏安装 + 独立的库目录 + 造包用的工作目录。
struct Fixture {
    _game: tempfile::TempDir,
    _library_dir: tempfile::TempDir,
    work: tempfile::TempDir,
    installation: Installation,
    library: Library,
}

fn fixture() -> Fixture {
    let game = tempfile::tempdir().expect("game dir");
    std::fs::write(game.path().join(MARKER), b"stub").expect("marker");
    std::fs::write(
        game.path().join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");

    let installation =
        Installation::from_root(game.path(), DiscoverySource::Manual).expect("valid installation");

    let library_dir = tempfile::tempdir().expect("library dir");

    Fixture {
        installation,
        library: Library::new(library_dir.path().join("data")),
        work: tempfile::tempdir().expect("work dir"),
        _game: game,
        _library_dir: library_dir,
    }
}

/// 造一个 CCM 风格的战役包（metadata.txt 在子目录里 —— 也就是最常见的那种）。
fn package(fixture: &Fixture, name: &str, title: &str, maps: &[&str]) -> PathBuf {
    let mut files: Vec<(String, String)> = vec![(
        "Reborn/metadata.txt".to_string(),
        format!("title={title}\nauthor=Creator\ncampaign=WOL\nversion=1.0\n"),
    )];
    for map in maps {
        files.push((format!("Reborn/maps/{map}"), "stub".to_string()));
    }

    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();

    build_zip(fixture.work.path(), name, &borrowed)
}

fn slot(fixture: &Fixture, slug: &str) -> crate::library::SlotView {
    fixture
        .library
        .slots(Some(&fixture.installation))
        .into_iter()
        .find(|slot| slot.slug == slug)
        .expect("slot exists")
}

#[test]
fn slots_follow_official_release_order() {
    let fixture = fixture();
    let slugs: Vec<String> = fixture
        .library
        .slots(Some(&fixture.installation))
        .into_iter()
        .map(|slot| slot.slug)
        .collect();

    // 主菜单只有四大战役（进化归虫群之心、序章归虚空之遗）
    assert_eq!(slugs, vec!["wol", "hots", "lotv", "nova"]);

    // 用户提的问题：虚空之遗必须排在诺娃前面
    let lotv = slugs.iter().position(|slug| slug == "lotv").expect("lotv");
    let nova = slugs.iter().position(|slug| slug == "nova").expect("nova");
    assert!(lotv < nova);
}

#[test]
fn imports_multiple_versions_of_the_same_campaign() {
    let fixture = fixture();

    let first = import(
        &fixture.library,
        &package(&fixture, "a.zip", "自由之翼：重生", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import first");
    let second = import(
        &fixture.library,
        &package(
            &fixture,
            "b.zip",
            "自由之翼：重生",
            &["01.SC2Map", "02.SC2Map"],
        ),
        "wol",
        ImportMode::Rename,
    )
    .expect("import second");

    assert_eq!(first.name, "自由之翼：重生");
    assert_ne!(first.id, second.id, "同名的两个版本必须能并存");
    assert_eq!(second.map_count, 2);

    let wol = slot(&fixture, "wol");
    assert_eq!(wol.variants.len(), 2);
    assert_eq!(wol.active, None, "默认应当是原版战役");
}

#[test]
fn activate_then_deactivate_restores_vanilla() {
    let fixture = fixture();
    let variant = import(
        &fixture.library,
        &package(&fixture, "a.zip", "重生", &["01.SC2Map", "02.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import");

    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("activate");

    // 自由之翼的地图直接放在 Maps/Campaign 根下
    let target = fixture.installation.campaign_maps_root.join("01.SC2Map");
    assert!(target.is_file(), "启用后地图应出现在官方战役目录");

    let wol = slot(&fixture, "wol");
    assert_eq!(wol.active.as_deref(), Some(variant.id.as_str()));
    assert_eq!(wol.active_name.as_deref(), Some("重生"));

    activate(&fixture.library, &fixture.installation, "wol", None).expect("deactivate");

    assert!(!target.exists(), "切回原版后我们放的文件必须被清理");
    assert_eq!(slot(&fixture, "wol").active, None);
}

#[test]
fn switching_between_versions_swaps_files() {
    let fixture = fixture();
    let a = import(
        &fixture.library,
        &package(&fixture, "a.zip", "版本A", &["A.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import a");
    let b = import(
        &fixture.library,
        &package(&fixture, "b.zip", "版本B", &["B.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import b");

    activate(&fixture.library, &fixture.installation, "wol", Some(&a.id)).expect("activate a");
    assert!(
        fixture
            .installation
            .campaign_maps_root
            .join("A.SC2Map")
            .is_file()
    );

    activate(&fixture.library, &fixture.installation, "wol", Some(&b.id)).expect("activate b");
    assert!(
        !fixture
            .installation
            .campaign_maps_root
            .join("A.SC2Map")
            .exists(),
        "切换后旧版本的文件必须消失"
    );
    assert!(
        fixture
            .installation
            .campaign_maps_root
            .join("B.SC2Map")
            .is_file()
    );
}

#[test]
fn official_map_is_backed_up_and_restored() {
    let fixture = fixture();

    // 造一个"官方地图"
    std::fs::create_dir_all(&fixture.installation.campaign_maps_root).expect("mkdir");
    let official = fixture.installation.campaign_maps_root.join("01.SC2Map");
    std::fs::write(&official, b"OFFICIAL").expect("write official");

    let variant = import(
        &fixture.library,
        &package(&fixture, "a.zip", "重生", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import");
    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("activate");

    assert_eq!(std::fs::read(&official).expect("read"), b"stub");

    activate(&fixture.library, &fixture.installation, "wol", None).expect("deactivate");
    assert_eq!(
        std::fs::read(&official).expect("read"),
        b"OFFICIAL",
        "官方地图必须被原样还原"
    );
}

#[test]
fn removing_active_version_switches_back_to_vanilla_first() {
    let fixture = fixture();
    let variant = import(
        &fixture.library,
        &package(&fixture, "a.zip", "重生", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import");
    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("activate");

    remove_variant(&fixture.library, &fixture.installation, "wol", &variant.id).expect("remove");

    assert!(
        !fixture
            .installation
            .campaign_maps_root
            .join("01.SC2Map")
            .exists()
    );
    let wol = slot(&fixture, "wol");
    assert!(wol.variants.is_empty());
    assert_eq!(wol.active, None);
}

#[test]
fn rejects_unknown_or_malicious_slots() {
    let fixture = fixture();
    let path = package(&fixture, "a.zip", "重生", &["01.SC2Map"]);

    for bad in ["nope", "../evil", "", "wol/../.."] {
        assert!(
            import(&fixture.library, &path, bad, ImportMode::Rename).is_err(),
            "槽位 {bad} 必须被拒绝"
        );
    }
}

#[test]
fn slot_for_routes_evolution_packages_to_hots() {
    use crate::campaign::metadata::CampaignType;
    use crate::library::slot_for;

    assert_eq!(slot_for(&CampaignType::Wol), Some("wol"));
    assert_eq!(slot_for(&CampaignType::Hots), Some("hots"));
    // 进化包自动归到「虫群之心」——这就是「自动判断是哪个战役」的依据
    assert_eq!(slot_for(&CampaignType::HotsEvolution), Some("hots"));
    assert_eq!(slot_for(&CampaignType::Lotv), Some("lotv"));
    assert_eq!(slot_for(&CampaignType::LotvPrologue), Some("lotv"));
    assert_eq!(slot_for(&CampaignType::Nova), Some("nova"));
    // 认不出来时必须返回 None，让界面去问用户
    assert_eq!(slot_for(&CampaignType::Other("???".into())), None);
}

#[test]
fn package_cover_is_picked_up() {
    let fixture = fixture();
    let path = build_zip(
        fixture.work.path(),
        "with-cover.zip",
        &[
            ("m/metadata.txt", "title=带封面的战役\ncampaign=WOL\n"),
            ("m/cover.png", "PNGDATA"),
            ("m/01.SC2Map", "stub"),
        ],
    );

    let variant = import(&fixture.library, &path, "wol", ImportMode::Rename).expect("import");
    assert_eq!(
        variant.cover.as_deref(),
        Some("cover.png"),
        "包内自带 cover.png 时应当被识别为封面"
    );

    let stored = fixture
        .library
        .slot_dir("wol")
        .join(&variant.id)
        .join("cover.png");
    assert!(stored.is_file());
}

#[test]
fn package_without_cover_leaves_it_empty() {
    let fixture = fixture();
    let variant = import(
        &fixture.library,
        &package(&fixture, "a.zip", "无封面", &["01.SC2Map"]),
        "wol",
        ImportMode::Rename,
    )
    .expect("import");

    assert_eq!(variant.cover, None, "没有封面时留空，由界面退回官方美术");
}

#[test]
fn evolution_package_targets_evolution_directory() {
    let fixture = fixture();
    let path = build_zip(
        fixture.work.path(),
        "evo.zip",
        &[
            ("m/metadata.txt", "title=进化重制\ncampaign=HOTSEVO\n"),
            ("m/01.SC2Map", "stub"),
        ],
    );

    // 进化包导入到「虫群之心」槽位……
    let variant = import(&fixture.library, &path, "hots", ImportMode::Rename).expect("import");
    assert_eq!(
        variant.target_sub.as_deref(),
        Some("swarm/evolution"),
        "启用时应当落到 Maps/Campaign/swarm/evolution"
    );

    // ……但启用后地图落在进化目录
    activate(
        &fixture.library,
        &fixture.installation,
        "hots",
        Some(&variant.id),
    )
    .expect("activate");
    assert!(
        fixture
            .installation
            .campaign_maps_root
            .join("swarm")
            .join("evolution")
            .join("01.SC2Map")
            .is_file()
    );
}

#[test]
fn hots_maps_land_in_swarm_subdirectory() {
    let fixture = fixture();
    let path = build_zip(
        fixture.work.path(),
        "hots.zip",
        &[
            ("m/metadata.txt", "title=虫群重制\ncampaign=HOTS\n"),
            ("m/01.SC2Map", "stub"),
        ],
    );

    let variant = import(&fixture.library, &path, "hots", ImportMode::Rename).expect("import");
    activate(
        &fixture.library,
        &fixture.installation,
        "hots",
        Some(&variant.id),
    )
    .expect("activate");

    assert!(
        fixture
            .installation
            .campaign_maps_root
            .join("swarm")
            .join("01.SC2Map")
            .is_file(),
        "虫群之心的地图应当落到 Maps/Campaign/swarm"
    );
}

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
            description: Some("新描述".to_string()),
        },
    )
    .expect("update");

    assert_eq!(updated.name, "改过的名字");
    assert_eq!(updated.author.as_deref(), Some("某位作者"));
    assert_eq!(updated.registration_id.as_deref(), Some("someone.reborn"));
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
