//! 启用 / 停用 / 切换 / 回滚，以及两个战役抢同一个目标时的激活冲突。

use super::fixture::*;
use crate::library::{ImportMode, activate, conflicts, import, remove_variant};
use std::path::PathBuf;

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

/// 位置撞车时只让出**那一条**，不是把别人的整套文件都撤掉 ——
/// 全撤的话，库里还写着「启用中」，游戏目录里却已经空了。
#[test]
fn a_conflict_takes_over_one_path_not_the_whole_owner() {
    use crate::library::install::{Manifest, Owner, Plan};

    let fixture = fixture();
    let data = fixture.library.root();
    let work = fixture.work.path();

    let only = work.join("wol-only.SC2Mod");
    let shared = work.join("shared.SC2Mod");
    let newcomer = work.join("new.SC2Mod");
    std::fs::write(&only, b"wol").expect("写");
    std::fs::write(&shared, b"shared").expect("写");
    std::fs::write(&newcomer, b"new").expect("写");

    let campaign = Owner::Campaign {
        slot: "wol".to_string(),
        variant: "v1".to_string(),
    };
    let mut manifest = Manifest::load_migrating(data, &fixture.installation);

    // 战役占两个位置
    let mut plan_a = Plan::new(campaign.clone());
    plan_a.push(&only, "Mods/WolOnly.SC2Mod");
    plan_a.push(&shared, "Mods/Shared.SC2Mod");
    manifest
        .apply(data, &fixture.installation, &plan_a)
        .expect("装 A");

    // 独立模组只抢其中一个
    let mut plan_b = Plan::new(Owner::Mod {
        id: "m1".to_string(),
    });
    plan_b.push(&newcomer, "Mods/Shared.SC2Mod");
    let warnings = manifest
        .apply(data, &fixture.installation, &plan_b)
        .expect("装 B");
    assert_eq!(warnings.len(), 1, "抢同一个位置要说一声");

    let mods = fixture.installation.mods_root.clone();
    assert_eq!(
        std::fs::read(mods.join("Shared.SC2Mod")).expect("读"),
        b"new",
        "后装的赢"
    );
    assert!(
        mods.join("WolOnly.SC2Mod").is_file(),
        "撞车只让出一条，别人其余的还在"
    );
    assert_eq!(manifest.of(&campaign).len(), 1, "账上也只剩那一条");
}

/// 造一个「带同一个模组」的战役包；槽位决定 metadata 里声明的资料片。
fn conflict_package(
    fixture: &Fixture,
    file: &str,
    slot: &str,
    title: &str,
    mod_dir: &str,
) -> PathBuf {
    let campaign = if slot == "hots" { "HOTS" } else { "WOL" };
    let owned = [
        (
            "Reborn/metadata.txt".to_string(),
            format!("title={title}\nauthor=Creator\ncampaign={campaign}\nversion=1.0\n"),
        ),
        ("Reborn/maps/a.SC2Map".to_string(), "stub".to_string()),
        (
            format!("Reborn/Mods/{mod_dir}/Base.SC2Data/x"),
            "stub".to_string(),
        ),
    ];
    let borrowed: Vec<(&str, &str)> = owned
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();
    build_zip(fixture.work.path(), file, &borrowed)
}

#[test]
fn activation_conflicts_report_targets_owned_by_another_active_campaign() {
    let fixture = fixture();
    let wol = import(
        &fixture.library,
        &conflict_package(
            &fixture,
            "wol.zip",
            "wol",
            "自由之翼：重生",
            "Shared.SC2Mod",
        ),
        "wol",
        ImportMode::Rename,
    )
    .expect("import wol");
    let hots = import(
        &fixture.library,
        &conflict_package(
            &fixture,
            "hots.zip",
            "hots",
            "虫群之心：进化",
            "Shared.SC2Mod",
        ),
        "hots",
        ImportMode::Rename,
    )
    .expect("import hots");

    // 一个都还没启用：谁也没占位置
    assert!(
        conflicts(&fixture.library, &fixture.installation, "hots", &hots.id)
            .expect("check")
            .is_empty(),
        "没启用任何东西时不该报冲突"
    );

    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&wol.id),
    )
    .expect("activate wol");
    // 自由之翼那个版本此时已经把 Mods/Shared.SC2Mod 铺进游戏目录了
    assert!(
        fixture
            .installation
            .mods_root
            .join("Shared.SC2Mod")
            .exists(),
        "前提：启用 wol 之后那个模组确实在游戏目录里"
    );

    let found =
        conflicts(&fixture.library, &fixture.installation, "hots", &hots.id).expect("check");
    assert_eq!(found.len(), 1, "应当正好报一条冲突：{found:?}");
    assert_eq!(found[0].target, "Mods/Shared.SC2Mod");
    assert_eq!(found[0].slot, "wol");
    assert_eq!(found[0].slot_name, "自由之翼");
    assert_eq!(found[0].variant, wol.id);
    assert_eq!(found[0].variant_name, "自由之翼：重生");

    // **检查本身不写盘**：虫群之心的东西一个都不该出现，启用状态也没变
    assert!(
        !fixture
            .installation
            .mods_root
            .join("Evolution.SC2Mod")
            .exists(),
        "只是查了一下，不该往游戏目录写东西"
    );
    assert!(
        slot(&fixture, "hots").active.is_none(),
        "查冲突不该改启用状态"
    );

    // 真的启用虫群之心之后：那个位置归它自己了，再查就不算冲突
    activate(
        &fixture.library,
        &fixture.installation,
        "hots",
        Some(&hots.id),
    )
    .expect("activate hots");
    assert!(
        conflicts(&fixture.library, &fixture.installation, "hots", &hots.id)
            .expect("check")
            .is_empty(),
        "自己上次留下的不算冲突"
    );
}

#[test]
fn activation_conflicts_only_consider_what_is_actually_active() {
    let fixture = fixture();
    // 同一个槽位里两个版本：A 带 Shared 模组，B 带别的
    let a = import(
        &fixture.library,
        &conflict_package(&fixture, "a.zip", "wol", "重生 A", "Shared.SC2Mod"),
        "wol",
        ImportMode::Rename,
    )
    .expect("import a");
    let b = import(
        &fixture.library,
        &conflict_package(&fixture, "b.zip", "wol", "重生 B", "Other.SC2Mod"),
        "wol",
        ImportMode::Rename,
    )
    .expect("import b");
    let hots = import(
        &fixture.library,
        &conflict_package(&fixture, "hots.zip", "hots", "进化", "Shared.SC2Mod"),
        "hots",
        ImportMode::Rename,
    )
    .expect("import hots");

    // A 启用时确实会撞
    activate(&fixture.library, &fixture.installation, "wol", Some(&a.id)).expect("activate a");
    assert_eq!(
        conflicts(&fixture.library, &fixture.installation, "hots", &hots.id)
            .expect("check")
            .len(),
        1
    );

    // 换成 B：A 的东西已经撤下去了 —— 库里 A 还在，但它没启用，不该再报冲突
    activate(&fixture.library, &fixture.installation, "wol", Some(&b.id)).expect("activate b");
    assert!(
        conflicts(&fixture.library, &fixture.installation, "hots", &hots.id)
            .expect("check")
            .is_empty(),
        "没启用的版本不占任何位置，不该报冲突"
    );
}

#[test]
fn activation_conflicts_compare_paths_the_way_the_platform_does() {
    let fixture = fixture();
    // 同一个模组，只有大小写不同
    let wol = import(
        &fixture.library,
        &conflict_package(&fixture, "wol.zip", "wol", "重生", "Shared.SC2Mod"),
        "wol",
        ImportMode::Rename,
    )
    .expect("import wol");
    let hots = import(
        &fixture.library,
        &conflict_package(&fixture, "hots.zip", "hots", "进化", "shared.sc2mod"),
        "hots",
        ImportMode::Rename,
    )
    .expect("import hots");

    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&wol.id),
    )
    .expect("activate wol");
    let found =
        conflicts(&fixture.library, &fixture.installation, "hots", &hots.id).expect("check");
    if cfg!(windows) {
        assert_eq!(
            found.len(),
            1,
            "Windows 不区分大小写，这是同一个位置：{found:?}"
        );
        assert_eq!(found[0].target, "Mods/shared.sc2mod");
    } else {
        assert!(found.is_empty(), "区分大小写的平台上这是两个位置");
    }
}

#[test]
fn same_path_follows_platform_rules() {
    // 直接钉住判据本身：别哪天顺手把大小写折叠的规则改了
    use crate::library::activation::same_path;
    assert_eq!(
        same_path("Mods/Foo.SC2Mod", "mods/foo.sc2mod"),
        cfg!(windows)
    );
    assert!(same_path("Mods/Foo.SC2Mod", "Mods/Foo.SC2Mod"));
    assert!(!same_path("Mods/Foo.SC2Mod", "Mods/Bar.SC2Mod"));
}
