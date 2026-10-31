//! 数据根解析、应用路径与旧 `data/` 迁移。

use std::path::{Path, PathBuf};

/// 建一个像样的游戏安装目录，返回它。
fn installation(base: &Path) -> PathBuf {
    let path = base.join("BLZ").join("StarCraft II");
    std::fs::create_dir_all(&path).expect("安装目录");
    path
}

/// 新数据根：安装目录的同级。
fn new_root(base: &Path) -> PathBuf {
    base.join("BLZ").join(crate::library::DATA_DIR_NAME)
}

/// 旧的绿色版数据目录：exe 同级的 `data/`。
fn legacy_dir(base: &Path) -> PathBuf {
    base.join("launcher").join("data")
}

/// 绿色版的启动器本体（只算路径，不建文件）。
fn exe(base: &Path) -> PathBuf {
    base.join("launcher").join("miyin-launcher.exe")
}

/// 数据根永远在**安装目录的同级**；没有安装目录就没有数据根，不再退回 exe 旁边。
#[test]
fn data_root_is_the_installation_sibling() {
    assert_eq!(
        crate::library::data_root(Path::new("D:/Game/BLZ/StarCraft II")),
        PathBuf::from("D:/Game/BLZ").join(crate::library::DATA_DIR_NAME)
    );
    // 相对路径也照算，不会滑到 exe 同级的 data/
    assert_eq!(
        crate::library::data_root(Path::new("StarCraft II")),
        PathBuf::from(crate::library::DATA_DIR_NAME)
    );
}

/// `app_paths` 只算路径 —— 不建目录、不迁移。
#[test]
fn app_paths_only_computes_paths() {
    let base = tempfile::tempdir().expect("临时目录");
    let installation = installation(base.path());

    let paths = crate::library::app_paths(&installation);
    let root = new_root(base.path());

    assert_eq!(paths.root, root);
    assert_eq!(paths.config, root.join("config.json"));
    assert_eq!(paths.network, root.join("network.json"));
    assert_eq!(paths.webview2, root.join("webview2"));
    assert_eq!(paths.temp, root.join("temp"));
    assert!(!paths.root.exists(), "只算路径，不该碰磁盘");
}

/// `prepare_app_paths`：建好整套目录，并把 exe 旁的旧 data/ 整体搬进来。
#[test]
fn prepare_creates_the_layout_and_moves_the_legacy_data() {
    let base = tempfile::tempdir().expect("临时目录");
    let installation = installation(base.path());
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(legacy.join("campaigns").join("wol")).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"{}").expect("索引");
    std::fs::write(legacy.join("campaigns").join("wol").join("a.txt"), b"hello").expect("版本内容");

    let paths =
        crate::library::prepare_app_paths(&installation, &exe(base.path())).expect("准备数据根");

    let root = new_root(base.path());
    assert_eq!(paths.root, root);
    assert!(!legacy.exists(), "搬完旧路径必须消失");
    assert_eq!(
        std::fs::read_to_string(root.join("library.json")).expect("索引"),
        "{}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("campaigns").join("wol").join("a.txt")).expect("内容"),
        "hello"
    );
    assert!(
        root.join("webview2").is_dir(),
        "WebView2 profile 目录要建好"
    );
    assert!(root.join("temp").is_dir(), "临时目录要建好");
    assert_eq!(
        paths.config.parent(),
        Some(root.as_path()),
        "config / network 的父目录就是数据根"
    );

    // 第二次启动：什么都不再搬，库原样。
    let again =
        crate::library::prepare_app_paths(&installation, &exe(base.path())).expect("再准备一次");
    assert_eq!(again, paths);
    assert_eq!(
        std::fs::read_to_string(root.join("library.json")).expect("索引"),
        "{}"
    );
}

/// 新根已经有库时，旧 data/ 一个字节都不动（数据是用户的，宁可让他自己看一眼）。
#[test]
fn prepare_does_not_move_legacy_when_the_new_root_has_a_library() {
    let base = tempfile::tempdir().expect("临时目录");
    let installation = installation(base.path());
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(&legacy).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"OLD").expect("旧索引");

    let root = new_root(base.path());
    std::fs::create_dir_all(&root).expect("新数据目录");
    std::fs::write(root.join("library.json"), b"NEW").expect("新索引");

    let paths =
        crate::library::prepare_app_paths(&installation, &exe(base.path())).expect("准备数据根");

    assert_eq!(paths.root, root);
    assert_eq!(
        std::fs::read_to_string(root.join("library.json")).expect("新索引"),
        "NEW"
    );
    assert_eq!(
        std::fs::read_to_string(legacy.join("library.json")).expect("旧索引还在"),
        "OLD",
        "旧数据既不该被覆盖也不该被删"
    );
}

/// 全新机器：没有旧 data/，直接用新根。
#[test]
fn fresh_install_uses_the_new_root_without_migrating() {
    let base = tempfile::tempdir().expect("临时目录");
    let installation = installation(base.path());

    let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
    assert_eq!(plan.root, new_root(base.path()));
    assert!(!plan.migrated);
    assert!(plan.legacy.is_none(), "没有旧数据时不该报 legacy");
}

/// 旧数据要**搬过去**，而且内容一个不少（搬完旧的不在原处）。
#[test]
fn migration_moves_legacy_data_and_keeps_the_content() {
    let base = tempfile::tempdir().expect("临时目录");
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(legacy.join("campaigns").join("wol")).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"{}").expect("索引");
    std::fs::write(legacy.join("campaigns").join("wol").join("a.txt"), b"hello").expect("版本内容");

    let installation = installation(base.path());

    let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
    assert!(plan.migrated, "应该搬过来：{}", plan.note);
    assert_eq!(plan.root, new_root(base.path()));
    assert!(!legacy.exists(), "旧的应该已经不在原处");
    assert_eq!(
        std::fs::read_to_string(plan.root.join("library.json")).expect("索引"),
        "{}"
    );
    assert_eq!(
        std::fs::read_to_string(plan.root.join("campaigns").join("wol").join("a.txt"))
            .expect("版本内容"),
        "hello"
    );
}

/// 新根已经有东西时**一律不动**：不覆盖、不合并、不删旧数据（数据是用户的）。
#[test]
fn migration_never_touches_an_existing_new_root() {
    let base = tempfile::tempdir().expect("临时目录");
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(&legacy).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"OLD").expect("旧索引");

    let installation = installation(base.path());
    let new_root = new_root(base.path());
    std::fs::create_dir_all(&new_root).expect("新数据目录");
    std::fs::write(new_root.join("library.json"), b"NEW").expect("新索引");

    let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
    assert!(!plan.migrated, "新根已经有东西，不许动");
    assert_eq!(
        std::fs::read_to_string(plan.root.join("library.json")).expect("新索引"),
        "NEW"
    );
    assert_eq!(
        std::fs::read_to_string(legacy.join("library.json")).expect("旧索引还在"),
        "OLD",
        "旧数据既不该被覆盖也不该被删"
    );
}

/// verifier 实测的那个坑：跨盘搬迁断在半路，新根留下**半份**（没有 library.json）。
/// 以前 `root.exists()` 直接认它 → 索引读成空 → 界面上整个库消失。
#[test]
fn half_copied_root_is_not_trusted() {
    let base = tempfile::tempdir().expect("临时目录");
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(legacy.join("campaigns").join("wol")).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"{\"slots\":{}}").expect("旧索引");

    let installation = installation(base.path());
    let root = new_root(base.path());
    std::fs::create_dir_all(root.join("campaigns")).expect("半份新根");
    std::fs::write(root.join("campaigns").join("half.txt"), b"half").expect("半份内容");

    let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
    assert!(plan.migrated, "半份不该被认成完整库：{}", plan.note);
    assert!(
        plan.root.join("library.json").is_file(),
        "完整的那份要搬进来"
    );
    assert!(!legacy.exists(), "旧数据搬走了");
    assert!(
        plan.note.contains("没完成"),
        "要说清上次没搬完：{}",
        plan.note
    );
}

/// 上一次留下 `<root>.partial`（搬到一半断电），下次启动要接着办完。
#[test]
fn partial_leftover_is_recovered_on_next_start() {
    let base = tempfile::tempdir().expect("临时目录");
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(&legacy).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"{}").expect("旧索引");

    let installation = installation(base.path());
    let partial = crate::library::data_root::partial_root(&new_root(base.path()));
    std::fs::create_dir_all(&partial).expect("上次的中转目录");
    std::fs::write(partial.join("junk.txt"), b"junk").expect("半份内容");

    let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
    assert!(plan.migrated, "{}", plan.note);
    assert!(plan.root.join("library.json").is_file());
    assert!(
        !crate::library::data_root::partial_root(&plan.root).exists(),
        "中转目录要清干净"
    );
}

/// 新根路径上是个**文件** —— 挪到一边留证，然后把旧数据迁进来（而不是退回旧根）。
#[test]
fn a_file_blocking_the_new_root_is_parked() {
    let base = tempfile::tempdir().expect("临时目录");
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(&legacy).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"{}").expect("旧索引");

    let installation = installation(base.path());
    std::fs::write(new_root(base.path()), b"not a dir").expect("占位文件");

    let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
    assert!(plan.migrated, "应该绕过挡路的文件并完成迁移：{}", plan.note);
    assert_eq!(plan.root, new_root(base.path()), "数据根不换地方");
    assert!(plan.root.is_dir(), "现在是个目录了");
    assert!(plan.root.join("library.json").is_file(), "旧数据搬进来了");
    assert!(
        plan.note.contains("没完成"),
        "要说清发生了什么事：{}",
        plan.note
    );
}

/// S4：空目录不算「半份」—— 没导入过东西的新机器不该每次启动攒一个 .incomplete。
#[test]
fn an_empty_root_is_not_half_baked() {
    let base = tempfile::tempdir().expect("临时目录");
    let installation = installation(base.path());
    let root = new_root(base.path());
    std::fs::create_dir_all(&root).expect("空的新根");

    for _ in 0..3 {
        let plan = crate::library::resolve_data_root(&installation, &exe(base.path()));
        assert_eq!(plan.root, root, "空目录就该直接用");
    }
    assert!(
        !base
            .path()
            .join("BLZ")
            .join("miyin_launcher_data.incomplete")
            .exists()
    );
    assert!(
        !base
            .path()
            .join("BLZ")
            .join("miyin_launcher_data.incomplete-1")
            .exists()
    );
}

/// 用户报的：迁移完成后**必须**把原来那份删掉，否则每次启动都会被当成「另一份数据」再合并一遍。
#[test]
fn migration_removes_the_legacy_dir_so_it_is_not_merged_twice() {
    let base = tempfile::tempdir().expect("临时目录");
    let legacy = legacy_dir(base.path());
    std::fs::create_dir_all(&legacy).expect("旧数据");
    std::fs::write(legacy.join("library.json"), b"{}").expect("旧索引");
    let installation = installation(base.path());
    let exe = exe(base.path());

    let first = crate::library::resolve_data_root(&installation, &exe);
    assert!(first.migrated, "{}", first.note);
    assert!(!legacy.exists(), "迁完旧的必须不在：{}", first.note);

    let second = crate::library::resolve_data_root(&installation, &exe);
    assert!(
        second.legacy.is_none(),
        "第二次启动不该还惦记着旧数据，否则会反复合并"
    );
    assert!(!second.migrated);
}
