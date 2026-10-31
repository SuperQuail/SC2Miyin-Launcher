//! 激活 / 存档 / 安装清单的端到端回归（编号沿用 `docs/review/mount-launch-audit.md`）。
//!
//! - **R1**：开了存档隔离之后，`activate` 切版本时 `Banks` 要跟着切组 ——
//!   不然玩 B 时写进的进度会存回 A 的组（下一次切组就被覆盖掉）。
//! - **R4**：「这一版装着没」看安装清单，不看 `slot.active` —— 装着但非 active
//!   的版本删除时，游戏目录里那一份也得撤掉，否则成了永远没人认领的幽灵。
//! - **R3**：清单里的来源存**相对库根**的路径（写的时候带 `/`），库根搬家之后
//!   `target_of` 仍能反查到游戏目录里那一份；读的路径由直接构造的清单钉住。
//!
//! 全程 `tempfile`，绝不碰真机安装目录 / 文档目录（`AGENTS.md` §6）。

use std::io::Write;
use std::path::{Path, PathBuf};

use miyin_core::library::install::{Installed, Manifest, Owner, Plan};
use miyin_core::library::{ImportMode, Library, Variant, activate, import, remove_variant};
use miyin_core::saves;
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

/// 假游戏安装 + 独立库根 + 临时文档目录。
struct Sandbox {
    _work: tempfile::TempDir,
    work: PathBuf,
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

        let mut installation =
            Installation::from_root(&game, DiscoverySource::Manual).expect("有效安装");

        // `banks_root` 是按真机「我的文档」推出来的 —— 测试绝不许碰它，
        // 换成临时目录，开存档隔离才敢真的开关（见 `activation::switch_saves`）。
        let documents = work.path().join("documents");
        installation.documents_root = Some(documents.clone());
        installation.user_maps_root = Some(documents.join("Maps"));
        installation.banks_root = Some(documents.join("Banks"));
        installation.arcade_banks_root = Some(documents.join("ArcadeBanks"));

        Self {
            library: Library::new(work.path().join("data")),
            work: work.path().to_path_buf(),
            installation,
            _work: work,
        }
    }

    /// 造一个最小战役包（元数据 + 地图），返回 zip 路径。
    fn package(&self, zip: &str, title: &str, campaign: &str, maps: &[&str]) -> PathBuf {
        let mut files: Vec<(String, String)> = vec![(
            "Reborn/metadata.txt".to_string(),
            format!("title={title}\nauthor=回归\ncampaign={campaign}\nversion=1.0\n"),
        )];
        for map in maps {
            files.push((format!("Reborn/maps/{map}"), "stub".to_string()));
        }

        let borrowed: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, content)| (path.as_str(), content.as_str()))
            .collect();
        build_zip(&self.work, zip, &borrowed)
    }

    /// 造包并导入到某个槽位。
    fn import_variant(
        &self,
        slot: &str,
        zip: &str,
        title: &str,
        campaign: &str,
        maps: &[&str],
    ) -> Variant {
        let package = self.package(zip, title, campaign, maps);
        import(&self.library, &package, slot, ImportMode::Rename).expect("导入")
    }

    fn banks(&self) -> PathBuf {
        self.installation
            .banks_root
            .clone()
            .expect("测试里给了 Banks")
    }

    fn activate(&self, slot: &str, variant_id: &str) {
        activate(&self.library, &self.installation, slot, Some(variant_id)).expect("启用");
    }
}

/// Banks 里现在有哪些存档文件。
fn bank_files(banks: &Path) -> Vec<String> {
    saves::snapshot(banks)
        .expect("快照")
        .files
        .into_iter()
        .map(|file| file.name)
        .collect()
}

/// 某个版本现在用哪个存档组目录。
fn group_dir(sandbox: &Sandbox, slot: &str, variant_id: &str) -> PathBuf {
    let key = saves::isolation_key(&sandbox.library, slot, Some(variant_id));
    let name = saves::profile_for(sandbox.library.root(), &key)
        .expect("读隔离状态")
        .unwrap_or_else(|| panic!("{key} 该有自己的一组"));
    saves::root(sandbox.library.root()).join(name)
}

/// 「原版」组目录（打开隔离时收下的那份）。
fn vanilla_group(sandbox: &Sandbox) -> PathBuf {
    let entry = saves::backups(sandbox.library.root())
        .expect("列备份")
        .into_iter()
        .find(|item| item.label == saves::ORIGINAL)
        .expect("开隔离时应当把现有存档收成原版组");
    saves::root(sandbox.library.root()).join(entry.name)
}

/// R1：隔离开着时切版本，`Banks` 要跟着切组 ——
/// 新版本从空档开始，旧版本的进度存回**它自己**的组，原版那份一个字节不动。
#[test]
fn switching_versions_switches_the_banks_group() {
    let sandbox = Sandbox::new();
    let banks = sandbox.banks();
    std::fs::create_dir_all(&banks).expect("建 Banks");
    std::fs::write(banks.join("原版.SC2Bank"), b"vanilla").expect("原版存档");

    // 用户在设置页打开存档隔离：现在这份被收成「原版」组
    saves::set_isolation(sandbox.library.root(), &banks, true).expect("打开隔离");

    let a = sandbox.import_variant("wol", "a.zip", "版本A", "WOL", &["A.SC2Map"]);
    let b = sandbox.import_variant("wol", "b.zip", "版本B", "WOL", &["B.SC2Map"]);

    sandbox.activate("wol", &a.id);
    assert!(
        bank_files(&banks).is_empty(),
        "切到 A 应当从 A 自己的空组开始，不能把原版进度留在 Banks 里"
    );

    // 玩家在 A 里打了一关
    std::fs::write(banks.join("A.SC2Bank"), b"play A").expect("写 A 的进度");

    sandbox.activate("wol", &b.id);
    assert!(
        bank_files(&banks).is_empty(),
        "切到 B 之后 Banks 里不该还留着 A 的进度"
    );
    assert!(
        group_dir(&sandbox, "wol", &a.id)
            .join("A.SC2Bank")
            .is_file(),
        "A 的进度要存回 A 自己的组"
    );
    assert!(
        vanilla_group(&sandbox).join("原版.SC2Bank").is_file(),
        "原版那份不许被动"
    );
}

/// R4：清单里装着、但索引里的 `active` 不指向它的版本，删除时也要撤掉游戏目录里的文件。
///
/// `active` 只记「最后选中的那个」：上一次激活半途失败、老数据、同一槽位装了好几部
/// （自制战役的常态）都会出现「装着但非 active」。
#[test]
fn removing_an_installed_but_unselected_version_clears_the_game_folder() {
    let sandbox = Sandbox::new();
    let a = sandbox.import_variant("custom", "a.zip", "第一部", "custom", &["a.SC2Map"]);
    let b = sandbox.import_variant("custom", "b.zip", "第二部", "custom", &["b.SC2Map"]);

    // 清单：A 装着（来源在库里，跟着版本目录走）
    let owner = Owner::Campaign {
        slot: "custom".to_string(),
        variant: a.id.clone(),
    };
    let source = sandbox
        .library
        .slot_dir("custom")
        .join(&a.id)
        .join("probe.SC2Map");
    std::fs::write(&source, b"map").expect("写源文件");
    let deployed = sandbox
        .installation
        .custom_campaigns_root
        .join(&a.id)
        .join("a.SC2Map");
    let mut manifest = Manifest::load_migrating(sandbox.library.root(), &sandbox.installation);
    let mut plan = Plan::new(owner.clone());
    plan.push(&source, format!("Maps/CustomCampaigns/{}/a.SC2Map", a.id));
    manifest
        .apply(sandbox.library.root(), &sandbox.installation, &plan)
        .expect("铺 A");
    assert!(deployed.is_file(), "前提：A 确实铺在游戏目录里");

    // 索引里的「启用」指着别人 —— 这正是旧代码会漏掉的状态
    let mut index = sandbox.library.index();
    index.slots.entry("custom".to_string()).or_default().active = Some(b.id.clone());
    sandbox.library.save_index(&index).expect("写索引");

    remove_variant(&sandbox.library, &sandbox.installation, "custom", &a.id).expect("删除 A");

    assert!(
        !deployed.exists(),
        "装着但非 active 的版本删除后，游戏目录里的文件也必须撤掉"
    );
    assert!(
        Manifest::load(sandbox.library.root()).of(&owner).is_empty(),
        "清单里不许留下没人认领的记录"
    );
    assert!(
        !sandbox.library.slot_dir("custom").join(&a.id).exists(),
        "库里的版本目录删掉"
    );
    assert!(
        sandbox.library.slot_dir("custom").join(&b.id).is_dir(),
        "同槽位别的版本一个字节都不动"
    );
}

/// R3（读）：清单里存的是相对库根的来源，库根搬家之后仍然反查得到游戏目录里那一份。
#[test]
fn a_relative_source_still_resolves_after_the_library_moves() {
    let sandbox = Sandbox::new();
    let old_root = sandbox.library.root().to_path_buf();
    let map = old_root.join("campaigns/wol/重生/a.SC2Map");
    std::fs::create_dir_all(map.parent().expect("父目录")).expect("建版本目录");
    std::fs::write(&map, b"map").expect("写地图");

    let mut manifest = Manifest::default();
    manifest.files.push(Installed {
        target: "Maps/Campaign/a.SC2Map".to_string(),
        owner: Owner::Campaign {
            slot: "wol".to_string(),
            variant: "重生".to_string(),
        },
        backup: None,
        // 相对库根，统一正斜杠
        source: Some("campaigns/wol/重生/a.SC2Map".to_string()),
    });
    manifest.save(&old_root).expect("写清单");

    // 整个库搬家（`resolve_data_root` 的数据迁移就是这个效果）
    let moved = sandbox.work.join("data-moved");
    std::fs::rename(&old_root, &moved).expect("搬库");

    let manifest = Manifest::load(&moved);
    let source = moved.join("campaigns/wol/重生/a.SC2Map");

    // 反查要求「装好的那份真的在盘上」—— 补上它，别把账实不符混进这条用例
    let installed = sandbox.installation.campaign_maps_root.join("a.SC2Map");
    std::fs::create_dir_all(&sandbox.installation.campaign_maps_root).expect("建 Campaign 目录");
    std::fs::write(&installed, b"map").expect("写");

    assert_eq!(
        manifest
            .target_of(&moved, &sandbox.installation, &source)
            .expect("该能反查到装到哪了"),
        Some(installed),
        "换库根之后仍然要能反查到游戏目录里那一份"
    );
}

/// R3（写）：`apply` 记的来源必须是相对库根的 —— 记绝对路径，搬一次家就再也对不上。
#[test]
fn activation_records_the_source_relative_to_the_library_root() {
    let sandbox = Sandbox::new();
    let variant = sandbox.import_variant("wol", "a.zip", "版本A", "WOL", &["A.SC2Map"]);
    sandbox.activate("wol", &variant.id);

    let owner = Owner::Campaign {
        slot: "wol".to_string(),
        variant: variant.id.clone(),
    };
    let recorded = Manifest::load(sandbox.library.root())
        .of(&owner)
        .into_iter()
        .find_map(|item| item.source)
        .expect("装了就该记一条来源");

    assert!(
        !Path::new(&recorded).is_absolute(),
        "清单里的来源要跟着库根走，不能是绝对路径：{recorded}"
    );
    assert!(
        recorded.starts_with(&format!("campaigns/wol/{}/", variant.id)),
        "应当是相对库根的路径：{recorded}"
    );
    assert!(!recorded.contains('\\'), "统一用 / 分隔：{recorded}");
}
/// R1（回滚）：启用半途失败回滚到原版时，**存档组也要跟着回原版**。
///
/// 旧代码只回滚游戏目录里的文件：游戏里是原版、启动器却还停在上次那部战役的组上，
/// 下一次切组就会把两份进度搅在一起（见 `activation::activate` 的回滚分支）。
#[test]
fn a_failed_activation_switches_the_save_group_back_to_vanilla() {
    let sandbox = Sandbox::new();
    let banks = sandbox.banks();
    std::fs::create_dir_all(&banks).expect("建 Banks");
    std::fs::write(banks.join("原版.SC2Bank"), b"vanilla").expect("原版存档");
    saves::set_isolation(sandbox.library.root(), &banks, true).expect("开隔离");
    let vanilla = saves::isolation(sandbox.library.root())
        .expect("读隔离状态")
        .active
        .expect("开隔离时收下的原版组");

    let a = sandbox.import_variant("wol", "a.zip", "版本A", "WOL", &["A.SC2Map"]);
    sandbox.activate("wol", &a.id);
    assert_ne!(
        saves::isolation(sandbox.library.root())
            .expect("读")
            .active
            .as_deref(),
        Some(vanilla.as_str()),
        "前提：启用 A 之后存档组跟到了 A 自己那一组"
    );

    let b = sandbox.import_variant("wol", "b.zip", "版本B", "WOL", &["B.SC2Map"]);

    // 让铺 B 失败：把 Maps/Campaign 换成一个同名文件，目标父目录就建不出来
    let campaign = sandbox.installation.campaign_maps_root.clone();
    std::fs::remove_dir_all(&campaign).expect("清掉 Campaign");
    std::fs::write(&campaign, b"x").expect("用文件占住目录位置");

    let error = activate(&sandbox.library, &sandbox.installation, "wol", Some(&b.id))
        .expect_err("铺盘失败必须报错");
    assert!(
        format!("{error}").contains("已回滚"),
        "失败要说明已回滚到原版：{error}"
    );

    // 游戏目录回了原版，存档组也必须跟着回去
    assert_eq!(
        saves::isolation(sandbox.library.root())
            .expect("读")
            .active
            .as_deref(),
        Some(vanilla.as_str()),
        "回滚后存档组要回原版组，别停在上次那部"
    );
    assert_eq!(
        std::fs::read_to_string(banks.join("原版.SC2Bank")).expect("读"),
        "vanilla",
        "回滚之后 Banks 里该是原版那份"
    );
    assert_eq!(
        saves::profile_for(sandbox.library.root(), "wol").expect("查"),
        Some(vanilla),
        "原版组要顺手绑到这个槽位的原版键上"
    );
}
