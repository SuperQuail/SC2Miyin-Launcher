//! 导入、槽位落点与封面。

use super::fixture::*;
use crate::library::{ImportMode, activate, import, import_with_password};

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

/// 带进化任务的虫群之心改版（真实样本：AI盟友虫群之心）：
/// 包内顶层 `evolution/…` 必须落到 `Maps/Campaign/swarm/evolution/`，
/// 不能和普通关卡一起拍进 swarm 根。
#[test]
fn hots_evolution_subfolder_keeps_its_directory() {
    let fixture = fixture();
    let path = build_zip(
        fixture.work.path(),
        "ai-hots.zip",
        &[
            (
                "metadata.txt",
                "title=AI盟友虫群之心\nauthor=Ylvae\ncampaign=HOTS\nversion=4\n",
            ),
            ("zchar01.SC2Map/Base.SC2Data/x", "stub"),
            ("evolution/zevolutionzergling.SC2Map/Base.SC2Data/x", "stub"),
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

    let swarm = fixture.installation.campaign_maps_root.join("swarm");
    assert!(
        swarm
            .join("zchar01.SC2Map")
            .join("Base.SC2Data")
            .join("x")
            .is_file(),
        "普通关卡应当落到 Maps/Campaign/swarm/"
    );
    assert!(
        swarm
            .join("evolution")
            .join("zevolutionzergling.SC2Map")
            .join("Base.SC2Data")
            .join("x")
            .is_file(),
        "进化任务必须落到 Maps/Campaign/swarm/evolution/，而不是 swarm 根"
    );
}

/// 「从文件夹导入」：用户选的是**包外面那一层**，唯一的子目录才是包。
///
/// 铺出来必须是 `maps/a.SC2Map`，不能带着 `MyCampaign/` 那层壳（游戏扫不到）。
#[test]
fn imports_a_folder_source() {
    let fixture = fixture();

    let root = fixture.work.path().join("root");
    let pack = root.join("MyCampaign");
    std::fs::create_dir_all(pack.join("maps")).expect("mkdir");
    std::fs::write(
        pack.join("metadata.txt"),
        "title=文件夹战役\nauthor=Creator\n",
    )
    .expect("write");
    std::fs::write(pack.join("maps/a.SC2Map"), b"stub").expect("write");

    let variant =
        import(&fixture.library, &root, "custom", ImportMode::Rename).expect("从文件夹导入");

    assert_eq!(variant.name, "文件夹战役");
    let dir = fixture.library.slot_dir("custom").join(&variant.id);
    assert!(dir.join("maps/a.SC2Map").is_file(), "地图要落在版本目录里");
    assert!(!dir.join("MyCampaign").exists(), "包外面那层不能跟着铺进去");
}

/// 加密包：没密码时报错且**不留下半个版本**；密码对了才进库。
#[test]
fn imports_an_encrypted_package_with_password() {
    let fixture = fixture();
    let path = encrypted_package(&fixture, "locked.zip", "miyin", "加密包");

    assert!(
        import(&fixture.library, &path, "wol", ImportMode::Rename).is_err(),
        "没密码必须拒绝"
    );
    assert!(
        slot(&fixture, "wol").variants.is_empty(),
        "失败不能留下记录"
    );

    let variant = import_with_password(
        &fixture.library,
        &path,
        "wol",
        ImportMode::Rename,
        Some("miyin"),
    )
    .expect("带密码导入");

    assert_eq!(variant.name, "加密包");
    assert_eq!(variant.map_count, 1);
    let dir = fixture.library.slot_dir("wol").join(&variant.id);
    assert!(dir.join("maps/01.SC2Map").is_file(), "内容要真的解出来");
}
