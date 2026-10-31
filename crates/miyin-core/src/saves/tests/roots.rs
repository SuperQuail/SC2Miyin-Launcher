//! 真实存档根（多账号）与隔离状态损坏的测试。
//!
//! 真机上游戏读的是 `Accounts/<账号>/<地域>/Banks`，而
//! `Documents/StarCraft II/Banks` 是空目录 —— 这组用例钉住
//! 「一组 = 所有账号的 Banks 一起切」，以及状态文件坏了不许静默当关闭。

use std::path::{Path, PathBuf};

use super::fixture::write;
use crate::saves::*;

/// 造一份「真机」文档目录：两个账号各有一份 Banks 进度。
fn documents(tmp: &Path) -> PathBuf {
    let docs = tmp.join("StarCraft II");
    write(
        &docs.join("Accounts/A/CN/Banks/地图A"),
        "a.SC2Bank",
        "A 的进度",
    );
    write(&docs.join("Accounts/B/NA/Banks"), "b.SC2Bank", "B 的进度");
    docs
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("读存档")
}

#[test]
fn discovery_finds_every_accounts_banks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let docs = documents(tmp.path());

    let roots = SavesRoots::discover(&docs);
    assert_eq!(
        roots.banks().len(),
        2,
        "两个账号的 Banks 都算：{:?}",
        roots.banks()
    );

    let names: Vec<String> = snapshot_paths(&roots)
        .expect("快照")
        .files
        .into_iter()
        .map(|file| file.name)
        .collect();
    assert_eq!(
        names,
        vec![
            "Accounts/A/CN/Banks/地图A/a.SC2Bank",
            "Accounts/B/NA/Banks/b.SC2Bank"
        ],
        "相对 documents 的层级要留着（组里按它存）"
    );

    // 老入口：传那个空的老 Banks 也按真实多根来（commands::campaign 走的正是它）
    std::fs::create_dir_all(docs.join("Banks")).expect("建空 Banks");
    let legacy = SavesRoots::from_banks(&docs.join("Banks"));
    assert_eq!(
        legacy.banks().len(),
        2,
        "旁边有 Accounts 就不能只看那个没人读的空目录"
    );
}

#[test]
fn isolation_moves_every_accounts_saves_together() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let docs = documents(tmp.path());
    let data = tmp.path().join("data");
    let roots = SavesRoots::discover(&docs);

    // 打开隔离：两个账号的进度一起收进「原版」一组，账号 / 地域层级保留
    let state = set_isolation_paths(&data, &roots, true).expect("开隔离");
    let vanilla = state.active.clone().expect("现有进度要收成原版");
    let group = root(&data).join(&vanilla);
    assert_eq!(
        read(&group.join("Accounts/A/CN/Banks/地图A/a.SC2Bank")),
        "A 的进度",
        "A 的进度要进组，而且带着账号 / 地域层级"
    );
    assert_eq!(
        read(&group.join("Accounts/B/NA/Banks/b.SC2Bank")),
        "B 的进度"
    );

    // 两个账号各打了一关：存回原版组
    write(
        &docs.join("Accounts/A/CN/Banks/地图A"),
        "a.SC2Bank",
        "A 新进度",
    );
    write(&docs.join("Accounts/B/NA/Banks"), "b.SC2Bank", "B 新进度");
    save_active_paths(&data, &roots).expect("存回");
    assert_eq!(
        read(&group.join("Accounts/A/CN/Banks/地图A/a.SC2Bank")),
        "A 新进度"
    );
    assert_eq!(
        read(&group.join("Accounts/B/NA/Banks/b.SC2Bank")),
        "B 新进度"
    );

    // 换到一个空的新战役组：**两个账号**的 Banks 一起清空
    let profile = ensure_for_slot_paths(&data, &roots, "custom/甲", "战役甲")
        .expect("切")
        .expect("开着");
    assert_ne!(profile, vanilla);
    assert!(
        snapshot_paths(&roots).expect("快照").files.is_empty(),
        "两个账号都要清空"
    );
    assert!(
        !docs.join("Accounts/A/CN/Banks/地图A").exists(),
        "A 的地图子目录也要清掉，别留下上一部的旧档"
    );

    // 切回原版：两个账号的进度原样铺回
    switch_paths(&data, &roots, &vanilla).expect("切回");
    assert_eq!(
        read(&docs.join("Accounts/A/CN/Banks/地图A/a.SC2Bank")),
        "A 新进度"
    );
    assert_eq!(
        read(&docs.join("Accounts/B/NA/Banks/b.SC2Bank")),
        "B 新进度"
    );
}

#[test]
fn empty_banks_never_make_an_empty_snapshot() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let docs = tmp.path().join("StarCraft II");
    std::fs::create_dir_all(docs.join("Accounts/A/CN/Banks")).expect("建空 Banks");
    let data = tmp.path().join("data");
    let roots = SavesRoots::discover(&docs);

    assert_eq!(roots.banks().len(), 1);
    let set = snapshot_paths(&roots).expect("快照");
    assert!(!set.missing, "目录在，只是没有文件");
    assert!(set.files.is_empty());
    assert!(
        backup_paths(&roots, &data, "手动").is_err(),
        "空 Banks 不造空快照"
    );

    let state = set_isolation_paths(&data, &roots, true).expect("开关照常打开");
    assert!(state.active.is_none(), "没东西可收，先空着");
    assert!(backups(&data).expect("列").is_empty(), "一个组都不该建");
}

#[test]
fn without_accounts_it_falls_back_to_the_old_banks_folder() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let docs = tmp.path().join("StarCraft II");
    write(&docs.join("Banks"), "old.SC2Bank", "老进度");

    let roots = SavesRoots::discover(&docs);
    assert_eq!(
        roots.banks(),
        &[docs.join("Banks")],
        "没有 Accounts 才退回旧目录"
    );
    assert_eq!(
        snapshot_paths(&roots).expect("快照").files[0].name,
        "old.SC2Bank",
        "退回旧目录时组里是平铺文件名（和老格式的组一致）"
    );

    // 还原一遍：铺回原位
    let data = tmp.path().join("data");
    let name = backup_paths(&roots, &data, "老格式").expect("备份");
    std::fs::write(docs.join("Banks/old.SC2Bank"), "被改过").expect("改");
    restore_paths(&roots, &data, &name).expect("还原");
    assert_eq!(read(&docs.join("Banks/old.SC2Bank")), "老进度");
}

#[test]
fn a_broken_state_file_is_reported_and_kept_as_evidence() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let saves_dir = data.join("saves");
    std::fs::create_dir_all(&saves_dir).expect("建目录");
    std::fs::write(saves_dir.join("state.json"), "{ 这不是 JSON").expect("写坏文件");

    let state = isolation(&data).expect("读");
    assert!(!state.enabled, "坏了还是按关着处理（不假装开着）");
    let warning = state.warning.clone().expect("不许静默：要给一条人话");
    assert!(warning.contains("状态文件"), "{warning}");

    // 下一次写盘：坏文件先挪到一边，别把里面还记得的归属直接盖掉
    let banks = tmp.path().join("Banks");
    write(&banks, "a.SC2Bank", "进度");
    set_isolation(&data, &banks, true).expect("打开");
    let kept: Vec<String> = std::fs::read_dir(&saves_dir)
        .expect("列")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("state.json.corrupt-"))
        .collect();
    assert_eq!(kept.len(), 1, "坏文件要留证据：{kept:?}");
    assert!(
        isolation(&data).expect("再读").warning.is_none(),
        "新状态是好的"
    );
    assert!(isolation(&data).expect("再读").enabled);
}

#[test]
fn a_nested_save_can_move_between_groups() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let nested = "Accounts/A/CN/Banks/地图A/a.SC2Bank";
    write(
        &root(&data).join("甲/Accounts/A/CN/Banks/地图A"),
        "a.SC2Bank",
        "进度",
    );
    std::fs::create_dir_all(root(&data).join("乙")).expect("建乙");

    move_save_file(&data, "甲", nested, "乙").expect("挪");
    assert_eq!(
        read(&root(&data).join("乙").join(nested)),
        "进度",
        "层级要跟着走"
    );
    assert!(!root(&data).join("甲").join(nested).exists(), "源组里没了");

    // 带 .. 的照样拒绝（跳不出 data/saves）
    assert!(move_save_file(&data, "乙", "../a.SC2Bank", "甲").is_err());
}
