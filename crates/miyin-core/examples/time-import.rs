//! 给「导入 -> 启用」各阶段计时，找出慢在哪。
//!
//! 用法：
//!
//! `@text
//! cargo run --release -p miyin-core --example time-import -- <包路径>
//! `@

use std::path::PathBuf;
use std::time::Instant;

use miyin_core::campaign::package;
use miyin_core::library;
use miyin_core::sc2::{DiscoverySource, Installation};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(package) = args.get(1) else {
        eprintln!("用法: time-import <包路径>");
        std::process::exit(2);
    };
    let package = PathBuf::from(package);

    let work = std::env::temp_dir().join("miyin-time-import");
    let _ = std::fs::remove_dir_all(&work);

    let game = work.join("game");
    std::fs::create_dir_all(&game).expect("建游戏目录");
    std::fs::write(game.join("StarCraft II.exe"), b"stub").expect("marker");
    std::fs::write(
        game.join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");
    let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");

    let library = library::Library::new(work.join("data"));
    let size = std::fs::metadata(&package)
        .map(|meta| meta.len())
        .unwrap_or(0);
    println!("包 {} （{} MB）", package.display(), size / 1024 / 1024);
    println!();

    // 1) 预检 —— 界面点「导入」之前就要跑一次
    let start = Instant::now();
    let inspection = package::inspect(&package).expect("预检");
    let inspect_ms = start.elapsed().as_millis();
    println!(
        "1) 预检 inspect          {:>7} ms   （{} 条记录 / {} 张地图 / {} 个模组）",
        inspect_ms, inspection.entry_count, inspection.map_count, inspection.mod_count
    );

    // 2) 导入 —— 解包进库
    let start = Instant::now();
    let variant = library::import(
        &library,
        &package,
        &inspection
            .suggested_slot
            .clone()
            .unwrap_or_else(|| "wol".to_string()),
        Default::default(),
    )
    .expect("导入");
    println!(
        "2) 导入 import           {:>7} ms",
        start.elapsed().as_millis()
    );

    // 3) 启用 —— 铺进游戏目录
    let start = Instant::now();
    library::activate(&library, &installation, "wol", Some(&variant.id)).expect("启用");
    println!(
        "3) 启用 activate         {:>7} ms",
        start.elapsed().as_millis()
    );

    println!();

    println!("现场：{}", work.display());
}
