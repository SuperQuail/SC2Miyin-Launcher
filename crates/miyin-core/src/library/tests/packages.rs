//! 包归属判定、镜像布局、预演与写盘闸门。

use super::fixture::*;
use crate::library::{MapEntry, activate, import, remove_variant, resolve_main_map};

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

#[test]
fn deleting_one_slots_variant_leaves_the_other_slot_alone() {
    // 踩过：remove_variant 里为了「删正在启用的版本」调了 deactivate，
    // 而统一安装引擎之后 deactivate 的含义变成了「撤掉**所有**战役」——
    // 于是删自由之翼的一个版本，把虫群之心也一起卸了。
    let fixture = fixture();

    let wol_zip = build_zip(
        fixture.work.path(),
        "wol.zip",
        &[
            (
                "WolPack/metadata.txt",
                "title=自由改版\nauthor=某人\ncampaign=WOL\nversion=1.0\n",
            ),
            ("WolPack/maps/wol01.SC2Map", "stub"),
        ],
    );
    let hots_zip = build_zip(
        fixture.work.path(),
        "hots.zip",
        &[
            (
                "HotsPack/metadata.txt",
                "title=虫心改版\nauthor=某人\ncampaign=HOTS\nversion=1.0\n",
            ),
            ("HotsPack/maps/hots01.SC2Map", "stub"),
        ],
    );

    let wol = import(&fixture.library, &wol_zip, "wol", Default::default()).expect("导入 wol");
    let hots = import(&fixture.library, &hots_zip, "hots", Default::default()).expect("导入 hots");

    activate(
        &fixture.library,
        &fixture.installation,
        "wol",
        Some(&wol.id),
    )
    .expect("启用 wol");
    activate(
        &fixture.library,
        &fixture.installation,
        "hots",
        Some(&hots.id),
    )
    .expect("启用 hots");

    let wol_map = fixture.installation.campaign_maps_root.join("wol01.SC2Map");
    let hots_map = fixture
        .installation
        .campaign_maps_root
        .join("swarm")
        .join("hots01.SC2Map");
    assert!(wol_map.is_file(), "wol 的装上了");
    assert!(hots_map.is_file(), "hots 的也装上了");

    // 删掉正在启用的 wol 那一版
    remove_variant(&fixture.library, &fixture.installation, "wol", &wol.id).expect("删 wol");

    assert!(!wol_map.exists(), "wol 的该撤掉");
    assert!(
        hots_map.is_file(),
        "**hots 的不该受影响** —— 只撤被删的那个槽位"
    );
    assert_eq!(
        slot(&fixture, "hots").active.as_deref(),
        Some(hots.id.as_str()),
        "别的槽位仍然处于启用状态"
    );
}

#[test]
fn maps_outside_the_campaign_subdirs_are_allowed() {
    use crate::library::install::{Manifest, Owner, Plan};

    // 踩过：白名单原来只放行 Maps/Campaign 和 Maps/CustomCampaigns，
    // 而复刻战役 SCMR 整包要落在 Maps/Starcraft Mass Recall/…（它的启动器地图
    // 按这个**相对 Maps/** 的路径联动关卡）。地图放对了，却被自己的闸门拒绝，
    // 用户看到的是一句「拒绝往 … 写东西」。
    //
    // 白名单是 Maps / Mods / Interfaces 这三个**根**，不收窄到子目录。
    let fixture = fixture();
    let data = fixture.library.root();

    let source = fixture.work.path().join("Terran01.SC2Map");
    std::fs::write(&source, b"map").expect("写");

    let mut plan = Plan::new(Owner::Campaign {
        slot: "wol".to_string(),
        variant: "scmr".to_string(),
    });
    plan.push(
        &source,
        "Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
    );

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    manifest
        .apply(data, &fixture.installation, &plan)
        .expect("Maps/ 底下任意位置都该放行");

    assert!(
        fixture
            .installation
            .maps_root
            .join("Starcraft Mass Recall")
            .join("1. Rebel Yell")
            .join("Terran01.SC2Map")
            .is_file()
    );

    // 但游戏目录**之外**照样拒绝 —— 闸门还在
    let mut escape = Plan::new(Owner::Mod {
        id: "evil".to_string(),
    });
    escape.push(&source, "../别处/x.SC2Mod");
    assert!(
        manifest
            .apply(data, &fixture.installation, &escape)
            .is_err(),
        "越界路径必须拒绝"
    );
}

#[test]
fn opening_a_map_uses_the_installed_copy() {
    use crate::library::install::{Manifest, Owner, Plan};

    // 打开地图必须用**游戏目录里那一份**，不是库里那一份。
    //
    // 库里那份只是留底（切回原版时能还原）。而游戏和编辑器读的是装好的那份 ——
    // 复刻战役的启动器地图里写的是 GameSetNextMap("Starcraft Mass Recall/…")，
    // 这种路径**相对 Maps/**，只有装好的那份才在正确的位置上。
    // 实测从库里打开会报「无法打开地图」。
    let fixture = fixture();
    let data = fixture.library.root();

    let source = fixture.work.path().join("Terran01.SC2Map");
    std::fs::write(&source, b"map").expect("写");

    let mut plan = Plan::new(Owner::Campaign {
        slot: "wol".to_string(),
        variant: "remake".to_string(),
    });
    plan.push(
        &source,
        "Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
    );

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    manifest
        .apply(data, &fixture.installation, &plan)
        .expect("装");

    let installed = manifest
        .target_of(data, &fixture.installation, &source)
        .expect("该能反查到装到哪了")
        .expect("装好的那一份得在盘上");

    assert_eq!(
        installed,
        fixture
            .installation
            .maps_root
            .join("Starcraft Mass Recall")
            .join("1. Rebel Yell")
            .join("Terran01.SC2Map"),
        "要指向游戏目录里那一份"
    );

    // 没装过的路径反查不出来 —— 调用方会退回用库里那份
    assert!(
        manifest
            .target_of(
                data,
                &fixture.installation,
                &fixture.work.path().join("没有这个.SC2Map")
            )
            .expect("没装过不算对不上")
            .is_none()
    );
}

/// 预演要说清「这份计划会动到游戏目录里的哪些东西」，而且一个字都不写盘。
#[test]
fn preview_says_what_will_be_touched() {
    use crate::library::install::{Manifest, Owner, Plan};

    let fixture = fixture();
    let data = fixture.library.root();
    let work = fixture.work.path();

    let ours = work.join("ours.SC2Mod");
    let extra = work.join("extra.SC2Mod");
    std::fs::write(&ours, b"ours").expect("写");
    std::fs::write(&extra, b"extra").expect("写");

    // 游戏目录里先有一份「官方文件」，它会是被覆盖的那一类
    std::fs::create_dir_all(&fixture.installation.mods_root).expect("建模组目录");
    std::fs::write(
        fixture.installation.mods_root.join("Official.SC2Mod"),
        b"official",
    )
    .expect("写官方文件");

    let campaign = Owner::Campaign {
        slot: "wol".to_string(),
        variant: "v1".to_string(),
    };
    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    let mut plan = Plan::new(campaign.clone());
    plan.push(&ours, "Mods/Official.SC2Mod");
    plan.push(&extra, "Mods/New.SC2Mod");

    // 还没装：一条覆盖、一条新增
    let fresh = manifest
        .preview(&fixture.installation, &plan)
        .expect("预演");
    assert_eq!(fresh.add.len(), 1, "New.SC2Mod 是新的");
    assert_eq!(fresh.overwrite.len(), 1, "Official.SC2Mod 会被备份后覆盖");
    assert_eq!(fresh.takeover.len(), 0);
    assert_eq!(fresh.bytes, 9, "ours(4) + extra(5)");

    manifest
        .apply(data, &fixture.installation, &plan)
        .expect("装");

    // 装完再预演同一份计划：两条都成了「重铺我们自己的」，不算接管
    let again = manifest
        .preview(&fixture.installation, &plan)
        .expect("预演");
    assert_eq!(again.add.len(), 0);
    assert_eq!(again.overwrite.len(), 2);
    assert_eq!(again.takeover.len(), 0);
    assert!(again.delete.is_empty());

    // 换成只装一条：另一条会被撤掉
    let mut smaller = Plan::new(campaign.clone());
    smaller.push(&ours, "Mods/Official.SC2Mod");
    let fewer = manifest
        .preview(&fixture.installation, &smaller)
        .expect("预演");
    assert_eq!(fewer.delete, vec!["Mods/New.SC2Mod".to_string()]);

    // 别人想抢同一个位置：算接管，界面上要说清是谁让位
    let mut other = Plan::new(Owner::Mod {
        id: "m1".to_string(),
    });
    other.push(&extra, "Mods/Official.SC2Mod");
    let takeover = manifest
        .preview(&fixture.installation, &other)
        .expect("预演");
    assert_eq!(takeover.takeover.len(), 1);
    assert_eq!(takeover.add.len(), 0);
    assert_eq!(
        takeover.takeover[0].owner.as_ref().map(|owner| owner.key()),
        Some("campaign-wol-v1".to_string()),
        "要说清现在归谁"
    );
}

/// 落点只要求「在游戏目录里」：不在 Maps / Mods / Interfaces 里也允许，出目录一律拒绝。
#[test]
fn anything_inside_the_game_folder_can_be_written() {
    use crate::library::install::{Manifest, Owner, Plan};

    let fixture = fixture();
    let data = fixture.library.root();
    let work = fixture.work.path();

    let blob = work.join("blob.bin");
    std::fs::write(&blob, b"blob").expect("写");

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    let mut plan = Plan::new(Owner::Mod {
        id: "m1".to_string(),
    });
    plan.push(&blob, "SC2Data/thing.bin");
    plan.push(&blob, "root-level.txt");
    manifest
        .apply(data, &fixture.installation, &plan)
        .expect("装");

    assert!(
        fixture
            .installation
            .root
            .join("SC2Data/thing.bin")
            .is_file()
    );
    assert!(fixture.installation.root.join("root-level.txt").is_file());

    let mut escape = Plan::new(Owner::Mod {
        id: "m2".to_string(),
    });
    escape.push(&blob, "../outside.bin");
    let error = manifest
        .apply(data, &fixture.installation, &escape)
        .expect_err("出游戏目录必须拒绝");
    assert!(
        format!("{error}").contains("只能写星际争霸安装目录里面的文件"),
        "{error}"
    );
}

/// 清单说装了、磁盘上却没有：反查必须报「对不上」，不能当成没装静默退回库里那份。
#[test]
fn a_manifest_record_without_the_file_is_reported() {
    use crate::error::Error;
    use crate::library::install::{Installed, Manifest, Owner};

    let fixture = fixture();
    let data = fixture.library.root();
    let source = data.join("campaigns/wol/重生/gone.SC2Map");
    std::fs::create_dir_all(source.parent().expect("父目录")).expect("建目录");
    std::fs::write(&source, b"map").expect("写地图");

    // 手写一条记录：清单说它装进了游戏目录，游戏目录里其实没有它
    let mut manifest = Manifest::default();
    manifest.files.push(Installed {
        target: "Maps/Campaign/gone.SC2Map".to_string(),
        owner: Owner::Campaign {
            slot: "wol".to_string(),
            variant: "重生".to_string(),
        },
        backup: None,
        source: Some("campaigns/wol/重生/gone.SC2Map".to_string()),
    });

    let error = manifest
        .target_of(data, &fixture.installation, &source)
        .expect_err("账实不符必须报错，不能静默返回 None");
    assert!(matches!(error, Error::ManifestMismatch(_)), "{error:?}");
    assert!(
        error.to_string().contains("Maps/Campaign/gone.SC2Map"),
        "要说清是哪个位置对不上：{error}"
    );
}

/// 接管别人的位置时，对方那份删不掉**必须报错** —— 以前 `let _ =` 吞掉失败，
/// 清单说让出了、文件还在，账实就此永久分叉。
#[cfg(windows)]
#[test]
fn takeover_reports_a_target_it_cannot_release() {
    use crate::library::install::{Installed, Manifest, Owner, Plan};

    let fixture = fixture();
    let data = fixture.library.root();
    let target = fixture
        .installation
        .root
        .join("Maps")
        .join("Campaign")
        .join("locked.SC2Map");
    std::fs::create_dir_all(target.parent().expect("父目录")).expect("建目录");
    std::fs::write(&target, b"other").expect("写");

    // 不共享句柄 = 别人删不掉它，正是「游戏正跑着占着文件」的现场（ERROR_SHARING_VIOLATION）
    use std::os::windows::fs::OpenOptionsExt;
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&target)
        .expect("锁住文件");

    let mut manifest = Manifest::default();
    manifest.files.push(Installed {
        target: "Maps/Campaign/locked.SC2Map".to_string(),
        owner: Owner::Mod {
            id: "other".to_string(),
        },
        backup: None,
        source: None,
    });

    let source = fixture.work.path().join("mine.SC2Map");
    std::fs::write(&source, b"mine").expect("写");
    let mut plan = Plan::new(Owner::Campaign {
        slot: "wol".to_string(),
        variant: "v2".to_string(),
    });
    plan.push(&source, "Maps/Campaign/locked.SC2Map");

    let error = manifest
        .apply(data, &fixture.installation, &plan)
        .expect_err("让不出位置就得报错，不能装作接管成功");
    assert!(error.to_string().contains("locked.SC2Map"), "{error}");
    assert!(target.exists(), "删不掉的那份还得在盘上");

    // 放开句柄，tempdir 才清得掉
    drop(locked);
}
