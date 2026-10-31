//! 模组挂载配置、按文件夹归并，以及启动时的自动挂载同步。

use super::fixture::*;
use crate::library::{ImportMode, activate, import};

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

    let rows = group_mods(&payloads, &["Alenger".to_string()], &[]);
    assert_eq!(rows.len(), 2, "三个文件应当归成一个模组");
    assert_eq!(rows[0].name, "Alenger");
    assert_eq!(rows[0].parts, 3);
    assert!(rows[0].mounted);
    assert!(!rows[0].in_game, "还没铺进游戏目录");
    assert_eq!(rows[1].name, "孤零零");
    assert_eq!(rows[1].parts, 1);
    assert!(!rows[1].mounted);
}

/// 没启用的战役版本：**配置里的挂载照样是勾上的**，但文件还没铺进游戏目录。
///
/// 这条以前断言「没启用 = 显示未挂载」，结果用户勾的模组下次读回来又变未勾，
/// 启动前的挂载检查也跟着把「配置里全挂着」误判成「一个都没挂」（issue 反馈）。
/// 现在用 `in_game` 表达「真的铺了没」。
#[test]
fn inactive_variant_reports_configured_mounts_but_not_in_game() {
    let fixture = fixture();
    let path = build_zip(
        fixture.work.path(),
        "with-mod.zip",
        &[
            (
                "Reborn/metadata.txt",
                "title=带模组\nauthor=A\ncampaign=WOL\nversion=1.0\n",
            ),
            ("Reborn/maps/a.SC2Map", "stub"),
            ("Reborn/Mods/Kit.SC2Mod/Base.SC2Data/x", "stub"),
        ],
    );

    let variant = import(&fixture.library, &path, "wol", ImportMode::Rename).expect("import");
    let rows = fixture.library.variant_mods("wol", &variant.id);
    assert!(!rows.is_empty(), "这个包里确实带了模组");
    assert!(
        rows.iter().all(|row| row.mounted),
        "没配过（mounted_mods = None）就是全挂，勾选状态不该被启用状态吞掉：{rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.in_game),
        "没启用的版本不该有文件铺进游戏目录：{rows:?}"
    );
}

/// 自动挂载**不抢**用户手动放的模组：账上没有、盘上已有一份，就跳过并提示。
///
/// 现实场景：installed.json 丢了 / 读坏了（读坏会被当空清单，见 install.rs），
/// 而游戏目录里已经躺着同一个模组 —— 那是用户自己的东西，不能悄悄覆盖。
#[test]
fn startup_sync_does_not_clobber_unmanaged_files() {
    let fixture = fixture();
    let zip = build_zip(
        fixture.work.path(),
        "manual-clash.zip",
        &[
            (
                "MyCampaign/metadata.txt",
                "title=手动冲突\nauthor=A\ncampaign=custom\nversion=1.0\n",
            ),
            ("MyCampaign/maps/a.SC2Map", "stub"),
            ("MyCampaign/Mods/Kit.SC2Mod/Base.SC2Data/x", "stub"),
        ],
    );
    let variant = import(&fixture.library, &zip, "custom", ImportMode::Rename).expect("import");
    activate(
        &fixture.library,
        &fixture.installation,
        "custom",
        Some(&variant.id),
    )
    .expect("activate");

    // 清单丢了（等价于读坏后被当空清单）
    std::fs::remove_file(fixture.library.root().join("installed.json")).expect("删清单");

    // 用户手动放了一份同名模组
    let manual = fixture.installation.root.join("Mods").join("Kit.SC2Mod");
    std::fs::create_dir_all(&manual).expect("建手动目录");
    std::fs::write(manual.join("mine.txt"), b"manual").expect("写手动文件");

    let warnings = crate::library::activation::sync_mods(&fixture.library, &fixture.installation)
        .expect("自动挂载");
    assert!(
        !warnings.is_empty(),
        "被手动放的文件占着时应当有提示：{warnings:?}"
    );
    assert!(
        manual.join("mine.txt").is_file(),
        "自动挂载不该动用户手动放的文件"
    );
}

/// 启动时的自动挂载：模组被手动删掉后自动补回；取消挂载后自动从游戏目录撤掉。
#[test]
fn startup_sync_repairs_missing_mods_and_respects_unmount() {
    let fixture = fixture();
    let zip = build_zip(
        fixture.work.path(),
        "with-mod.zip",
        &[
            (
                "MyCampaign/metadata.txt",
                "title=带模组\nauthor=A\ncampaign=custom\nversion=1.0\n",
            ),
            ("MyCampaign/maps/a.SC2Map", "stub"),
            ("MyCampaign/Mods/Kit.SC2Mod/Base.SC2Data/x", "stub"),
        ],
    );

    let variant = import(&fixture.library, &zip, "custom", ImportMode::Rename).expect("import");
    activate(
        &fixture.library,
        &fixture.installation,
        "custom",
        Some(&variant.id),
    )
    .expect("activate");

    let mod_dir = fixture.installation.root.join("Mods").join("Kit.SC2Mod");
    assert!(
        mod_dir.join("Base.SC2Data").join("x").is_file(),
        "启用后模组应当铺进 Mods/"
    );

    // 手动删掉（重装游戏 / 用户清理）—— 启动时自动补回来，不用再点一次启用
    std::fs::remove_dir_all(&mod_dir).expect("删掉模组");
    let warnings = crate::library::activation::sync_mods(&fixture.library, &fixture.installation)
        .expect("自动挂载");
    assert!(warnings.is_empty(), "正常补挂不该有提示：{warnings:?}");
    assert!(
        mod_dir.join("Base.SC2Data").join("x").is_file(),
        "自动挂载应把模组补回游戏目录"
    );

    // 取消挂载 -> 下次启动把它从游戏目录撤掉
    fixture
        .library
        .set_mounted_mods("custom", &variant.id, &[])
        .expect("取消挂载");
    crate::library::activation::sync_mods(&fixture.library, &fixture.installation).expect("同步");
    assert!(!mod_dir.exists(), "取消挂载后模组不该继续留在游戏目录");
}
