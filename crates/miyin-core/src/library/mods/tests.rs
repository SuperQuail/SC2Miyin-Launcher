//! 内联测试：战役没启用时，它带来的模组不该铺进游戏目录。

use super::*;
use crate::sc2::{DiscoverySource, Installation};

fn mod_record(id: &str, folder: &str, source: ModSource) -> StandaloneMod {
    StandaloneMod {
        id: id.to_string(),
        name: folder.to_string(),
        author: None,
        version: None,
        description: None,
        modid: None,
        fingerprint: String::new(),
        folder: folder.to_string(),
        kind: ModKind::File,
        source,
        enabled: true,
        imported_at: 0,
        size_bytes: 0,
        parts: 1,
    }
}

/// **战役没启用，它带来的模组就不该铺进游戏目录。**
///
/// 战役停了、模组还躺在 `Mods/` 里，那叫"停了个寂寞" —— 而且那些模组本来就是
/// 跟着那一版战役来的。用户自己导的独立模组不受影响。
#[test]
fn campaign_mods_follow_their_campaign() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let game = tmp.path().join("游戏");
    std::fs::create_dir_all(&game).expect("建目录");
    std::fs::write(game.join("StarCraft II.exe"), b"stub").expect("标记");
    let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");

    let data = tmp.path().join("data");

    // 独立模组：内容在库里
    let lib_dir = data.join("mods").join("lib1");
    std::fs::create_dir_all(&lib_dir).expect("建目录");
    std::fs::write(lib_dir.join("独立.SC2Mod"), b"x").expect("写");

    // 战役带来的模组：内容在战役版本目录里
    let camp_dir = data
        .join("campaigns")
        .join("wol")
        .join("v1")
        .join("附带的模组");
    std::fs::create_dir_all(&camp_dir).expect("建目录");
    std::fs::write(camp_dir.join("附带.SC2Mod"), b"y").expect("写");

    let index = serde_json::json!({
        "mods": [
            mod_record("lib1", "独立.SC2Mod", ModSource::Library),
            mod_record(
                "campaign-wol-v1-abc",
                "附带的模组",
                ModSource::Campaign {
                    slot: "wol".to_string(),
                    variant: "v1".to_string(),
                    path: "附带的模组".to_string(),
                },
            ),
        ]
    });
    std::fs::write(data.join("mods.json"), index.to_string()).expect("写索引");

    // wol 没有任何版本被启用（连 library.json 都没有）
    let report = sync(&data, &installation).expect("同步");

    assert!(
        report.placed.iter().any(|name| name.contains("独立")),
        "独立模组照铺：{:?}",
        report.placed
    );
    assert!(
        !report.placed.iter().any(|name| name.contains("附带")),
        "战役没启用，它带来的模组不该铺：{:?}",
        report.placed
    );
    assert!(
        !game.join("Mods").join("附带的模组").exists(),
        "游戏目录里也不该有"
    );
    assert!(
        report.warnings.iter().any(|line| line.contains("没铺")),
        "要有话说明白为什么没铺：{:?}",
        report.warnings
    );
}
