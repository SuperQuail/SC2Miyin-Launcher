//! 战役库的端到端测试：导入 → 启用 → 切换 → 还原。

use std::path::{Path, PathBuf};

use crate::campaign::package;
use crate::library::{
    ImportMode, Library, MapEntry, VariantChanges, VersionRelation, activate, compare_versions,
    compose, conflict_for, import, patch, remove_variant, resolve_main_map, update_variant,
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

    // 主菜单 = 四大原版战役 + 自制战役（进化归虫群之心、序章归虚空之遗）
    assert_eq!(slugs, vec!["wol", "hots", "lotv", "nova", "custom"]);

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

#[test]
fn patch_overlays_the_campaign_and_can_be_toggled() {
    let fixture = fixture();

    // 战役本体：一张地图
    let campaign = build_zip(
        fixture.work.path(),
        "campaign.zip",
        &[
            (
                "metadata.txt",
                "title=本体战役\ncampaign=WOL\nid=demo.base\nversion=1.0\n",
            ),
            ("paiur01.SC2Map", "本体的地图"),
        ],
    );
    let variant =
        import(&fixture.library, &campaign, "wol", ImportMode::Rename).expect("import campaign");

    // 补丁：一个模组，没有地图 -> 应当被判为补丁
    let patch_zip = build_zip(
        fixture.work.path(),
        "patch.zip",
        &[
            ("patch.txt", "name=数值补丁\nauthor=某人\npriority=200\n"),
            ("Mods/Extra.SC2Mod", "补丁的模组"),
        ],
    );
    let patch = patch::import_patch(&fixture.library, &patch_zip).expect("import patch");
    assert_eq!(patch.name, "数值补丁");
    assert_eq!(patch.priority, 200, "优先级应当从 patch.txt 读出来");
    assert!(patch.requires.is_empty(), "没有声明依赖 -> 只能手动指定",);

    // 通用补丁**不参与**自动挂载
    assert!(!patch::matches_slot(&fixture.library, "wol", &patch));
    let auto = patch::auto_bind(&fixture.library).expect("auto bind");
    assert!(auto.is_empty(), "没有依赖的补丁不该被自动挂上");

    // 手动挂上并启用
    patch::bind(&fixture.library, "wol", &patch.id).expect("bind");
    let composition = {
        // 先看合成清单：补丁的模组应当出现在里面
        let index = fixture.library.index();
        let variant_now = index.slots["wol"].variants[0].clone();
        compose::compose(&fixture.library, "wol", &variant_now, Some(""))
    };
    assert_eq!(
        composition.patched_count(),
        1,
        "合成清单里应当有 1 项来自补丁"
    );

    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("activate");

    let mods_file = fixture.installation.mods_root.join("Extra.SC2Mod");
    let map_file = fixture
        .installation
        .campaign_maps_root
        .join("paiur01.SC2Map");
    assert!(mods_file.is_file(), "补丁的模组应当被铺进 Mods");
    assert!(map_file.is_file(), "战役本体的地图应当照常铺进去");
    assert_eq!(
        std::fs::read_to_string(&map_file).expect("read"),
        "本体的地图"
    );

    // 关掉补丁 -> 它的文件撤下，战役本体不受影响
    patch::configure_binding(&fixture.library, "wol", &patch.id, Some(false), None)
        .expect("disable patch");
    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("re-activate");
    assert!(!mods_file.exists(), "关掉补丁后它的文件应当被撤下");
    assert!(map_file.is_file(), "战役本体不该受影响");

    // 再打开 -> 又回来
    patch::configure_binding(&fixture.library, "wol", &patch.id, Some(true), None)
        .expect("enable patch");
    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("re-activate");
    assert!(mods_file.is_file(), "重新打开补丁后文件应当回来");

    // 切回原版 -> 干净
    activate(&fixture.library, &fixture.installation, "wol", None).expect("vanilla");
    assert!(!map_file.exists());
    assert!(!mods_file.exists());
}

#[test]
fn patch_priority_decides_who_wins() {
    let fixture = fixture();

    let campaign = build_zip(
        fixture.work.path(),
        "c.zip",
        &[
            ("metadata.txt", "title=本体\ncampaign=WOL\n"),
            ("paiur01.SC2Map", "本体"),
        ],
    );
    let variant = import(&fixture.library, &campaign, "wol", ImportMode::Rename).expect("import");

    // 两个补丁盖同一个文件，优先级不同
    let low = build_zip(
        fixture.work.path(),
        "low.zip",
        &[("Mods/Shared.SC2Mod", "低优先级")],
    );
    let high = build_zip(
        fixture.work.path(),
        "high.zip",
        &[("Mods/Shared.SC2Mod", "高优先级")],
    );

    let low = patch::import_patch(&fixture.library, &low).expect("low");
    let high = patch::import_patch(&fixture.library, &high).expect("high");

    patch::bind(&fixture.library, "wol", &low.id).expect("bind low");
    patch::bind(&fixture.library, "wol", &high.id).expect("bind high");
    patch::configure_binding(&fixture.library, "wol", &low.id, None, Some(100)).expect("prio low");
    patch::configure_binding(&fixture.library, "wol", &high.id, None, Some(900))
        .expect("prio high");

    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&variant.id),
    )
    .expect("activate");

    let shared = fixture.installation.mods_root.join("Shared.SC2Mod");
    assert_eq!(
        std::fs::read_to_string(&shared).expect("read"),
        "高优先级",
        "优先级高的补丁应当胜出"
    );
}

#[test]
fn exported_package_is_ccm_readable_and_round_trips() {
    let fixture = fixture();

    let campaign = build_zip(
        fixture.work.path(),
        "golden.zip",
        &[
            (
                "metadata.txt",
                "title=黄金之遗\ncampaign=LoTV\nid=HTXL.golden\nauthor=HTXL\nversion=1.32\n",
            ),
            ("paiur01.SC2Map", "第一张"),
            ("paiur02.SC2Map", "第二张"),
            ("HTXL.SC2Mod", "模组"),
        ],
    );
    let variant = import(&fixture.library, &campaign, "lotv", ImportMode::Rename).expect("import");

    // 再挂一个补丁，验证它能被一起带走
    let patch_zip = build_zip(
        fixture.work.path(),
        "p.zip",
        &[("Mods/Extra.SC2Mod", "补丁内容")],
    );
    let patch = patch::import_patch(&fixture.library, &patch_zip).expect("patch");
    patch::bind(&fixture.library, "lotv", &patch.id).expect("bind");

    let destination = fixture.work.path().join("导出.zip");
    let report = crate::library::export::export(
        &fixture.library,
        "lotv",
        &variant,
        &crate::library::export::ExportOptions {
            destination: destination.clone(),
            merge_patches: false,
        },
    )
    .expect("export");

    assert_eq!(report.maps, 2);
    assert_eq!(report.mods, 1);
    assert_eq!(report.patches, vec!["p".to_string()], "补丁应当被一起带走");

    // 用启动器自己的预检读回来：必须仍被认成 CCM 包、归属不变
    let back = package::inspect(&destination).expect("inspect export");
    assert_eq!(
        back.format,
        crate::campaign::CampaignFormat::Ccm,
        "必须是 CCM 读得懂的形态"
    );
    assert_eq!(
        back.campaign_type,
        crate::campaign::metadata::CampaignType::Lotv
    );
    assert_eq!(back.name.as_deref(), Some("黄金之遗"));
    assert_eq!(back.author.as_deref(), Some("HTXL"));
    assert_eq!(back.id.as_deref(), Some("HTXL.golden"));
    assert_eq!(back.version.as_deref(), Some("1.32"));
    assert_eq!(back.map_count, 2);
    assert_eq!(back.mod_count, 1);

    // CCM 只认根目录平铺的 .SC2Map；我们的额外数据全在 Miyin/ 里
    let file = std::fs::File::open(&destination).expect("open");
    let mut zip = zip::ZipArchive::new(file).expect("zip");
    let names: Vec<String> = (0..zip.len())
        .filter_map(|index| {
            zip.by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .collect();
    assert!(names.contains(&"metadata.txt".to_string()));
    assert!(
        names.contains(&"paiur01.SC2Map".to_string()),
        "地图要平铺在根"
    );
    assert!(names.contains(&"Miyin/metadata.json".to_string()));
    assert!(names.contains(&"Miyin/bindings.json".to_string()));
    assert!(
        names.iter().any(|name| name.starts_with("Miyin/patches/")),
        "补丁原件要留在 Miyin/patches/ 下"
    );
    assert!(
        !names
            .iter()
            .any(|name| name.starts_with("Maps/") || name.starts_with("Mods/")),
        "导出物顶层不该出现游戏目录结构，否则 CCM 读不到"
    );
}

#[test]
fn exporting_with_merged_patches_puts_them_on_top() {
    let fixture = fixture();

    let campaign = build_zip(
        fixture.work.path(),
        "c.zip",
        &[
            ("metadata.txt", "title=本体\ncampaign=WOL\n"),
            ("a.SC2Map", "本体"),
        ],
    );
    let variant = import(&fixture.library, &campaign, "wol", ImportMode::Rename).expect("import");

    let patch_zip = build_zip(
        fixture.work.path(),
        "p.zip",
        &[("Mods/Only.SC2Mod", "补丁")],
    );
    let patch = patch::import_patch(&fixture.library, &patch_zip).expect("patch");
    patch::bind(&fixture.library, "wol", &patch.id).expect("bind");

    // 附加模式：顶层只有本体
    let plain = fixture.work.path().join("plain.zip");
    let report = crate::library::export::export(
        &fixture.library,
        "wol",
        &variant,
        &crate::library::export::ExportOptions {
            destination: plain.clone(),
            merge_patches: false,
        },
    )
    .expect("export plain");
    assert_eq!(report.mods, 0, "附加模式不该把补丁放进顶层");

    // 合成模式：顶层是打完补丁的样子
    let merged = fixture.work.path().join("merged.zip");
    let report = crate::library::export::export(
        &fixture.library,
        "wol",
        &variant,
        &crate::library::export::ExportOptions {
            destination: merged.clone(),
            merge_patches: true,
        },
    )
    .expect("export merged");
    assert_eq!(report.mods, 1, "合成模式应当把补丁的模组并进顶层");

    let back = package::inspect(&merged).expect("inspect");
    assert_eq!(back.mod_count, 1);
}

#[test]
fn exported_patch_round_trips_as_a_patch() {
    let fixture = fixture();

    let patch_zip = build_zip(
        fixture.work.path(),
        "kindergarten.zip",
        &[
            (
                "patch.txt",
                "name=幼儿园补丁\nauthor=小白\nversion=3.0\nrequires=KerriganRogue\npriority=150\n",
            ),
            ("Mods/KerriganRogue.SC2Mod", "补丁内容"),
        ],
    );
    let patch = patch::import_patch(&fixture.library, &patch_zip).expect("import patch");
    assert_eq!(patch.priority, 150);

    let destination = fixture.work.path().join("补丁导出.zip");
    let report = crate::library::export::export_patch(&fixture.library, &patch, &destination)
        .expect("export patch");
    assert_eq!(report.mods, 1);
    assert_eq!(report.maps, 0);

    // 重新导入：必须还是补丁，元数据与依赖不能丢
    let back = package::inspect(&destination).expect("inspect export");
    assert_eq!(
        back.kind,
        crate::campaign::metadata::PackageKind::Patch,
        "导出物必须仍被认成补丁"
    );
    assert_eq!(back.name.as_deref(), Some("幼儿园补丁"));
    assert_eq!(back.author.as_deref(), Some("小白"));
    assert_eq!(back.version.as_deref(), Some("3.0"));
    assert_eq!(back.priority, Some(150));
    assert_eq!(back.requires, vec!["KerriganRogue".to_string()]);

    // 内容是**平铺**的：裸的 .SC2Mod 重新导入时会落到 Mods/ 下，
    // 因此补丁能挂到任意战役上，而不是写死某个目录
    let file = std::fs::File::open(&destination).expect("open");
    let mut zip = zip::ZipArchive::new(file).expect("zip");
    let names: Vec<String> = (0..zip.len())
        .filter_map(|index| {
            zip.by_index(index)
                .ok()
                .map(|entry| entry.name().to_string())
        })
        .collect();
    assert!(names.contains(&"patch.txt".to_string()));
    assert!(
        names.contains(&"KerriganRogue.SC2Mod".to_string()),
        "补丁内容要平铺在根"
    );
    assert!(names.contains(&"Miyin/metadata.json".to_string()));
    assert!(
        !names.iter().any(|name| name.starts_with("Mods/")),
        "不该写成镜像路径，否则绑到别的战役会摆错位置"
    );

    // 真的再导一次
    let again = patch::import_patch(&fixture.library, &destination).expect("re-import");
    assert_eq!(again.name, "幼儿园补丁");
    assert_eq!(again.priority, 150);
}

#[test]
fn patch_metadata_can_be_edited() {
    let fixture = fixture();

    let patch_zip = build_zip(fixture.work.path(), "p.zip", &[("Mods/X.SC2Mod", "内容")]);
    let patch = patch::import_patch(&fixture.library, &patch_zip).expect("import");

    let updated = patch::update_patch(
        &fixture.library,
        &patch.id,
        patch::PatchChanges {
            name: Some("改过的补丁名".to_string()),
            author: Some("某位作者".to_string()),
            registration_id: Some("someone.patch".to_string()),
            description: Some("新描述".to_string()),
            priority: Some(500),
        },
    )
    .expect("update");

    assert_eq!(updated.name, "改过的补丁名");
    assert_eq!(updated.author.as_deref(), Some("某位作者"));
    assert_eq!(updated.registration_id.as_deref(), Some("someone.patch"));
    assert_eq!(updated.priority, 500);

    // 改动要落盘
    let reloaded = fixture
        .library
        .index()
        .patches
        .get(&patch.id)
        .cloned()
        .expect("reload");
    assert_eq!(reloaded.name, "改过的补丁名");

    // 包内容不该被动过
    assert!(
        fixture
            .library
            .patch_dir(&patch.id)
            .join("Mods/X.SC2Mod")
            .is_file()
    );

    // 空名字要被拒绝
    assert!(
        patch::update_patch(
            &fixture.library,
            &patch.id,
            patch::PatchChanges {
                name: Some("   ".to_string()),
                ..patch::PatchChanges::default()
            },
        )
        .is_err()
    );
}

#[test]
fn main_map_resolution_never_guesses() {
    let maps = vec![
        MapEntry {
            path: "1. Rebel Yell/Terran01.SC2Map".to_string(),
            name: "Terran01".to_string(),
            chapter: Some("1. Rebel Yell".to_string()),
            size: 0,
            is_main: false,
        },
        MapEntry {
            path: "2. Overmind/Zerg01.SC2Map".to_string(),
            name: "Zerg01".to_string(),
            chapter: Some("2. Overmind".to_string()),
            size: 0,
            is_main: false,
        },
    ];

    // 声明了且存在 -> 用它
    let hit = resolve_main_map(&maps, Some("1. Rebel Yell/Terran01.SC2Map"));
    assert_eq!(hit.path.as_deref(), Some("1. Rebel Yell/Terran01.SC2Map"));
    assert!(hit.warning.is_none());

    // 大小写与斜杠方向都该认
    let loose = resolve_main_map(&maps, Some("1. rebel yell\\terran01.sc2map"));
    assert_eq!(loose.path.as_deref(), Some("1. Rebel Yell/Terran01.SC2Map"));

    // 只写文件名也认
    let by_name = resolve_main_map(&maps, Some("Zerg01"));
    assert_eq!(by_name.path.as_deref(), Some("2. Overmind/Zerg01.SC2Map"));

    // 声明了但找不到 -> 只警告，不阻断
    let missing = resolve_main_map(&maps, Some("3. The Fall/Protoss99.SC2Map"));
    assert_eq!(missing.path, None);
    assert!(missing.warning.is_some());

    // 没声明、有多张 -> 不猜
    let unset = resolve_main_map(&maps, None);
    assert_eq!(unset.path, None);
    assert!(!unset.automatic);

    // 没声明、只有一张 -> 替你选（省一次点击）
    let solo = resolve_main_map(&maps[..1], None);
    assert_eq!(solo.path.as_deref(), Some("1. Rebel Yell/Terran01.SC2Map"));
    assert!(solo.automatic);
}

#[test]
fn natural_order_puts_terran2_before_terran10() {
    let mut names = vec!["Terran10", "Terran2", "Terran1"];
    names.sort_by(|left, right| super::natural_cmp(left, right));
    assert_eq!(names, vec!["Terran1", "Terran2", "Terran10"]);

    // 章节也要按数字排
    let mut chapters = vec!["2. Overmind", "10. Later", "1. Rebel Yell"];
    chapters.sort_by(|left, right| super::natural_cmp(left, right));
    assert_eq!(chapters, vec!["1. Rebel Yell", "2. Overmind", "10. Later"]);
}
#[test]
fn unconfigured_variants_mount_every_mod() {
    use crate::campaign::package::{Payload, PayloadTarget};
    use crate::library::{Variant, effective_mounted_mods};

    let mods = |name: &str| Payload {
        source: name.to_string(),
        target: PayloadTarget::Mod {
            name: name.to_string(),
        },
        expanded: false,
        is_mod: true,
    };

    let mut variant = Variant {
        id: "v1".to_string(),
        payloads: vec![mods("A.SC2Mod"), mods("B.SC2Mod")],
        ..Variant::default()
    };

    // 老记录没有 mounted_mods 字段 -> 按「全挂」算。
    // 这是防回归：曾经用空 Vec 表示"没配过"，结果官方战役包里带的模组
    // 会突然一个都不铺。
    assert_eq!(variant.mounted_mods, None);
    assert_eq!(
        effective_mounted_mods(&variant),
        vec!["A.SC2Mod".to_string(), "B.SC2Mod".to_string()]
    );

    // 用户明确关掉一个 -> 只铺剩下的
    variant.mounted_mods = Some(vec!["B.SC2Mod".to_string()]);
    assert_eq!(
        effective_mounted_mods(&variant),
        vec!["B.SC2Mod".to_string()]
    );

    // 用户明确全关 -> 一个都不铺（这与"没配过"是两回事）
    variant.mounted_mods = Some(Vec::new());
    assert!(effective_mounted_mods(&variant).is_empty());
}
#[test]
fn mods_are_grouped_by_folder_not_by_file() {
    use crate::campaign::package::{Payload, PayloadTarget};
    use crate::library::{group_mods, mod_key_of, normalize_mod_key};

    // 真实样本「疯批帝国军械库」：Mods/Alenger/ 下面 18 个 .SC2Mod
    // —— 那是**一个**模组，不是 18 个
    assert_eq!(
        mod_key_of("Mods/Alenger/1钢铁.SC2Mod").unwrap().key,
        "Alenger"
    );
    assert_eq!(
        mod_key_of("Mods/Alenger/通用效果.SC2Mod").unwrap().key,
        "Alenger"
    );

    // 单文件形态
    let single = mod_key_of("Mods/3疯批帝国之翼.SC2Mod").unwrap();
    assert_eq!(single.key, "3疯批帝国之翼.SC2Mod");
    assert_eq!(single.name, "3疯批帝国之翼");

    // 解开目录树形态
    let expanded = mod_key_of("Mods/kit_liberty_story.SC2Mod/Assets/x.dds").unwrap();
    assert_eq!(expanded.key, "kit_liberty_story.SC2Mod");
    assert_eq!(expanded.name, "kit_liberty_story");

    // 不在 Mods/ 下 —— 不是模组
    assert!(mod_key_of("Maps/Campaign/a.SC2Map").is_none());

    // 作者写依赖时偷懒不写前缀，也得认
    for raw in ["Mods/Alenger", "Mods/Alenger/", "Alenger", "Alenger.SC2Mod"] {
        assert_eq!(normalize_mod_key(raw), "Alenger", "写的是 {raw}");
    }

    // 分组：同一个文件夹下的多个载荷归成一行
    let payload = |source: &str| Payload {
        source: source.to_string(),
        target: PayloadTarget::Mirror {
            path: source.to_string(),
        },
        expanded: false,
        is_mod: true,
    };
    let payloads = vec![
        payload("Mods/Alenger/1钢铁.SC2Mod"),
        payload("Mods/Alenger/2贝希摩斯虫群.SC2Mod"),
        payload("Mods/Alenger/通用效果.SC2Mod"),
        payload("Mods/孤零零.SC2Mod"),
    ];

    let rows = group_mods(&payloads, &["Alenger".to_string()]);
    assert_eq!(rows.len(), 2, "三个文件应当归成一个模组");
    assert_eq!(rows[0].name, "Alenger");
    assert_eq!(rows[0].parts, 3);
    assert!(rows[0].mounted);
    assert_eq!(rows[1].name, "孤零零");
    assert_eq!(rows[1].parts, 1);
    assert!(!rows[1].mounted);
}
#[test]
fn mod_import_dedupes_and_picks_versions() {
    use crate::library::mods::{self, ModImportAction, ModImportMode};

    let dir = tempfile::tempdir().expect("临时目录");
    let data = dir.path().join("data");
    std::fs::create_dir_all(&data).expect("建目录");

    // 造一个模组目录：Mods/Demo/A.SC2Mod
    let make = |name: &str, body: &str| {
        let root = dir.path().join(name);
        let inner = root.join("Demo");
        std::fs::create_dir_all(&inner).expect("建模组目录");
        std::fs::write(inner.join("A.SC2Mod"), body).expect("写文件");
        root
    };

    let first = make("src1", "hello");
    let same = make("src2", "hello");
    let other = make("src3", "totally different");

    // ---- 1) 全新导入 ----
    let one = mods::import(
        &data,
        &first,
        Some("demo.mod"),
        Some("1.0"),
        ModImportMode::Auto,
    )
    .expect("导入 1");
    assert_eq!(one.action, ModImportAction::Added);
    assert_eq!(one.record.modid.as_deref(), Some("demo.mod"));
    assert_eq!(one.record.version.as_deref(), Some("1.0"));
    assert_eq!(mods::list(&data).len(), 1);

    // ---- 2) 内容一模一样：不留第二份 ----
    let two = mods::import(
        &data,
        &same,
        Some("demo.mod"),
        Some("1.0"),
        ModImportMode::Auto,
    )
    .expect("导入 2");
    assert_eq!(
        two.action,
        ModImportAction::Duplicate,
        "完全一样就该直接复用"
    );
    assert_eq!(mods::list(&data).len(), 1, "库里不该出现第二份一样的内容");
    // 复用的是原来那条，id 也应当是原来那个
    assert_eq!(two.record.id, one.record.id);

    // ---- 3) 内容不同、版本号撞了：自动改版本号 ----
    let three = mods::import(
        &data,
        &other,
        Some("demo.mod"),
        Some("1.2"),
        ModImportMode::Auto,
    )
    .expect("导入 3");
    assert_eq!(three.action, ModImportAction::NewVersion);
    assert_eq!(
        three.record.version.as_deref(),
        Some("1.2"),
        "包内声明的版本要被采纳"
    );
    assert_eq!(mods::list(&data).len(), 2);

    // ---- 3b) 没声明版本、内容又不同：推出来的版本号撞了，自动改名 ----
    let unnamed = make("src3b", "yet another body");
    let renamed = mods::import(&data, &unnamed, Some("demo.mod"), None, ModImportMode::Auto)
        .expect("导入 3b");
    assert_eq!(renamed.action, ModImportAction::Renamed);
    assert_ne!(
        renamed.record.version.as_deref(),
        Some("1.2"),
        "撞号要自动换个版本号"
    );

    // ---- 4) 作为独立改版：即使 modid 相同也单独一个模组 ----
    let four = mods::import(
        &data,
        &other,
        Some("demo.mod"),
        Some("1.2"),
        ModImportMode::Separate,
    )
    .expect("导入 4");
    assert_eq!(four.action, ModImportAction::Added);
    let id = four.record.modid.clone().unwrap_or_default();
    assert!(id.starts_with("demo.mod#"), "独立改版应当另起一个 id：{id}");
    // 1.0 + 1.2 + 撞号改名的那个 + 独立改版 = 4
    assert_eq!(mods::list(&data).len(), 4);

    // ---- 5) 预检能认出「库里已有同族的」 ----
    //
    // 用一份**全新的内容** —— 拿 other 的话，它在第 3 步已经进库了，
    // 预检会正确地说这是重复的（那也是一种正确行为，但不是这一步要验的）。
    let fresh = make("src5", "brand new content nobody has");
    let preview = mods::preview(&data, &fresh, Some("demo.mod"), Some("1.5")).expect("预检");
    assert_eq!(preview.modid, "demo.mod");
    assert!(
        preview.existing.len() >= 2,
        "同 modid 的已有版本应当被列出来"
    );
    assert!(!preview.duplicate, "全新内容不该判成重复");
    assert_eq!(preview.suggested_version, "1.5", "包内声明的版本要被采纳");

    // 反过来：内容确实重复时要认出来
    let again = mods::preview(&data, &other, Some("demo.mod"), Some("1.2")).expect("预检 2");
    assert!(again.duplicate, "同一份内容第二次来应当判成重复");
}

#[test]
fn enabling_a_mod_disables_its_siblings() {
    use crate::library::mods;

    let dir = tempfile::tempdir().expect("临时目录");
    let data = dir.path().join("data");
    std::fs::create_dir_all(&data).expect("建目录");

    let make = |name: &str, body: &str| {
        let root = dir.path().join(name);
        std::fs::create_dir_all(root.join("Demo")).expect("建模组目录");
        std::fs::write(root.join("Demo").join("A.SC2Mod"), body).expect("写文件");
        root
    };

    mods::import(
        &data,
        &make("a", "one"),
        Some("demo.mod"),
        Some("1.0"),
        Default::default(),
    )
    .expect("导入 A");
    mods::import(
        &data,
        &make("b", "two"),
        Some("demo.mod"),
        Some("2.0"),
        Default::default(),
    )
    .expect("导入 B");

    let all = mods::list(&data);
    assert_eq!(all.len(), 2);

    // 启用第一个
    mods::set_enabled(&data, &all[0].id, true).expect("启用");
    let after_first = mods::list(&data);
    assert_eq!(after_first.iter().filter(|item| item.enabled).count(), 1);

    // 再启用第二个：第一个必须让位 —— 它们在游戏目录里抢同一个文件夹
    mods::set_enabled(&data, &all[1].id, true).expect("启用第二个");
    let after_second = mods::list(&data);
    let on: Vec<&str> = after_second
        .iter()
        .filter(|item| item.enabled)
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(on, vec![all[1].id.as_str()], "同 modid 只能有一个启用");
}
#[test]
fn imported_mods_keep_their_original_folder_shape() {
    use crate::library::mods::{self, ModKind};

    let fixture = fixture();
    let data = fixture.library.root();

    // ---- 1) 目录形态：Alenger/ 里两个 .SC2Mod（真实样本有 18 个）----
    let src = fixture.work.path().join("Alenger");
    std::fs::create_dir_all(&src).expect("建模组目录");
    std::fs::write(src.join("1钢铁.SC2Mod"), b"one").expect("写文件");
    std::fs::write(src.join("通用效果.SC2Mod"), b"two").expect("写文件");

    let one = mods::import(data, &src, None, Some("1.0"), Default::default()).expect("导入目录");
    assert_eq!(one.record.folder, "Alenger", "名字必须原样保留，不能加后缀");
    assert_eq!(one.record.kind, ModKind::Folder);
    assert_eq!(one.record.parts, 2);

    mods::set_enabled(data, &one.record.id, true).expect("启用");
    mods::sync(data, &fixture.installation).expect("铺进游戏目录");

    let placed = fixture.installation.mods_root.join("Alenger");
    assert!(placed.is_dir(), "应当铺成 Mods/Alenger/ 目录");
    assert!(
        placed.join("1钢铁.SC2Mod").is_file(),
        "层级不能变：地图里写的是 Mods\\Alenger\\1钢铁.SC2Mod"
    );
    assert!(
        !fixture
            .installation
            .mods_root
            .join("Alenger.SC2Mod")
            .exists(),
        "绝不能自作主张加 .SC2Mod 后缀 —— 加了地图就找不到这个模组"
    );

    // ---- 2) 单文件形态：一个 .SC2Mod 文件 ----
    let file = fixture.work.path().join("孤单.SC2Mod");
    std::fs::write(&file, b"alone").expect("写文件");

    let two = mods::import(data, &file, None, Some("1.0"), Default::default()).expect("导入文件");
    assert_eq!(two.record.folder, "孤单.SC2Mod", "文件名原样保留");
    assert_eq!(two.record.kind, ModKind::File);

    mods::set_enabled(data, &two.record.id, true).expect("启用 2");
    mods::sync(data, &fixture.installation).expect("再铺一次");

    assert!(
        fixture.installation.mods_root.join("孤单.SC2Mod").is_file(),
        "单文件模组要铺成**文件**，不是目录"
    );
}

#[test]
fn importing_a_campaign_archive_picks_the_mod_inside() {
    use std::io::Write;

    use crate::library::mods::{self, ModKind};

    let fixture = fixture();

    // 照真实样本（疯批帝国军械库）的摆法：包根一个战役目录，
    // 里面 Maps/ 是战役地图、Mods/Alenger/ 才是用户要的模组
    let zip = fixture.work.path().join("pkg.zip");
    {
        let file = std::fs::File::create(&zip).expect("建包");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (entry, content) in [
            ("疯批帝国军械库2.3/Maps/Campaign/a.SC2Map", "map"),
            ("疯批帝国军械库2.3/Mods/Alenger/1钢铁.SC2Mod", "one"),
            ("疯批帝国军械库2.3/Mods/Alenger/2贝希摩斯虫群.SC2Mod", "two"),
        ] {
            writer.start_file(entry, options).expect("start");
            writer.write_all(content.as_bytes()).expect("write");
        }
        writer.finish().expect("finish");
    }

    let one = mods::import(
        fixture.library.root(),
        &zip,
        None,
        Some("1.0"),
        Default::default(),
    )
    .expect("导入");

    assert_eq!(
        one.record.folder, "Alenger",
        "要从战役包里挑出 Mods/ 下那个唯一的模组"
    );
    assert_eq!(one.record.kind, ModKind::Folder);
    assert_eq!(one.record.parts, 2, "只应当有模组自己的两个文件");

    // 地图不能跟着进来 —— 不然它会被当成模组内容一起铺到游戏目录里
    mods::set_enabled(fixture.library.root(), &one.record.id, true).expect("启用");
    mods::sync(fixture.library.root(), &fixture.installation).expect("铺");

    let placed = fixture.installation.mods_root.join("Alenger");
    assert!(placed.join("1钢铁.SC2Mod").is_file());
    assert!(
        !fixture.installation.mods_root.join("Maps").exists(),
        "战役地图不该跟着模组铺进游戏 Mods 目录"
    );
}
#[test]
fn a_package_with_several_mods_imports_all_of_them() {
    use std::io::Write;

    use crate::library::mods::{self, ModKind};

    let fixture = fixture();

    // **照真实样本（疯批帝国军械库）一比一摆**：
    // 包根一个战役目录，里面 Maps/ 是战役地图，Mods/ 下有三个模组 ——
    // 一个单文件 + 两个目录（其中一个名字带 .SC2Mod 后缀）。
    let zip = fixture.work.path().join("armory.zip");
    {
        let file = std::fs::File::create(&zip).expect("建包");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (entry, content) in [
            ("疯批帝国军械库2.3/Maps/Campaign/a.SC2Map", "map"),
            ("疯批帝国军械库2.3/Mods/3疯批帝国之翼.SC2Mod", "single"),
            ("疯批帝国军械库2.3/Mods/Alenger/1钢铁.SC2Mod", "one"),
            ("疯批帝国军械库2.3/Mods/Alenger/2贝希摩斯虫群.SC2Mod", "two"),
            (
                "疯批帝国军械库2.3/Mods/kit_liberty_story.SC2Mod/Assets/x.dds",
                "asset",
            ),
        ] {
            writer.start_file(entry, options).expect("start");
            writer.write_all(content.as_bytes()).expect("write");
        }
        writer.finish().expect("finish");
    }

    let result = mods::import(
        fixture.library.root(),
        &zip,
        None,
        Some("2.4"),
        Default::default(),
    )
    .expect("导入");

    // 三个模组，一个都不该漏
    assert_eq!(result.records.len(), 3, "Mods/ 下有三个模组");
    let folders: Vec<&str> = result
        .records
        .iter()
        .map(|item| item.folder.as_str())
        .collect();
    assert_eq!(
        folders,
        vec![
            "3疯批帝国之翼.SC2Mod",
            "Alenger",
            "kit_liberty_story.SC2Mod"
        ],
        "名字必须原样保留，顺序按名字稳定排序"
    );

    let alenger = result
        .records
        .iter()
        .find(|item| item.folder == "Alenger")
        .expect("应当有 Alenger");
    assert_eq!(alenger.kind, ModKind::Folder);
    assert_eq!(alenger.parts, 2, "只算它自己的两个 .SC2Mod");

    let single = result
        .records
        .iter()
        .find(|item| item.folder == "3疯批帝国之翼.SC2Mod")
        .expect("应当有单文件那个");
    assert_eq!(single.kind, ModKind::File);

    // 全部启用后铺一遍：层级必须和原来一致
    for record in &result.records {
        mods::set_enabled(fixture.library.root(), &record.id, true).expect("启用");
    }
    mods::sync(fixture.library.root(), &fixture.installation).expect("铺");

    let mods_root = &fixture.installation.mods_root;
    assert!(
        mods_root.join("Alenger").join("1钢铁.SC2Mod").is_file(),
        "目录模组要保持层级"
    );
    assert!(
        mods_root.join("3疯批帝国之翼.SC2Mod").is_file(),
        "单文件模组要铺成文件"
    );
    assert!(
        mods_root
            .join("kit_liberty_story.SC2Mod")
            .join("Assets")
            .join("x.dds")
            .is_file(),
        "带后缀的目录模组同样保持层级"
    );
    assert!(
        !mods_root.join("Maps").exists(),
        "战役地图绝不能跟着模组铺进游戏 Mods 目录"
    );
}
#[test]
fn mods_inside_a_named_package_keep_their_folder() {
    use crate::campaign::package::payload_target;
    use crate::library::compose::{Placement, payload_target_path};

    // 真实样本的摆法：包根多一层「包名」目录，模组在它下面的 Mods/ 里。
    //
    // 踩过的坑：payload_target 原来只判断「开头是不是 Mods/」，
    // 这一层包名目录把前缀吃掉了，于是掉进「模组取最后一段当名字」那条路，
    // Mods/Alenger/1钢铁.SC2Mod 变成 Mods/1钢铁.SC2Mod ——
    // **Alenger/ 这一层没了**，地图里声明的 Mods\Alenger\… 自然找不到。
    let campaign = Placement::Campaign { sub: None };

    let nested = payload_target(
        "疯批帝国军械库2.3/Mods/Alenger/1钢铁.SC2Mod",
        true,
        "",
        false,
    );
    assert_eq!(
        payload_target_path(&nested, &campaign).as_deref(),
        Some("Mods/Alenger/1钢铁.SC2Mod"),
        "包名下面那一层也要原样保留"
    );

    let expanded = payload_target(
        "疯批帝国军械库2.3/Mods/kit_liberty_story.SC2Mod/Assets/x.dds",
        true,
        "",
        false,
    );
    assert_eq!(
        payload_target_path(&expanded, &campaign).as_deref(),
        Some("Mods/kit_liberty_story.SC2Mod/Assets/x.dds")
    );

    // 地图同理
    let map = payload_target(
        "疯批帝国军械库2.3/Maps/Campaign/thanson01.SC2Map",
        false,
        "",
        false,
    );
    assert_eq!(
        payload_target_path(&map, &campaign).as_deref(),
        Some("Maps/Campaign/thanson01.SC2Map")
    );

    // 顶层就是 Mods/ 的老写法照样对
    let plain = payload_target("Mods/SCMRmod.SC2Mod", true, "", false);
    assert_eq!(
        payload_target_path(&plain, &campaign).as_deref(),
        Some("Mods/SCMRmod.SC2Mod")
    );

    // 小写 maps/ 是作者的**分类习惯**，不是游戏目录名 ——
    // 不能当镜像（那会送到游戏根下），而是拍平到战役目录里。
    let lowercase = payload_target("maps/a.SC2Map", false, "", false);
    assert_eq!(
        payload_target_path(&lowercase, &campaign).as_deref(),
        Some("Maps/Campaign/a.SC2Map"),
        "分类目录只取文件名，落到游戏真正会扫的位置"
    );
}

#[test]
fn payload_counts_use_payload_roots_not_file_extensions() {
    use crate::campaign::package::{Payload, PayloadTarget, count_payloads};

    // 真实样本：地图是**解开的目录树**，里面全是 .xml / .galaxy。
    // 按文件扩展名数会得到「0 张地图」。
    //
    // 喂进来的是**归一化之后**的落点（payload_target 已经把包名那层剥掉了），
    // 这正是 count_payloads 实际会看到的东西。
    let make = |path: &str, is_mod: bool| Payload {
        source: path.to_string(),
        target: PayloadTarget::Mirror {
            path: path.to_string(),
        },
        expanded: !is_mod,
        is_mod,
    };

    let payloads = vec![
        make("Maps/Campaign/tarcade.SC2Map", false),
        make("Maps/Campaign/thanson01.SC2Map", false),
        // Alenger 下面 15 个 .SC2Mod —— 那是**一个**模组
        make("Mods/Alenger/1钢铁.SC2Mod", true),
        make("Mods/Alenger/2贝希摩斯虫群.SC2Mod", true),
        make("Mods/3疯批帝国之翼.SC2Mod", true),
    ];

    let (maps, mods) = count_payloads(&payloads);
    assert_eq!(maps, 2, "两张地图（解开的目录树也算）");
    assert_eq!(mods, 2, "两个模组 —— Alenger 那些文件要归成一个");
}
#[test]
fn different_owners_do_not_clobber_each_other() {
    use crate::library::install::{Manifest, Owner, Plan};

    let fixture = fixture();
    let data = fixture.library.root();
    let work = fixture.work.path();

    let a = work.join("a.SC2Map");
    let b = work.join("b.SC2Map");
    std::fs::write(&a, b"aaa").expect("写");
    std::fs::write(&b, b"bbb").expect("写");

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);

    // 战役 A 铺一张地图
    let mut plan_a = Plan::new(Owner::Campaign {
        slot: "wol".to_string(),
        variant: "v1".to_string(),
    });
    plan_a.push(&a, "Maps/Campaign/alpha.SC2Map");
    manifest
        .apply(data, &fixture.installation, &plan_a)
        .expect("装 A");

    // 战役 B（**另一个槽位**）也铺一张 —— 老代码会把 A 的整个撤掉
    let mut plan_b = Plan::new(Owner::Campaign {
        slot: "lotv".to_string(),
        variant: "v1".to_string(),
    });
    plan_b.push(&b, "Maps/Campaign/void/beta.SC2Map");
    let warnings = manifest
        .apply(data, &fixture.installation, &plan_b)
        .expect("装 B");
    assert!(warnings.is_empty(), "目标不冲突就不该报警：{warnings:?}");

    let alpha = fixture.installation.campaign_maps_root.join("alpha.SC2Map");
    let beta = fixture
        .installation
        .campaign_maps_root
        .join("void")
        .join("beta.SC2Map");
    assert!(alpha.is_file(), "A 的还在");
    assert!(beta.is_file(), "B 的也装上了 —— 两个槽位互不干扰");

    // 撤掉 A：只撤 A 的
    manifest
        .remove(
            data,
            &fixture.installation,
            &Owner::Campaign {
                slot: "wol".to_string(),
                variant: "v1".to_string(),
            },
        )
        .expect("撤 A");
    assert!(!alpha.exists(), "A 的该撤掉");
    assert!(beta.is_file(), "B 的一个字节都不该动");
}

#[test]
fn conflicting_targets_warn_instead_of_silently_overwriting() {
    use crate::library::install::{Manifest, Owner, Plan};

    let fixture = fixture();
    let data = fixture.library.root();
    let work = fixture.work.path();

    let a = work.join("a.SC2Mod");
    let b = work.join("b.SC2Mod");
    std::fs::write(&a, b"first").expect("写");
    std::fs::write(&b, b"second").expect("写");

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);

    // 战役要往 Mods/Shared.SC2Mod 放
    let mut plan_a = Plan::new(Owner::Campaign {
        slot: "wol".to_string(),
        variant: "v1".to_string(),
    });
    plan_a.push(&a, "Mods/Shared.SC2Mod");
    manifest
        .apply(data, &fixture.installation, &plan_a)
        .expect("装 A");

    // 独立模组也要往**同一个位置**放 —— 应当给出警告，而不是静默覆盖
    let mut plan_b = Plan::new(Owner::Mod {
        id: "m1".to_string(),
    });
    plan_b.push(&b, "Mods/Shared.SC2Mod");
    let warnings = manifest
        .apply(data, &fixture.installation, &plan_b)
        .expect("装 B");

    assert_eq!(warnings.len(), 1, "抢同一个位置要说一声：{warnings:?}");
    assert!(warnings[0].contains("Mods/Shared.SC2Mod"));

    // 后装的赢，内容确实是它的
    let content = std::fs::read(fixture.installation.mods_root.join("Shared.SC2Mod")).expect("读");
    assert_eq!(content, b"second");

    // A 已经不再拥有那个位置了（账已经转给 B）
    assert!(
        manifest
            .of(&Owner::Campaign {
                slot: "wol".to_string(),
                variant: "v1".to_string(),
            })
            .is_empty(),
        "位置被接管后，原来的 owner 不该还记着它"
    );
}
#[test]
fn a_custom_campaign_lands_in_custom_campaigns() {
    // 踩过的坑：require_slot 多加了 is_main() 过滤，custom 被挡在门外，
    // 导入自制战役直接报「未知的战役槽位：custom」。
    // 自制战役是独立的一类，**不该**再判断它属于哪部原版战役。
    let fixture = fixture();

    let zip = build_zip(
        fixture.work.path(),
        "custom.zip",
        &[
            (
                "MyCampaign/metadata.txt",
                "title=群友之战\nauthor=某人\ncampaign=custom\nversion=1.0\n",
            ),
            ("MyCampaign/maps/qunyou01.SC2Map", "stub"),
        ],
    );

    let variant =
        import(&fixture.library, &zip, "custom", Default::default()).expect("导入自制战役");
    assert_eq!(variant.version.as_deref(), Some("1.0"));

    // 自制战役排在槽位表的最后一项
    assert_eq!(slot(&fixture, "custom").variants.len(), 1);

    // 启用：要落到 Maps/CustomCampaigns/，**不是** Maps/Campaign/
    activate(
        &fixture.library,
        &fixture.installation,
        "custom",
        Some(&variant.id),
    )
    .expect("启用");

    // CustomCampaigns 下面那层目录用**版本目录名**（稳定、跟库里的对得上）
    let placed = fixture
        .installation
        .custom_campaigns_root
        .join(&variant.id)
        .join("qunyou01.SC2Map");
    assert!(
        placed.is_file(),
        "自制战役该进 CustomCampaigns，实际没有：{}",
        placed.display()
    );
    assert!(
        !fixture
            .installation
            .campaign_maps_root
            .join(&variant.id)
            .exists(),
        "自制战役绝不能进官方 Maps/Campaign"
    );
}
#[test]
fn an_explicit_slot_always_beats_auto_detection() {
    // 「用户说了算」的底线：从自制战役页点导入时传的是 custom，
    // 那就**必须**是自制战役 —— 包内声明了什么、识别链认出了什么都不算数。
    //
    // 踩过：用户在自制战役页导入一个没有元数据的包，识别链把它认成某部原版战役，
    // 战役就跑到那部战役底下去了，跟用户点的按钮完全对不上。
    let fixture = fixture();

    let zip = build_zip(
        fixture.work.path(),
        "looks-like-lotv.zip",
        &[
            (
                "Pack/metadata.txt",
                "title=看着像虚空之遗\nauthor=某人\ncampaign=lotv\nversion=1.0\n",
            ),
            ("Pack/maps/void01.SC2Map", "stub"),
        ],
    );

    let variant = import(&fixture.library, &zip, "custom", Default::default()).expect("导入");

    assert_eq!(
        slot(&fixture, "custom").variants.len(),
        1,
        "用户选的自制战役"
    );
    assert!(
        slot(&fixture, "lotv").variants.is_empty(),
        "包内写着 lotv 也不能自己跑过去"
    );

    // 反过来：不指定槽位、由调用方按识别结果传 lotv，那就该进 lotv
    let zip2 = build_zip(
        fixture.work.path(),
        "really-lotv.zip",
        &[
            (
                "Pack2/metadata.txt",
                "title=确实虚空\nauthor=某人\ncampaign=lotv\nversion=1.0\n",
            ),
            ("Pack2/maps/void02.SC2Map", "stub"),
        ],
    );
    import(&fixture.library, &zip2, "lotv", Default::default()).expect("导入 2");
    assert_eq!(slot(&fixture, "lotv").variants.len(), 1);
    let _ = variant;
}
#[test]
fn a_root_level_entry_map_is_picked_automatically() {
    let entry = |path: &str, name: &str| MapEntry {
        path: path.to_string(),
        name: name.to_string(),
        chapter: None,
        size: 0,
        is_main: false,
    };

    // **子目录里**那张叫 Launcher 的排在前面 —— 就是要证明它不会被选中。
    // 条件必须是「在包根 + 名字像入口」，放宽一点点就会把普通关卡认成入口。
    let maps = vec![
        entry("1. Rebel Yell/Launcher.SC2Map", "Launcher"),
        entry("1. Rebel Yell/Terran01.SC2Map", "Terran01"),
        entry("SCMR Campaign Launcher.SC2Map", "SCMR Campaign Launcher"),
    ];

    let choice = resolve_main_map(&maps, None);
    assert_eq!(
        choice.path.as_deref(),
        Some("SCMR Campaign Launcher.SC2Map"),
        "包根的入口地图应当被认出来（真实样本就是 SCMR 那张）"
    );
    assert!(choice.automatic, "是替你选的，用户一次点击就能改");
    assert!(choice.warning.is_none());

    // 声明了主地图时，**声明优先于入口启发式**
    let declared = resolve_main_map(&maps, Some("1. Rebel Yell/Terran01.SC2Map"));
    assert_eq!(
        declared.path.as_deref(),
        Some("1. Rebel Yell/Terran01.SC2Map")
    );
    assert!(!declared.automatic);

    // 没有入口地图、也没声明 -> 还是不猜
    let plain = vec![
        entry("1. Rebel Yell/Terran01.SC2Map", "Terran01"),
        entry("2. Overmind/Zerg01.SC2Map", "Zerg01"),
    ];
    let unset = resolve_main_map(&plain, None);
    assert_eq!(unset.path, None);
    assert!(!unset.automatic);
}
#[test]
fn a_package_shaped_like_the_game_root_keeps_map_folders() {
    use crate::campaign::package::{PayloadTarget, payload_target};

    // 复刻战役 SCMR 那种摆法：**包根直接有 Mods/**，兄弟目录
    // Starcraft Mass Recall/ 里是分章节的地图。
    //
    // 它的启动器地图里写的是
    //   GameSetNextMap("Starcraft Mass Recall/1. Rebel Yell/Terran01")
    // —— 这个路径**相对 Maps/**。所以装完必须长成
    //   Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map
    //
    // 踩过：这些地图被当成「作者的分类目录」拍平成 Maps/Campaign/Terran01.SC2Map，
    // 层级一没，启动器地图就联动不了任何关卡 —— 用户看到的是
    // 「能打开启动器，但点哪一关都进不去」。
    let mirrored = payload_target(
        "Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
        false,
        "",
        true,
    );
    assert_eq!(
        mirrored,
        PayloadTarget::Mirror {
            path: "Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map".to_string(),
        },
        "根上有 Mods/ 就说明作者按游戏根目录摆的，结构和名字都要原样保留"
    );

    // 启动器地图自己也在同一层
    let launcher = payload_target(
        "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
        false,
        "",
        true,
    );
    assert_eq!(
        launcher,
        PayloadTarget::Mirror {
            path: "Maps/Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map".to_string(),
        }
    );

    // **没有**那个信号时维持原样：交给分类目录规则处理（只取文件名），
    // 免得把作者随手起的分组目录硬塞进游戏目录
    let plain = payload_target("maps/a.SC2Map", false, "", false);
    assert_eq!(
        plain,
        PayloadTarget::Map {
            name: "a.SC2Map".to_string(),
        },
        "没有 Mods/ 信号就不敢动结构，按老规矩拍平"
    );
}
