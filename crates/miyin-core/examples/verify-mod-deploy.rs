//! 拿真实的模组包跑一遍「导入 -> 启用 -> 铺进游戏目录」，逐条核对目录对不对。
//!
//! 用法：
//!
//! `@text
//! cargo run -p miyin-core --example verify-mod-deploy -- <包路径>
//! `@
//!
//! 为什么要这个：模组导入的正确性最终体现在**游戏目录里长什么样**。
//! 单元测试验的是我「以为」的形状，拿真包铺一遍才看得出真实形状差在哪 ——
//! 「模组丢了文件夹层级」那个 bug 就是这么漏过去的。

use std::path::{Path, PathBuf};

use miyin_core::library::mods;
use miyin_core::sc2::{DiscoverySource, Installation};

/// 一条检查。
struct Check {
    what: String,
    ok: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(package) = args.get(1) else {
        eprintln!("用法: verify-mod-deploy <包路径>");
        std::process::exit(2);
    };
    let package = PathBuf::from(package);

    let work = std::env::temp_dir().join("miyin-verify-deploy");
    let _ = std::fs::remove_dir_all(&work);

    // 一个干净的「游戏目录」—— 不碰用户真实安装
    let game = work.join("game");
    std::fs::create_dir_all(&game).expect("建游戏目录");
    std::fs::write(game.join("StarCraft II.exe"), b"stub").expect("marker");
    std::fs::write(
        game.join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");
    let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");

    let data = work.join("data");
    std::fs::create_dir_all(&data).expect("建库目录");

    let mut checks: Vec<Check> = Vec::new();
    let mut check = |what: &str, ok: bool| {
        checks.push(Check {
            what: what.to_string(),
            ok,
        });
    };

    println!("包：{}", package.display());
    println!();

    // ================= 一、当独立模组导入 =================
    println!("── 一、独立模组路径 ──────────────────────────");

    let standalone = match mods::import(&data, &package, None, None, Default::default()) {
        Ok(result) => result,
        Err(error) => {
            println!("  导入失败：{error}");
            std::process::exit(1);
        }
    };

    println!("  导入 {} 个模组", standalone.records.len());
    for record in &standalone.records {
        println!(
            "    {:<26} 挂载名 {:<26} {:?}",
            record.name, record.folder, record.kind
        );
    }
    println!();

    for record in &standalone.records {
        mods::set_enabled(&data, &record.id, true).expect("启用");
    }
    mods::sync(&data, &installation).expect("铺盘");

    let mods_root = installation.mods_root.clone();
    println!("  铺盘后 <游戏>/Mods/：");
    print_tree(&mods_root, 2);

    // 真实样本的硬性期望
    check(
        "Mods/Alenger/ 是个目录（不是 Alenger.SC2Mod）",
        mods_root.join("Alenger").is_dir(),
    );
    check(
        "Mods/Alenger/1钢铁.SC2Mod 存在（层级没丢）",
        mods_root.join("Alenger").join("1钢铁.SC2Mod").is_file(),
    );
    check(
        "没有凭空多出 Alenger.SC2Mod",
        !mods_root.join("Alenger.SC2Mod").exists(),
    );
    check(
        "Mods/3疯批帝国之翼.SC2Mod 是**文件**",
        mods_root.join("3疯批帝国之翼.SC2Mod").is_file(),
    );
    check(
        "Mods/kit_liberty_story.SC2Mod/Assets 层级在",
        mods_root
            .join("kit_liberty_story.SC2Mod")
            .join("Assets")
            .is_dir(),
    );
    check(
        "战役地图没混进 Mods/",
        !mods_root.join("Maps").exists(),
    );

    println!();

    // ================= 二、当战役导入 =================
    println!("── 二、战役路径（用户更可能这么导）──────────");

    let library = miyin_core::library::Library::new(work.join("campaign-data"));
    let variant = match miyin_core::library::import(&library, &package, "wol", Default::default())
    {
        Ok(variant) => variant,
        Err(error) => {
            println!("  导入失败：{error}");
            std::process::exit(1);
        }
    };

    println!(
        "  版本 {}（{} 张地图 / {} 个模组）",
        variant.name, variant.map_count, variant.mod_count
    );

    check("地图数 > 0（解开的目录树也要算）", variant.map_count > 0);
    check("模组数按文件夹去重（不是按文件）", variant.mod_count > 0 && variant.mod_count < 10);

    // 模组落点必须带层级
    let compose_campaign = miyin_core::library::compose::Placement::Campaign { sub: None };
    let mut mod_targets: Vec<String> = Vec::new();
    for payload in &variant.payloads {
        if !payload.is_mod {
            continue;
        }
        if let Some(target) = miyin_core::library::compose::payload_target_path(
            &payload.target,
            &compose_campaign,
        ) {
            mod_targets.push(target);
        }
    }
    mod_targets.sort();
    mod_targets.dedup();

    println!("  模组落点（去重后 {} 条）：", mod_targets.len());
    for target in mod_targets.iter().take(6) {
        println!("    {target}");
    }
    if mod_targets.len() > 6 {
        println!("    …（还有 {} 条）", mod_targets.len() - 6);
    }

    check(
        "模组落点都保留了 Alenger/ 这一层",
        mod_targets
            .iter()
            .filter(|target| target.contains("Alenger"))
            .all(|target| target.starts_with("Mods/Alenger/")),
    );
    // Alenger 里的那些文件**绝不能**跑到 Mods/ 根下 ——
    // 原来那个 bug 就是它们被拍平成 Mods/1钢铁.SC2Mod 了。
    //
    // 注意别写成「Mods/ 下只许一段路径」：单文件模组
    // （Mods/3疯批帝国之翼.SC2Mod）本来就是一段，那是合法的。
    let leaked = [
        "Mods/1钢铁.SC2Mod",
        "Mods/10埃蒙.SC2Mod",
        "Mods/通用效果.SC2Mod",
        "Mods/AlengerBGM.SC2Mod",
    ];
    check(
        "Alenger 里的文件没漏到 Mods/ 根下",
        !mod_targets.iter().any(|target| leaked.contains(&target.as_str())),
    );

    // 真铺一遍
    let game2 = work.join("game2");
    std::fs::create_dir_all(&game2).expect("建游戏目录 2");
    std::fs::write(game2.join("StarCraft II.exe"), b"stub").expect("marker");
    std::fs::write(
        game2.join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");
    let installation2 = Installation::from_root(&game2, DiscoverySource::Manual).expect("安装 2");

    miyin_core::library::activate(&library, &installation2, "wol", Some(&variant.id))
        .expect("铺盘");

    println!();
    println!("  铺盘后 <游戏>/Mods/（截前 2 层）：");
    print_tree(&installation2.mods_root, 2);

    check(
        "战役路径也保住了 Alenger/ 层级",
        installation2
            .mods_root
            .join("Alenger")
            .join("1钢铁.SC2Mod")
            .is_file(),
    );
    check(
        "战役路径也没多出 Alenger.SC2Mod",
        !installation2.mods_root.join("Alenger.SC2Mod").exists(),
    );
    check(
        "地图铺到了 Maps/Campaign/",
        game2
            .join("Maps")
            .join("Campaign")
            .join("thanson01.SC2Map")
            .is_dir()
            || game2
                .join("Maps")
                .join("Campaign")
                .join("thanson01.SC2Map")
                .is_file(),
    );

    // ================= 汇总 =================
    println!();
    println!("══ 检查结果 ══════════════════════════════════");
    let mut failed = 0;
    for item in &checks {
        let mark = if item.ok {
            "  ✓"
        } else {
            failed += 1;
            "  ✗"
        };
        println!("{mark} {}", item.what);
    }
    println!();
    if failed == 0 {
        println!("全部通过（{} 项）", checks.len());
    } else {
        println!("{} / {} 项没通过", failed, checks.len());
        std::process::exit(1);
    }

    println!();
    println!("现场保留在：{}", work.display());
}

/// 打一棵目录树。
fn print_tree(root: &Path, max_depth: usize) {
    if !root.exists() {
        println!("    （不存在）");
        return;
    }
    for entry in walkdir::WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.path() == root {
            continue;
        }
        let marker = if entry.file_type().is_dir() { "/" } else { "" };
        println!(
            "    {}{}{}",
            "  ".repeat(entry.depth()),
            entry.file_name().to_string_lossy(),
            marker
        );
    }
}
