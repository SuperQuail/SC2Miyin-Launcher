//! 战役库的端到端测试：导入 → 启用 → 切换 → 还原。

use std::path::{Path, PathBuf};

use crate::campaign::package;
use crate::library::{
    ImportMode, Library, VariantChanges, VersionRelation, activate, compare_versions, compose,
    conflict_for, import, patch, remove_variant, update_variant,
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
