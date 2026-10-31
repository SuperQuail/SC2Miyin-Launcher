//! 多实例数据合并：计划、应用、冲突与遗留目录回收。

use std::path::Path;

/// 造一个只有索引和版本目录的库（合并只认这两样）。
fn seed_library(root: &Path, slot: &str, variants: &[(&str, u64, usize)]) {
    use crate::library::{Library, LibraryIndex};
    let library = Library::new(root);
    let mut index = LibraryIndex {
        version: 1,
        slots: Default::default(),
        patches: Default::default(),
        bindings: Default::default(),
    };
    let entry = index.slots.entry(slot.to_string()).or_default();
    for (id, bytes, maps) in variants {
        std::fs::create_dir_all(root.join("campaigns").join(slot).join(id)).expect("版本目录");
        std::fs::write(
            root.join("campaigns").join(slot).join(id).join("a.txt"),
            b"x",
        )
        .expect("内容");
        let mut record = crate::library::Variant {
            id: id.to_string(),
            name: id.to_string(),
            ..Default::default()
        };
        record.size_bytes = *bytes;
        record.map_count = *maps;
        entry.variants.push(record);
    }
    library.save_index(&index).expect("写索引");
}

#[test]
fn merge_plan_separates_additions_from_conflicts() {
    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    // 本地：一个目标没有的（New），一个同 id 但体积不同的（Same）
    seed_library(&local, "wol", &[("New", 100, 3), ("Same", 999, 5)]);
    seed_library(&target, "wol", &[("Same", 100, 5)]);

    let plan = merge::plan(&local, &target).expect("算合并案");
    assert_eq!(plan.additions.len(), 1);
    assert_eq!(plan.additions[0].variant, "New");
    assert_eq!(plan.conflicts.len(), 1, "同 id 体积不同 = 冲突");
    assert_eq!(plan.conflicts[0].variant, "Same");
    assert!(plan.keep_target.iter().any(|item| item == "active.json"));
    assert!(!plan.is_empty());
}

#[test]
fn merge_apply_moves_additions_and_keeps_both_on_demand() {
    use std::collections::BTreeMap;

    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    seed_library(&local, "wol", &[("New", 100, 3), ("Same", 999, 5)]);
    seed_library(&target, "wol", &[("Same", 100, 5)]);

    let plan = merge::plan(&local, &target).expect("算合并案");
    let mut choices = BTreeMap::new();
    choices.insert("wol/Same".to_string(), merge::MergeChoice::Both);
    let report = merge::apply(&target, &plan, &choices).expect("执行合并");

    assert_eq!(report.added, 1, "New 应该搬进来");
    assert_eq!(report.renamed, 1, "Same 选择了都留");
    assert!(target.join("campaigns").join("wol").join("New").is_dir());
    assert!(
        target.join("campaigns").join("wol").join("Same-2").is_dir(),
        "都留 = 换个目录名"
    );
    assert!(
        target.join("campaigns").join("wol").join("Same").is_dir(),
        "原来那份还在"
    );
    assert!(
        !local.join("campaigns").join("wol").join("New").exists(),
        "搬走了"
    );

    // 索引里三条都在
    let ids: Vec<String> = crate::library::Library::new(&target)
        .index()
        .slots
        .get("wol")
        .expect("槽位")
        .variants
        .iter()
        .map(|item| item.id.clone())
        .collect();
    assert!(ids.contains(&"New".to_string()));
    assert!(ids.contains(&"Same".to_string()));
    assert!(ids.contains(&"Same-2".to_string()));
}

/// S1（verifier 实测的真丢内容）：版本 id 里带点时，备份目录名**不许**把点当扩展名吃掉。
/// `军械库2.4` 与 `军械库2.5` 同时选「用本地这份」，两份备份必须落在不同目录。
#[test]
fn superseded_backup_name_does_not_collide_on_dotted_ids() {
    use std::collections::BTreeMap;

    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    seed_library(
        &local,
        "wol",
        &[("军械库2.4", 999, 5), ("军械库2.5", 888, 5)],
    );
    seed_library(
        &target,
        "wol",
        &[("军械库2.4", 100, 5), ("军械库2.5", 100, 5)],
    );

    let plan = merge::plan(&local, &target).expect("算合并案");
    assert_eq!(plan.conflicts.len(), 2, "两个都该是冲突");
    let mut choices = BTreeMap::new();
    choices.insert("wol/军械库2.4".to_string(), merge::MergeChoice::Local);
    choices.insert("wol/军械库2.5".to_string(), merge::MergeChoice::Local);
    let report = merge::apply(&target, &plan, &choices).expect("执行合并");

    assert_eq!(report.superseded.len(), 2, "两份备份都得留着");
    assert_ne!(
        report.superseded[0], report.superseded[1],
        "两个带点 id 的备份不能撞进同一个目录"
    );
    for path in &report.superseded {
        assert!(path.is_dir(), "{} 要在", path.display());
        assert!(
            path.join("a.txt").is_file(),
            "{} 的内容要完整",
            path.display()
        );
        assert!(
            path.to_string_lossy().contains("军械库2."),
            "名字里要保留完整的原 id：{}",
            path.display()
        );
    }
}

/// S2：目标有**同名目录**但索引里没记录（索引坏过/丢过的典型状态）→ 算冲突，不许静默覆盖。
#[test]
fn a_same_named_dir_without_a_record_is_a_conflict() {
    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    seed_library(&local, "wol", &[("重名战役", 100, 3)]);
    seed_library(&target, "wol", &[]);
    // 目标盘上有目录、索引里没有 —— 直接搬会覆盖同名文件
    std::fs::create_dir_all(target.join("campaigns").join("wol").join("重名战役"))
        .expect("孤儿目录");
    std::fs::write(
        target
            .join("campaigns")
            .join("wol")
            .join("重名战役")
            .join("a.txt"),
        b"TARGET",
    )
    .expect("孤儿内容");

    let plan = merge::plan(&local, &target).expect("算合并案");
    assert!(plan.additions.is_empty(), "不许当成不冲突的加法");
    assert_eq!(plan.conflicts.len(), 1, "要交给用户选");
}

/// N1（verifier 第三轮）：S2 那条「目标有目录、索引无记录」的冲突，用户选「用本地这份」之后，
/// 搬进去的版本**必须进索引** —— 否则磁盘上有、界面上看不见，等于用户视角的丢。
#[test]
fn local_on_a_dir_without_a_record_still_lands_in_the_index() {
    use std::collections::BTreeMap;

    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    seed_library(&local, "wol", &[("无记录战役", 78, 3)]);
    seed_library(&target, "wol", &[]);
    std::fs::create_dir_all(target.join("campaigns").join("wol").join("无记录战役"))
        .expect("目标孤儿目录");
    std::fs::write(
        target
            .join("campaigns")
            .join("wol")
            .join("无记录战役")
            .join("a.txt"),
        b"OLD",
    )
    .expect("孤儿内容");

    let plan = merge::plan(&local, &target).expect("算合并案");
    assert_eq!(plan.conflicts.len(), 1, "这条该是冲突");
    let mut choices = BTreeMap::new();
    choices.insert("wol/无记录战役".to_string(), merge::MergeChoice::Local);
    merge::apply(&target, &plan, &choices).expect("执行合并");

    let ids: Vec<String> = crate::library::Library::new(&target)
        .index()
        .slots
        .get("wol")
        .map(|slot| slot.variants.iter().map(|item| item.id.clone()).collect())
        .unwrap_or_default();
    assert!(
        ids.contains(&"无记录战役".to_string()),
        "搬进去的版本必须在索引里：{ids:?}"
    );
}

/// 合并把本地那份都并干净之后，本地目录要被删掉（不然下次启动又弹一遍）。
#[test]
fn merge_removes_the_local_root_once_everything_is_merged() {
    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    seed_library(&local, "wol", &[("新战役", 100, 3)]);
    seed_library(&target, "wol", &[]);

    let plan = merge::plan(&local, &target).expect("算合并案");
    let report = merge::apply(&target, &plan, &Default::default()).expect("执行合并");
    assert_eq!(report.added, 1);
    assert!(report.removed_local, "并干净了就该把本地删掉");
    assert!(!local.exists(), "本地那份应当已经不在了");
}

/// 反过来：本地还有没并的（用户选了「保持游戏目录那份」），本地目录**不许**删。
#[test]
fn merge_keeps_the_local_root_when_something_is_left_behind() {
    use std::collections::BTreeMap;

    use crate::library::merge;

    let base = tempfile::tempdir().expect("临时目录");
    let local = base.path().join("local");
    let target = base.path().join("target");
    seed_library(&local, "wol", &[("留下的", 999, 3)]);
    seed_library(&target, "wol", &[("留下的", 100, 3)]);

    let plan = merge::plan(&local, &target).expect("算合并案");
    assert_eq!(plan.conflicts.len(), 1);
    let mut choices = BTreeMap::new();
    choices.insert("wol/留下的".to_string(), merge::MergeChoice::Target);
    let report = merge::apply(&target, &plan, &choices).expect("执行合并");
    assert!(!report.removed_local, "还有没并的就不许删本地");
    assert!(
        local.join("campaigns").join("wol").join("留下的").is_dir(),
        "本地那份还在"
    );
}

// ---------- 启用冲突：两个战役抢同一个目标 ----------
