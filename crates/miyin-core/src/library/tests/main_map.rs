//! 主地图（入口地图）解析。

use crate::library::{MapEntry, resolve_main_map};

#[test]
fn main_map_resolution_never_guesses() {
    let maps = vec![
        MapEntry {
            path: "1. Rebel Yell/Terran01.SC2Map".to_string(),
            name: "Terran01".to_string(),
            chapter: Some("1. Rebel Yell".to_string()),
            size: 0,
            is_main: false,
        },
        MapEntry {
            path: "2. Overmind/Zerg01.SC2Map".to_string(),
            name: "Zerg01".to_string(),
            chapter: Some("2. Overmind".to_string()),
            size: 0,
            is_main: false,
        },
    ];

    // 声明了且存在 -> 用它
    let hit = resolve_main_map(&maps, Some("1. Rebel Yell/Terran01.SC2Map"));
    assert_eq!(hit.path.as_deref(), Some("1. Rebel Yell/Terran01.SC2Map"));
    assert!(hit.warning.is_none());

    // 大小写与斜杠方向都该认
    let loose = resolve_main_map(&maps, Some("1. rebel yell\\terran01.sc2map"));
    assert_eq!(loose.path.as_deref(), Some("1. Rebel Yell/Terran01.SC2Map"));

    // 只写文件名也认
    let by_name = resolve_main_map(&maps, Some("Zerg01"));
    assert_eq!(by_name.path.as_deref(), Some("2. Overmind/Zerg01.SC2Map"));

    // 声明了但找不到 -> 只警告，不阻断
    let missing = resolve_main_map(&maps, Some("3. The Fall/Protoss99.SC2Map"));
    assert_eq!(missing.path, None);
    assert!(missing.warning.is_some());

    // 没声明、有多张 -> 不猜
    let unset = resolve_main_map(&maps, None);
    assert_eq!(unset.path, None);
    assert!(!unset.automatic);

    // 没声明、只有一张 -> 替你选（省一次点击）
    let solo = resolve_main_map(&maps[..1], None);
    assert_eq!(solo.path.as_deref(), Some("1. Rebel Yell/Terran01.SC2Map"));
    assert!(solo.automatic);
}

#[test]
fn natural_order_puts_terran2_before_terran10() {
    let mut names = vec!["Terran10", "Terran2", "Terran1"];
    names.sort_by(|left, right| crate::library::natural_cmp(left, right));
    assert_eq!(names, vec!["Terran1", "Terran2", "Terran10"]);

    // 章节也要按数字排
    let mut chapters = vec!["2. Overmind", "10. Later", "1. Rebel Yell"];
    chapters.sort_by(|left, right| crate::library::natural_cmp(left, right));
    assert_eq!(chapters, vec!["1. Rebel Yell", "2. Overmind", "10. Later"]);
}

/// 打表的优先级要真的接在 `resolve_main_map` 里（known.rs 测的是表本身，
/// 这里钉的是「表排在启发式前面」这条接线）。
#[test]
fn the_preset_table_outranks_the_heuristics() {
    let entry = |path: &str, name: &str, chapter: Option<&str>| MapEntry {
        path: path.to_string(),
        name: name.to_string(),
        chapter: chapter.map(str::to_string),
        size: 0,
        is_main: false,
    };

    // 包根有一张 `Launcher`（启发式会选它），但表里点名的两张才是正经入口。
    let maps = vec![
        entry("Launcher.SC2Map", "Launcher", None),
        entry(
            "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
            "SCMR Campaign Launcher",
            Some("Starcraft Mass Recall"),
        ),
        entry(
            "Starcraft Mass Recall/SCMR RPG Campaign Launcher.SC2Map",
            "SCMR RPG Campaign Launcher",
            Some("Starcraft Mass Recall"),
        ),
    ];

    let choice = resolve_main_map(&maps, None);
    assert_eq!(
        choice.path.as_deref(),
        Some("Starcraft Mass Recall/SCMR RPG Campaign Launcher.SC2Map"),
        "群雄纪元的入口优先级最高，不能输给包根那张 Launcher"
    );
    assert!(choice.automatic, "打表结果也是推测，界面要按推测展示");
}

/// 最后的兜底：不认识任何预设时，不区分大小写的 `Launcher`。
#[test]
fn the_generic_launcher_fallback_is_case_insensitive() {
    let entry = |path: &str, name: &str| MapEntry {
        path: path.to_string(),
        name: name.to_string(),
        chapter: None,
        size: 0,
        is_main: false,
    };

    let maps = vec![
        entry("Some Campaign/1. First/a.SC2Map", "a"),
        entry("LAUNCHER.SC2Map", "LAUNCHER"),
    ];

    let choice = resolve_main_map(&maps, None);
    assert_eq!(choice.path.as_deref(), Some("LAUNCHER.SC2Map"));
    assert!(choice.automatic);
}
