//! 对一个或多个战役 / 补丁包跑预检，并打印结果。
//!
//! 调试用工具，例如：
//!
//! ```bash
//! cargo run -p miyin-core --example inspect -- "D:\\some\\pack.zip"
//! ```

use std::path::PathBuf;

use miyin_core::campaign::package::{self, PayloadTarget};

fn main() {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if paths.is_empty() {
        eprintln!("用法: cargo run -p miyin-core --example inspect -- <包路径>...");
        std::process::exit(2);
    }

    for path in paths {
        println!("============================================================");
        println!("包: {}", path.display());

        match package::inspect(&path) {
            Ok(inspection) => {
                println!("  类型    : {:?}", inspection.kind);
                println!("  格式    : {:?}", inspection.format);
                println!("  名称    : {:?}", inspection.name);
                println!("  作者    : {:?}", inspection.author);
                println!("  版本    : {:?}", inspection.version);
                println!("  注册ID  : {:?}", inspection.id);
                println!("  资料片  : {:?}", inspection.campaign_type);
                match &inspection.identification {
                    Some(found) => println!(
                        "  判定依据: {:?}（精确={}）依据内容={:?}",
                        found.evidence,
                        found.evidence.is_exact(),
                        found.detail
                    ),
                    None => println!("  判定依据: 包内已声明"),
                }
                println!("  建议槽位: {:?}", inspection.suggested_slot);
                println!("  内容根  : {:?}", inspection.content_root);
                println!("  封面    : {:?}", inspection.cover);
                println!("  标签    : {:?}", inspection.tags);
                println!("  依赖    : {:?}", inspection.requires);
                println!("  优先级  : {:?}", inspection.priority);
                println!(
                    "  地图 {} / 模组 {} / 载荷 {}",
                    inspection.map_count,
                    inspection.mod_count,
                    inspection.payloads.len()
                );
                for payload in inspection.payloads.iter().take(6) {
                    let target = match &payload.target {
                        PayloadTarget::Mirror { path } => format!("镜像 {path}"),
                        PayloadTarget::Mod { name } => format!("Mods/{name}"),
                        PayloadTarget::Map { name } => format!("战役目录/{name}"),
                    };
                    println!(
                        "    - {} -> {} [{}]",
                        payload.source,
                        target,
                        if payload.expanded {
                            "目录树"
                        } else {
                            "单文件"
                        }
                    );
                }
                if inspection.payloads.len() > 6 {
                    println!("    ... 其余 {} 个", inspection.payloads.len() - 6);
                }
                println!("  可安装  : {}", inspection.installable);
                for issue in &inspection.issues {
                    println!("  ! {:?} {} {}", issue.level, issue.code, issue.message);
                }
            }
            Err(error) => println!("  预检失败: {error}"),
        }
    }
}
