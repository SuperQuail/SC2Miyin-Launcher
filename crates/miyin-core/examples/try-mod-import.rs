//! 拿真实的模组包跑一遍导入 + 铺盘，把结果目录打出来。
//!
//! 用法：
//!
//! `@text
//! cargo run -p miyin-core --example try-mod-import -- <包路径>
//! `@
//!
//! 为什么要这个：模组导入的正确性最终体现在「游戏目录里长什么样」，
//! 单元测试覆盖的是我**以为**的形状；拿真包跑一遍才能看出真实形状差在哪。

use std::path::{Path, PathBuf};

use miyin_core::library::mods;
use miyin_core::sc2::{DiscoverySource, Installation};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(package) = args.get(1) else {
        eprintln!("用法: try-mod-import <包路径>");
        std::process::exit(1);
    };
    let package = PathBuf::from(package);

    let work = std::env::temp_dir().join("miyin-try-mod");
    let _ = std::fs::remove_dir_all(&work);

    // ---- 假的游戏安装 ----
    let game = work.join("game");
    std::fs::create_dir_all(&game).expect("建游戏目录");
    std::fs::write(game.join("StarCraft II.exe"), b"stub").expect("marker");
    std::fs::write(
        game.join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");
    let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");

    // ---- 库 ----
    let data = work.join("data");
    std::fs::create_dir_all(&data).expect("建库目录");

    println!("包: {}", package.display());
    println!();

    // ---- 导入 ----
    let result = match mods::import(&data, &package, None, None, Default::default()) {
        Ok(result) => result,
        Err(error) => {
            println!("导入失败: {error}");
            std::process::exit(1);
        }
    };

    println!("=== 导入结果：{} 个模组 ===", result.records.len());
    for (index, record) in result.records.iter().enumerate() {
        println!(
            "  {}. 显示名={:<28} 挂载名={:<28} 形态={:?} 文件数={} 版本={}",
            index + 1,
            record.name,
            record.folder,
            record.kind,
            record.parts,
            record.version.clone().unwrap_or_default()
        );
    }
    println!();

    // ---- 启用全部，铺进「游戏目录」----
    for record in &result.records {
        mods::set_enabled(&data, &record.id, true).expect("启用");
    }
    let placed = mods::sync(&data, &installation).expect("铺盘");
    println!("=== 铺进 <游戏>/Mods/ 的东西 ===");
    for name in &placed.placed {
        println!("  {name}");
    }
    println!();

    for warning in &placed.warnings {
        println!("  警告: {warning}");
    }
    println!();
    println!("=== <游戏>/Mods/ 的实际目录树（前 3 层）===");
    print_tree(&installation.mods_root, 3);

    println!();
    println!("=== 库里的样子 ===");
    for record in &result.records {
        let dir = data.join("mods").join(&record.id);
        println!("  data/mods/{}/", record.id);
        print_tree(&dir, 3);
    }

    println!();
    println!("完整路径（要看细节自己进去翻）: {}", work.display());
    println!();

    // ================= 战役那条路 =================
    //
    // 同一个包，用户更可能是当**战役**导入的。那条路走 compose.rs，
    // 载荷落点由 payload_target() 决定 —— 和独立模组那条路完全独立，
    // 得单独验。
    println!("======================================================");
    println!("第二条路：当**战役**导入");
    println!("======================================================");

    let library = miyin_core::library::Library::new(work.join("campaign-data"));
    let imported = match miyin_core::library::import(&library, &package, "wol", Default::default())
    {
        Ok(variant) => variant,
        Err(error) => {
            println!("战役导入失败: {error}");
            return;
        }
    };

    println!(
        "版本: {}（{} 张地图 / {} 个模组）",
        imported.name, imported.map_count, imported.mod_count
    );
    println!();
    println!("=== 载荷落点（这就是会铺进游戏目录的清单）===");
    for payload in &imported.payloads {
        let dest = miyin_core::library::compose::payload_target_path(
            &payload.target,
            &miyin_core::library::compose::Placement::Campaign { sub: None },
        );
        println!("  {:<56} -> {}", payload.source, dest.unwrap_or_default());
    }

    println!();
    println!("=== 铺盘后 <游戏>/Mods/ 的样子 ===");
    let game2 = work.join("game2");
    std::fs::create_dir_all(&game2).expect("建游戏目录 2");
    std::fs::write(game2.join("StarCraft II.exe"), b"stub").expect("marker");
    std::fs::write(
        game2.join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");
    let installation2 = Installation::from_root(&game2, DiscoverySource::Manual).expect("安装 2");

    match miyin_core::library::activate(&library, &installation2, "wol", Some(&imported.id)) {
        Ok(warnings) => {
            for warning in &warnings {
                println!("  警告: {warning}");
            }
            print_tree(&installation2.mods_root, 3);
        }
        Err(error) => println!("  铺盘失败: {error}"),
    }
}

/// 打一棵目录树，最多 max_depth 层。
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
        let depth = entry.depth();
        let name = entry.file_name().to_string_lossy();
        let marker = if entry.file_type().is_dir() { "/" } else { "" };
        println!("    {}{}{}", "  ".repeat(depth), name, marker);
    }
}
