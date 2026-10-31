//! 独立模组导入流水线：暂存、去重、兄弟互斥、目录形态。

use super::fixture::*;

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

/// 「作为独立改版」导入的副本 modid 不同（X#alt），落点却是同一个
/// Mods/<folder> —— 两份都开着只会互相覆盖，让位规则必须按落点也比一次。
#[test]
fn copies_of_one_mod_cannot_be_enabled_together() {
    use crate::library::mods::{self, ModImportMode};

    let fixture = fixture();
    let data = fixture.library.root();

    let src = fixture.work.path().join("Alenger");
    std::fs::create_dir_all(&src).expect("建模组目录");
    std::fs::write(src.join("1钢铁.SC2Mod"), b"one").expect("写文件");

    let first = mods::import(
        data,
        &src,
        Some("demo.mod"),
        Some("1.0"),
        ModImportMode::Auto,
    )
    .expect("导入");
    let second = mods::import(
        data,
        &src,
        Some("demo.mod"),
        Some("2.0"),
        ModImportMode::Separate,
    )
    .expect("作为独立改版再导一份");

    assert_ne!(first.record.id, second.record.id);
    assert_eq!(
        mods::placed_name(&first.record),
        mods::placed_name(&second.record),
        "两份抢的是同一个 Mods/<folder>"
    );

    mods::set_enabled(data, &first.record.id, true).expect("启用第一份");
    mods::set_enabled(data, &second.record.id, true).expect("启用第二份");

    let on = mods::list(data).iter().filter(|item| item.enabled).count();
    assert_eq!(on, 1, "同一个落点上只能开一份");
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
