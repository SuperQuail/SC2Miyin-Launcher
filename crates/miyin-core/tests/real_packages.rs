//! **真实战役包**的端到端测试：导入 -> 启用 -> 检查游戏目录里实际长什么样。
//!
//! # 为什么单独一层
//!
//! 单元测试造的是**我以为的形状**。而真实包一次次教我们不是那样 ——
//! 四个严重问题全是拿真包跑出来的，没有一个被单元测试拦住：
//!
//! | 问题 | 单元测试为什么没拦住 |
//! | --- | --- |
//! | 模组丢了文件夹层级 | 我造的假包结构跟真包不一样 |
//! | 自制战役「未知的战役槽位」 | 没测过 custom 这条槽位 |
//! | 地图被拍平，启动器地图联动不了 | 不知道启动器地图按什么路径找下一关 |
//! | 白名单把新落点拒了 | 白名单和落点分别测，没人测它们**合起来** |
//!
//! # 怎么跑
//!
//! ```text
//! cargo test -p miyin-core --test real_packages                 # 快的
//! cargo test -p miyin-core --test real_packages -- --ignored    # 连上 G 的大包
//! ```
//!
//! 没有 `reference/` 目录（CI、别人的机器）就**跳过**，不算失败 ——
//! 那些包是游戏版权内容，不进仓库。

use std::path::{Path, PathBuf};

use miyin_core::library::mods;
use miyin_core::library::{Library, activate, import};
use miyin_core::sc2::{DiscoverySource, Installation};

/// 参考包的目录。
fn reference_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/战役与补丁")
}

/// 找一个参考包；没有就返回 None（调用方跳过）。
fn package(name: &str) -> Option<PathBuf> {
    let path = reference_dir().join(name);
    path.is_file().then_some(path)
}

/// 一套可以随便折腾的环境：假的游戏安装 + 空的库。
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

    /// 导入 + 启用，返回版本。
    fn install(&self, package: &Path, slot: &str) -> miyin_core::library::Variant {
        let variant = import(&self.library, package, slot, Default::default())
            .unwrap_or_else(|error| panic!("导入 {} 失败：{error}", package.display()));
        activate(&self.library, &self.installation, slot, Some(&variant.id))
            .unwrap_or_else(|error| panic!("启用 {} 失败：{error}", package.display()));
        variant
    }

    /// 游戏目录里所有文件的相对路径（/ 分隔，排好序）。
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
}

/// 断言某个路径装上了。
fn assert_installed(sandbox: &Sandbox, relative: &str) {
    let path = sandbox
        .installation
        .root
        .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    assert!(
        path.exists(),
        "应当装上 {relative}，实际没有。游戏目录里现在有：\n{}",
        sandbox
            .deployed()
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ==================== 复刻战役（大包，默认跳过）====================

/// SCMR 8.0（1.4 GB）。
///
/// 这个包一次性教了我们三件事，所以断言写得比较死：
///
/// 1. 地图按 Maps/Starcraft Mass Recall/<章节>/<关卡>.SC2Map 落 —— 它的启动器
///    地图里写的是 GameSetNextMap("Starcraft Mass Recall/…")，相对 Maps/
/// 2. 四个模组落在 Mods/
/// 3. 包里的 Mods/ 是「按游戏根目录摆的」信号，不能把地图当分类目录拍平
#[test]
#[ignore = "要解开 1.4 GB 的包，跑起来慢；用 --ignored 显式跑"]
fn scmr_keeps_its_chapter_folders() {
    let Some(package) = package("SCMR8.0(未汉化原版).rar") else {
        eprintln!("跳过：没有 reference/战役与补丁/SCMR8.0(未汉化原版).rar");
        return;
    };

    let sandbox = Sandbox::new();
    let variant = sandbox.install(&package, "wol");

    assert_eq!(variant.map_count, 138, "138 张地图");
    assert!(variant.mod_count >= 4, "至少 4 个模组");

    // 启动器地图与第一章的第一关
    assert_installed(
        &sandbox,
        "Maps/Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
    );
    assert_installed(
        &sandbox,
        "Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
    );

    // **绝不能**被拍平
    assert!(
        !sandbox
            .installation
            .maps_root
            .join("Terran01.SC2Map")
            .exists(),
        "地图被拍平了 —— 启动器地图按相对 Maps/ 的路径找下一关，拍平就联动不了"
    );

    // 模组落在 Mods/
    assert_installed(&sandbox, "Mods/SCMRmod.SC2Mod");
    assert_installed(&sandbox, "Mods/SCMRassets.SC2Mod");
}

// ==================== 一包多模组 ====================

/// 疯批帝国军械库（250 MB）：包名目录下 Mods/ 里有三个模组。
#[test]
fn armory_package_installs_all_three_mods_with_their_folders() {
    let Some(package) = package("疯批帝国军械库2.4.zip") else {
        eprintln!("跳过：没有这个参考包");
        return;
    };

    let sandbox = Sandbox::new();
    let variant = sandbox.install(&package, "wol");
    assert!(variant.map_count > 0, "有地图");

    // Alenger 是个**目录**（不是 Alenger.SC2Mod），层级原样
    assert_installed(&sandbox, "Mods/Alenger/1钢铁.SC2Mod");
    assert_installed(&sandbox, "Mods/Alenger/通用效果.SC2Mod");
    assert!(
        !sandbox
            .installation
            .mods_root
            .join("Alenger.SC2Mod")
            .exists(),
        "绝不能自作主张加 .SC2Mod 后缀 —— 地图里写的是 Mods\\Alenger\\…"
    );

    // 单文件模组要铺成**文件**
    assert_installed(&sandbox, "Mods/3疯批帝国之翼.SC2Mod");
    assert!(
        sandbox
            .installation
            .mods_root
            .join("3疯批帝国之翼.SC2Mod")
            .is_file()
    );

    // 解开的目录树形态同样保留层级
    assert_installed(
        &sandbox,
        "Mods/kit_liberty_story.SC2Mod/ComponentList.SC2Components",
    );

    // 战役地图（包内是 Maps/ 镜像）不该混进 Mods/
    assert!(
        !sandbox.installation.mods_root.join("Maps").exists(),
        "地图混进 Mods 了"
    );
}

// ==================== 独立模组那条路 ====================

/// 同一个包走「独立模组」入口：一个包里的模组各自成为库记录。
#[test]
fn the_same_package_works_as_standalone_mods() {
    let Some(package) = package("疯批帝国军械库2.4.zip") else {
        eprintln!("跳过：没有这个参考包");
        return;
    };

    let sandbox = Sandbox::new();
    let data = sandbox.library.root();

    let result = mods::import(data, &package, None, None, Default::default()).expect("导入");
    assert_eq!(result.records.len(), 3, "Mods/ 下三个模组，一个都不该漏");

    let folders: Vec<&str> = result
        .records
        .iter()
        .map(|item| item.folder.as_str())
        .collect();
    assert!(folders.contains(&"Alenger"));
    assert!(folders.contains(&"3疯批帝国之翼.SC2Mod"));
    assert!(folders.contains(&"kit_liberty_story.SC2Mod"));

    for record in &result.records {
        mods::set_enabled(data, &record.id, true).expect("启用");
    }
    mods::sync(data, &sandbox.installation).expect("铺盘");

    assert_installed(&sandbox, "Mods/Alenger/1钢铁.SC2Mod");
    assert_installed(&sandbox, "Mods/3疯批帝国之翼.SC2Mod");
}

// ==================== 自制战役槽位 ====================

/// 一个真正的自制战役包走 custom 槽位：地图进 CustomCampaigns，不进官方目录。
#[test]
fn a_loose_campaign_package_installs_into_custom_campaigns() {
    let Some(package) = package("净化者纪元幼儿园（提示必须看）.zip") else {
        eprintln!("跳过：没有这个参考包");
        return;
    };

    let sandbox = Sandbox::new();
    let variant = sandbox.install(&package, "custom");

    assert!(variant.map_count > 0, "有地图");

    // 地图落在 CustomCampaigns 下面，不在官方 Campaign 里
    let deployed = sandbox.deployed();
    assert!(
        deployed
            .iter()
            .any(|path| path.starts_with("Maps/CustomCampaigns/")),
        "自制战役该进 CustomCampaigns，实际：\n{}",
        deployed
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        !deployed
            .iter()
            .any(|path| path.starts_with("Maps/Campaign/")),
        "自制战役绝不能进官方 Maps/Campaign"
    );
}
