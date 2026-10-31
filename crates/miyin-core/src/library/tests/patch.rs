//! 补丁叠加、优先级、导出回环与补丁元数据编辑。

use super::fixture::*;
use crate::campaign::package;
use crate::library::{ImportMode, activate, compose, import, patch};

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
            format: crate::library::export::ExportFormat::Miyin,
        },
    )
    .expect("export");

    assert_eq!(report.maps, 2);
    assert_eq!(report.mods, 1);
    assert_eq!(report.patches, vec!["p".to_string()], "补丁应当被一起带走");

    // 用启动器自己的预检读回来：必须仍被认成 CCM 包、归属不变
    let back = package::inspect(&destination).expect("inspect export");
    // 弥音包现在多写一份根 metadata.json（我们的标准格式，读回时保真），
    // 所以启动器自己认 Standard；CCM 那边仍然读 metadata.txt（下面会断言它在）。
    assert_eq!(
        back.format,
        crate::campaign::CampaignFormat::Standard,
        "启动器读根 metadata.json（标准格式）"
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
            format: crate::library::export::ExportFormat::Miyin,
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
            format: crate::library::export::ExportFormat::Miyin,
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
