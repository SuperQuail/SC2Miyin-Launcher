//! **目标语义探针**：配置了挂载 = 已挂载。
//!
//! 用户报障：在自制战役页面给某个版本打了勾，启动仍被拦住并提示
//! 「这个战役带了模组，但你一个都没挂载 —— 这样打开地图会报错」。
//!
//! 根因不在勾选没写进索引（它写进去了），而在读的那一侧：
//! `Library::variant_mods()` / `all_mods()` 拿 `slot.active` 当门控
//! （`crates/miyin-core/src/library/mod.rs:769-780`、`:798-805`）——
//! 只有**当前启用的那一个**版本才 mounted=true，其余版本一律 false。
//! 而启动拦截读的正是这份 mounted（`src-tauri/src/lib.rs:2494-2500` 与 `:921-928`）。
//!
//! 于是两条路都会被误拦：
//! (a) 版本还没启用 —— 启动本来就会把它 `activate`，但门控在激活**之前**跑；
//! (b) 同槽位启用了另一个版本 —— 自制战役可以同时装好几部（`activation.rs:108-125`），
//!     可 `slot.active` 只有一个值。
//!
//! 下面按**目标语义**断言（配置了就该显示挂载）：对修好门控的版本是绿的，
//! 对 HEAD（门控还在的那版）是红的。**不要把它改成断言现状。**
//! 上面引的 `mod.rs` / `lib.rs` 行号都按 **HEAD** 计（那是 bug 所在的位置）。
//!
//! 修法把两个维度拆开（`mod.rs:244-253` 的 `ModEntry`）：
//! - `mounted` = **用户配置**（勾选框状态），不再按 `slot.active` 门控；
//! - `in_game` = **实际铺在 `<游戏>/Mods/` 里**（按 `installed.json` 算）。
//!
//! 顺带覆盖 `activation::sync_mods`（`activation.rs:127-181`）：启动时把丢掉的模组补回来。
//!
//! 注意：`crates/miyin-core/src/library/tests.rs` 原来那条
//! `inactive_variant_reports_no_mounted_mods` 断言的是相反的语义（那是为另一个
//! 反馈「模组互相污染」打的补丁），改语义时必须一并处理，否则两边对打。
//!
//! 搭建手法抄自 `crates/miyin-core/tests/packages.rs`：真包是版权内容不能进仓库，
//! 这里按真实包的结构造小包，内容用占位字节。

use std::io::Write;
use std::path::{Path, PathBuf};

use miyin_core::library::{ImportMode, Library, ModEntry, Variant, activate, import};
use miyin_core::sc2::{DiscoverySource, Installation};

/// 造一个 zip 包，返回路径。
fn build_zip(dir: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("建包");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    for (entry, content) in files {
        writer.start_file(*entry, options).expect("写条目");
        writer.write_all(content.as_bytes()).expect("写内容");
    }
    writer.finish().expect("收尾");
    path
}

/// 假的游戏安装 + 空的库。
struct Sandbox {
    _work: tempfile::TempDir,
    installation: Installation,
    library: Library,
}

impl Sandbox {
    fn new() -> Self {
        let work = tempfile::tempdir().expect("临时目录");
        let game = work.path().join("game");
        std::fs::create_dir_all(&game).expect("建游戏目录");
        std::fs::write(game.join("StarCraft II.exe"), b"stub").expect("marker");
        std::fs::write(
            game.join(".build.info"),
            "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
        )
        .expect("build info");

        let installation =
            Installation::from_root(&game, DiscoverySource::Manual).expect("有效安装");
        let library = Library::new(work.path().join("data"));

        Self {
            _work: work,
            installation,
            library,
        }
    }

    /// 造一个带模组的自制战役包并导入 `custom` 槽位（**只导入，不启用**）。
    ///
    /// 结构就是真实包的样子：`metadata.txt` + `maps/` + `Mods/<模组>/`。
    /// 一定要带一张地图 —— 只有模组的包会被认成补丁（`PATCH_INFERRED`）。
    fn custom_package(&self, zip_name: &str, title: &str, mod_folder: &str) -> Variant {
        let metadata = format!("title={title}\nauthor=探针\ncampaign=custom\nversion=1.0\n");
        let mod_entry = format!("Demo/Mods/{mod_folder}/Base.SC2Data/x");
        let files: [(&str, &str); 3] = [
            ("Demo/metadata.txt", metadata.as_str()),
            ("Demo/maps/a.SC2Map", "map"),
            (mod_entry.as_str(), "mod"),
        ];

        let path = build_zip(self._work.path(), zip_name, &files);
        let variant = import(&self.library, &path, "custom", ImportMode::Rename)
            .unwrap_or_else(|error| panic!("导入 {zip_name} 失败：{error}"));
        assert!(variant.mod_count > 0, "这个包里确实带了模组，界面才有得勾");
        variant
    }

    fn rows(&self, variant_id: &str) -> Vec<ModEntry> {
        self.library.variant_mods("custom", variant_id)
    }

    /// 游戏目录里的某个相对路径。
    fn game_path(&self, relative: &str) -> PathBuf {
        self.installation
            .root
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR))
    }

    fn mount(&self, variant_id: &str, mods: &[&str]) {
        let list: Vec<String> = mods.iter().map(|item| item.to_string()).collect();
        self.library
            .set_mounted_mods("custom", variant_id, &list)
            .unwrap_or_else(|error| panic!("写挂载清单失败：{error}"));
    }

    /// 复刻启动拦截的表达式（`src-tauri/src/lib.rs:2494-2500`、`:921-928`）。
    fn launch_gate_blocks(rows: &[ModEntry]) -> bool {
        !rows.is_empty() && !rows.iter().any(|row| row.mounted)
    }
}

/// 勾上之后立刻算挂载 —— 不必先把版本启用。
///
/// 对应报障现场：勾上 → 点「启动」→ 后端 `prepared_map` 拦下来。
#[test]
fn a_configured_mount_list_counts_as_mounted_before_activation() {
    let sandbox = Sandbox::new();
    let variant = sandbox.custom_package("one.zip", "带模组的自制战役", "Kit.SC2Mod");

    // 界面「挂载模组」里打勾走的就是这条命令
    sandbox.mount(&variant.id, &["Kit.SC2Mod"]);

    // 勾选**确实写进索引了** —— 这条现在是绿的，说明不是写丢失
    let saved = sandbox
        .library
        .variant("custom", &variant.id)
        .expect("读回版本");
    assert_eq!(saved.mounted_mods, Some(vec!["Kit.SC2Mod".to_string()]));

    // 目标语义：配了就算挂上，哪怕这个版本还没被启用
    let rows = sandbox.rows(&variant.id);
    assert!(!rows.is_empty(), "包里带了模组");
    assert!(
        rows.iter().all(|row| row.mounted),
        "勾上的模组必须显示成已挂载，实际：{rows:?}"
    );
    assert!(
        !Sandbox::launch_gate_blocks(&rows),
        "启动/编辑器拦截会在这里报「这个战役带了模组，但你一个都没挂载」"
    );
}

/// 刚导入、还没配过清单的版本（`mounted_mods: None` = 全挂）同样算挂载。
///
/// 这条挡住另一种误拦：用户什么都没改，只是导入了包就点启动。
#[test]
fn an_unconfigured_variant_reports_all_mods_as_mounted() {
    let sandbox = Sandbox::new();
    let variant = sandbox.custom_package("fresh.zip", "刚导入的自制战役", "Kit.SC2Mod");

    let saved = sandbox
        .library
        .variant("custom", &variant.id)
        .expect("读回版本");
    assert_eq!(saved.mounted_mods, None, "导入时不写清单 = 没配过 = 全挂");

    let rows = sandbox.rows(&variant.id);
    assert!(!rows.is_empty());
    assert!(
        rows.iter().all(|row| row.mounted),
        "没配过清单按「全挂」算，实际：{rows:?}"
    );
}

/// 同槽位启用了另一个版本，不许把这一部的挂载状态抹掉。
///
/// 自制战役可以同时装好几部（`activation.rs:108-125`），但 `slot.active` 只有一个值。
#[test]
fn activating_another_variant_keeps_this_variants_mount_state() {
    let sandbox = Sandbox::new();
    let first = sandbox.custom_package("first.zip", "第一部", "Kit.SC2Mod");
    let second = sandbox.custom_package("second.zip", "第二部", "Other.SC2Mod");

    // 只启用第一部 —— `slot.active` 变成 first
    activate(
        &sandbox.library,
        &sandbox.installation,
        "custom",
        Some(&first.id),
    )
    .expect("启用第一部");

    // 用户也给第二部勾了模组
    sandbox.mount(&second.id, &["Other.SC2Mod"]);

    let second_rows = sandbox.rows(&second.id);
    assert!(!second_rows.is_empty());
    assert!(
        second_rows.iter().all(|row| row.mounted),
        "同槽位启用了别的版本，不该把这一部判成没挂：{second_rows:?}"
    );
    assert!(!Sandbox::launch_gate_blocks(&second_rows));

    // 第一部自己照旧
    let first_rows = sandbox.rows(&first.id);
    assert!(
        first_rows.iter().all(|row| row.mounted),
        "启用中的第一部也不能因为第二部而变：{first_rows:?}"
    );
}

/// 模组管理菜单（`all_mods`）看到的是同一份配置状态。
#[test]
fn all_mods_reports_the_configured_state_of_every_variant() {
    let sandbox = Sandbox::new();
    let first = sandbox.custom_package("first.zip", "第一部", "Kit.SC2Mod");
    let second = sandbox.custom_package("second.zip", "第二部", "Other.SC2Mod");

    activate(
        &sandbox.library,
        &sandbox.installation,
        "custom",
        Some(&first.id),
    )
    .expect("启用第一部");
    sandbox.mount(&second.id, &["Other.SC2Mod"]);

    let rows = sandbox.library.all_mods();
    let row_of = |variant_id: &str| {
        rows.iter()
            .find(|row| row.variant_id == variant_id)
            .unwrap_or_else(|| panic!("all_mods 里应当有版本 {variant_id}"))
    };

    assert!(row_of(&first.id).mounted, "启用中的那部是已启用");
    assert!(
        row_of(&second.id).mounted,
        "用户勾过的模组在模组管理菜单里也必须是「已启用」，否则勾了又自己弹回去"
    );
}

/// `Some([])` = 用户**明确**一个都不挂，这条不能被「配置即挂载」顺手改掉。
///
/// 见 `crates/miyin-core/src/library/mod.rs:129-141`：`None` 与 `Some([])` 是两件事。
#[test]
fn an_explicitly_empty_mount_list_still_reports_nothing_mounted() {
    let sandbox = Sandbox::new();
    let variant = sandbox.custom_package("off.zip", "全关的自制战役", "Kit.SC2Mod");

    sandbox.mount(&variant.id, &[]);

    let rows = sandbox.rows(&variant.id);
    assert!(!rows.is_empty());
    assert!(
        rows.iter().all(|row| !row.mounted),
        "明确一个都不挂，就该显示成没挂：{rows:?}"
    );
}

/// `mounted`（配置）与 `in_game`（真的铺了）是两个维度，界面不能拿一个顶两个。
///
/// 未启用时：配了但没铺；启用后：两个都成立。
#[test]
fn mounted_is_configuration_and_in_game_is_whats_deployed() {
    let sandbox = Sandbox::new();
    let variant = sandbox.custom_package("two-dim.zip", "两个维度", "Kit.SC2Mod");
    sandbox.mount(&variant.id, &["Kit.SC2Mod"]);

    let before = sandbox.rows(&variant.id);
    assert!(
        before.iter().all(|row| row.mounted),
        "配置里挂着：{before:?}"
    );
    assert!(
        before.iter().all(|row| !row.in_game),
        "还没启用，文件没铺进游戏目录：{before:?}"
    );

    activate(
        &sandbox.library,
        &sandbox.installation,
        "custom",
        Some(&variant.id),
    )
    .expect("启用");

    let after = sandbox.rows(&variant.id);
    assert!(after.iter().all(|row| row.mounted), "配置没变：{after:?}");
    assert!(
        after.iter().all(|row| row.in_game),
        "启用之后模组应该真的铺进 <游戏>/Mods/：{after:?}"
    );
    assert!(sandbox.game_path("Mods/Kit.SC2Mod/Base.SC2Data/x").exists());
}

/// 启动时的自动挂载（`activation::sync_mods`）：模组被删掉了，启动能自己补回来。
///
/// 场景：重装过游戏、用户手动清了 `Mods/`、上次铺到一半断了。
#[test]
fn sync_mods_redeploys_a_mod_that_vanished_from_the_game_folder() {
    let sandbox = Sandbox::new();
    let variant = sandbox.custom_package("sync.zip", "自动挂载", "Kit.SC2Mod");
    sandbox.mount(&variant.id, &["Kit.SC2Mod"]);
    activate(
        &sandbox.library,
        &sandbox.installation,
        "custom",
        Some(&variant.id),
    )
    .expect("启用");

    let deployed = sandbox.game_path("Mods/Kit.SC2Mod");
    assert!(deployed.exists(), "启用时就该铺好");
    std::fs::remove_dir_all(&deployed).expect("模拟用户把 Mods 里的模组删了");

    let warnings =
        miyin_core::library::activation::sync_mods(&sandbox.library, &sandbox.installation)
            .expect("自动挂载");
    assert!(warnings.is_empty(), "没有冲突时不该有提示：{warnings:?}");
    assert!(
        sandbox.game_path("Mods/Kit.SC2Mod/Base.SC2Data/x").exists(),
        "启动时该把丢掉的模组补回来"
    );
}
