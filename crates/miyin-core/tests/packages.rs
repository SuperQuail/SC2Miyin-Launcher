//! **自己造包**的端到端测试：导入 -> 启用 -> 检查游戏目录里实际长什么样。
//!
//! # 为什么不用 reference/ 里的真实战役包
//!
//! 那些是**游戏版权内容**：不进仓库，别人 clone 下来也跑不了测试。
//! 所以这里按真实包的**结构**造小包 —— 落点规则只关心路径和形态，
//! 内容用占位字节就够了。
//!
//! 每一节都注明了结构是从哪个真包学来的。
//!
//! # 为什么要有这一层
//!
//! 单元测试造的是我以为的形状，而真实包一次次教我们不是那样。目前踩过的四个：
//!
//! | 问题 | 单元测试为什么没拦住 |
//! | --- | --- |
//! | 模组丢了文件夹层级 | 我造的假包结构和真包不一样 |
//! | 自制战役「未知的战役槽位」 | 没测过 custom 这条槽位 |
//! | 地图被拍平，启动器地图联动不了 | 不知道启动器地图按什么路径找下一关 |
//! | 白名单把新落点拒了 | 白名单和落点分别测，没人测它们**合起来** |
//!
//! 这些现在都能在这里复现。

use std::io::Write;
use std::path::{Path, PathBuf};

use miyin_core::library::mods;
use miyin_core::library::{Library, activate, import};
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

    /// 导入 + 启用。
    fn install(&self, package: &Path, slot: &str) -> miyin_core::library::Variant {
        let variant = import(&self.library, package, slot, Default::default())
            .unwrap_or_else(|error| panic!("导入 {} 失败：{error}", package.display()));
        activate(&self.library, &self.installation, slot, Some(&variant.id))
            .unwrap_or_else(|error| panic!("启用 {} 失败：{error}", package.display()));
        variant
    }

    /// 游戏目录里所有文件的相对路径。
    fn deployed(&self) -> Vec<String> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&self.installation.root)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(relative) = entry.path().strip_prefix(&self.installation.root) else {
                continue;
            };
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
        out.sort();
        out
    }

    fn assert_installed(&self, relative: &str) {
        let path = self
            .installation
            .root
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert!(
            path.exists(),
            "应当装上 {relative}，实际没有。游戏目录里现在有：\n{}",
            self.deployed().join("\n")
        );
    }

    fn assert_absent(&self, relative: &str) {
        let path = self
            .installation
            .root
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        assert!(!path.exists(), "不该出现 {relative}");
    }
}

// ==================== 按游戏根目录摆的包 ====================

/// 复刻战役那种摆法（结构学自 SCMR 8.0）：
///
/// ```text
/// Mods/SCMRassets.SC2Mod          <- 已经在游戏根的位置上了
/// Mods/SCMRmod.SC2Mod
/// Starcraft Mass Recall/          <- 那这个兄弟目录就是 Maps/ 底下的
/// ├── 1. Rebel Yell/Terran01.SC2Map
/// └── SCMR Campaign Launcher.SC2Map
/// ```
///
/// 它的启动器地图里写的是 `GameSetNextMap("Starcraft Mass Recall/…")` ——
/// 相对 `Maps/`。所以装完必须长成 `Maps/Starcraft Mass Recall/…`，
/// **不能被当成分类目录拍平**。
#[test]
fn a_package_laid_out_like_the_game_root_keeps_its_folders() {
    let sandbox = Sandbox::new();
    let package = build_zip(
        sandbox._work.path(),
        "remake.zip",
        &[
            ("Mods/SCMRassets.SC2Mod", "asset"),
            ("Mods/SCMRmod.SC2Mod", "mod"),
            (
                "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
                "launcher",
            ),
            ("Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map", "map"),
            ("Starcraft Mass Recall/1. Rebel Yell/Terran02.SC2Map", "map"),
        ],
    );

    sandbox.install(&package, "wol");

    // 模组进 Mods/
    sandbox.assert_installed("Mods/SCMRassets.SC2Mod");
    sandbox.assert_installed("Mods/SCMRmod.SC2Mod");

    // 地图**保留章节层级**，整棵挂在 Maps/ 底下
    sandbox.assert_installed("Maps/Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map");
    sandbox.assert_installed("Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map");

    // **绝不能被拍平**
    sandbox.assert_absent("Maps/Campaign/Terran01.SC2Map");
    sandbox.assert_absent("Maps/Terran01.SC2Map");
}

// ==================== 包名目录下的一包多模组 ====================

/// 军械库那种摆法（结构学自「疯批帝国军械库 2.4」）：
/// 包根一层包名目录，下面 `Maps/` 是战役地图、`Mods/` 里塞了三个模组。
#[test]
fn a_package_with_several_mods_installs_all_of_them_with_their_folders() {
    let sandbox = Sandbox::new();
    let package = build_zip(
        sandbox._work.path(),
        "armory.zip",
        &[
            ("Pack/Maps/Campaign/a.SC2Map", "map"),
            ("Pack/Mods/3疯批帝国之翼.SC2Mod", "single"),
            ("Pack/Mods/Alenger/1钢铁.SC2Mod", "one"),
            ("Pack/Mods/Alenger/通用效果.SC2Mod", "two"),
            ("Pack/Mods/kit_liberty_story.SC2Mod/Assets/x.dds", "asset"),
        ],
    );

    sandbox.install(&package, "wol");

    // Alenger 是个**目录**，层级原样
    sandbox.assert_installed("Mods/Alenger/1钢铁.SC2Mod");
    sandbox.assert_installed("Mods/Alenger/通用效果.SC2Mod");
    sandbox.assert_absent("Mods/Alenger.SC2Mod");

    // 单文件模组铺成**文件**
    sandbox.assert_installed("Mods/3疯批帝国之翼.SC2Mod");

    // 解开目录树形态同样保留层级
    sandbox.assert_installed("Mods/kit_liberty_story.SC2Mod/Assets/x.dds");

    // 地图（包内是 Maps/ 镜像）进 Campaign，且没混进 Mods
    sandbox.assert_installed("Maps/Campaign/a.SC2Map");
    sandbox.assert_absent("Mods/Maps/Campaign/a.SC2Map");
}

/// 同一个包走「独立模组」入口：包里的模组各自成为库记录。
#[test]
fn the_same_package_works_as_standalone_mods() {
    let sandbox = Sandbox::new();
    let package = build_zip(
        sandbox._work.path(),
        "armory2.zip",
        &[
            ("Pack/Maps/Campaign/a.SC2Map", "map"),
            ("Pack/Mods/3疯批帝国之翼.SC2Mod", "single"),
            ("Pack/Mods/Alenger/1钢铁.SC2Mod", "one"),
            ("Pack/Mods/Alenger/通用效果.SC2Mod", "two"),
        ],
    );

    let data = sandbox.library.root();
    let result = mods::import(data, &package, None, None, Default::default()).expect("导入");
    assert_eq!(result.records.len(), 2, "Mods/ 下两个模组，一个都不该漏");

    let folders: Vec<&str> = result
        .records
        .iter()
        .map(|item| item.folder.as_str())
        .collect();
    assert!(folders.contains(&"Alenger"));
    assert!(folders.contains(&"3疯批帝国之翼.SC2Mod"));

    for record in &result.records {
        mods::set_enabled(data, &record.id, true).expect("启用");
    }
    mods::sync(data, &sandbox.installation).expect("铺盘");

    sandbox.assert_installed("Mods/Alenger/1钢铁.SC2Mod");
    sandbox.assert_installed("Mods/3疯批帝国之翼.SC2Mod");
}

// ==================== 自制战役槽位 ====================

/// 没元数据的散装战役包走 `custom` 槽位：地图进 CustomCampaigns。
#[test]
fn a_loose_campaign_package_installs_into_custom_campaigns() {
    let sandbox = Sandbox::new();
    let package = build_zip(
        sandbox._work.path(),
        "loose.zip",
        &[
            ("SomeCampaign/1. First/a.SC2Map", "map"),
            ("SomeCampaign/2. Second/b.SC2Map", "map"),
            ("SomeCampaign/launcher.SC2Map", "launcher"),
        ],
    );

    let variant = sandbox.install(&package, "custom");
    assert_eq!(variant.map_count, 3);

    let deployed = sandbox.deployed();
    assert!(
        deployed
            .iter()
            .any(|path| path.starts_with("Maps/CustomCampaigns/")),
        "自制战役该进 CustomCampaigns，实际：\n{}",
        deployed.join("\n")
    );
    assert!(
        !deployed
            .iter()
            .any(|path| path.starts_with("Maps/Campaign/")),
        "自制战役绝不能进官方 Maps/Campaign"
    );
}

// ==================== 导入之后的编辑与删除 ====================

/// 改元数据、删版本 —— 库这边的完整来回。
#[test]
fn a_variant_can_be_edited_then_removed_cleanly() {
    use miyin_core::library::{VariantChanges, remove_variant, update_variant};

    let sandbox = Sandbox::new();
    let package = build_zip(
        sandbox._work.path(),
        "edit.zip",
        &[
            (
                "C/metadata.txt",
                "title=原始名\nauthor=某人\ncampaign=WOL\nversion=1.0\n",
            ),
            ("C/maps/a.SC2Map", "map"),
        ],
    );

    let variant = sandbox.install(&package, "wol");
    sandbox.assert_installed("Maps/Campaign/a.SC2Map");

    // 改信息
    let updated = update_variant(
        &sandbox.library,
        "wol",
        &variant.id,
        VariantChanges {
            name: Some("改过的名字".to_string()),
            author: Some("某位作者".to_string()),
            registration_id: None,
            version: Some("2.0".to_string()),
            description: None,
            declared_mods: None,
        },
    )
    .expect("改信息");
    assert_eq!(updated.name, "改过的名字");
    assert_eq!(updated.version.as_deref(), Some("2.0"));

    // 删版本：它正在启用，要先撤下来
    remove_variant(&sandbox.library, &sandbox.installation, "wol", &variant.id).expect("删版本");

    sandbox.assert_absent("Maps/Campaign/a.SC2Map");
    assert!(
        sandbox
            .library
            .slots(None)
            .iter()
            .find(|slot| slot.slug == "wol")
            .is_some_and(|slot| slot.variants.is_empty()),
        "库里不该还留着这一版"
    );
}

// ==================== 切回原版 ====================

/// 启用再切回原版，游戏目录要回到原样（官方文件一个都不能少）。
#[test]
fn switching_back_restores_the_vanilla_files() {
    let sandbox = Sandbox::new();

    // 先摆一个「官方地图」，模拟游戏自带的东西
    let official = sandbox
        .installation
        .campaign_maps_root
        .join("Terran01.SC2Map");
    std::fs::create_dir_all(official.parent().expect("父目录")).expect("建目录");
    std::fs::write(&official, b"official").expect("写官方地图");

    let package = build_zip(
        sandbox._work.path(),
        "override.zip",
        &[
            (
                "C/metadata.txt",
                "title=改版\nauthor=某人\ncampaign=WOL\nversion=1.0\n",
            ),
            // 同名地图 —— 会盖掉官方那张
            ("C/maps/Terran01.SC2Map", "override"),
        ],
    );

    let variant = sandbox.install(&package, "wol");
    assert_eq!(
        std::fs::read(&official).expect("读"),
        b"override",
        "改版该盖上去"
    );

    // 切回原版
    activate(&sandbox.library, &sandbox.installation, "wol", None).expect("切回原版");

    assert_eq!(
        std::fs::read(&official).expect("读"),
        b"official",
        "**官方那张必须原样还回来** —— 这是整个方案的安全底线"
    );
    let _ = variant;
}
