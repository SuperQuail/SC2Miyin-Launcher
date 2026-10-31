//! 备份、还原与存档组文件搬运的测试。

use super::fixture::{library_with_lotv_mod, write};
use crate::saves::*;

#[test]
fn missing_banks_is_not_an_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let set = snapshot(&tmp.path().join("没有这个目录")).expect("快照");
    assert!(set.missing);
    assert!(set.files.is_empty());
}

#[test]
fn backup_then_restore_round_trips() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");

    write(&banks, "ENS.SC2Bank", "first");
    let name = backup(&banks, &library, "复刻战役").expect("备份");
    assert!(name.starts_with("复刻战役-"), "目录名要带标签：{name}");

    write(&banks, "ENS.SC2Bank", "second");
    write(&banks, "新文件.SC2Bank", "junk");

    let safety = restore(&banks, &library, &name).expect("还原");
    assert_eq!(
        std::fs::read_to_string(banks.join("ENS.SC2Bank")).expect("读"),
        "first"
    );
    assert!(!banks.join("新文件.SC2Bank").exists(), "备份里没有的要清掉");
    assert!(safety.starts_with("还原前-"), "还原前要留后路：{safety}");
    assert_eq!(
        std::fs::read_to_string(root(&library).join(&safety).join("ENS.SC2Bank")).expect("读"),
        "second"
    );
}

#[test]
fn listing_shows_labels() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");
    write(&banks, "a.SC2Bank", "x");
    backup(&banks, &library, "第一部").expect("备份");

    let list = backups(&library).expect("列备份");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].label, "第一部");
    assert_eq!(list[0].files, 1);
}

#[test]
fn group_files_lists_one_group_only() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    write(&root(&data).join("甲"), "a.SC2Bank", "1");
    write(&root(&data).join("甲"), "b.SC2Bank", "22");
    write(&root(&data).join("乙"), "c.SC2Bank", "333");

    let files = group_files(&data, "甲").expect("列甲");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].name, "a.SC2Bank");
    assert_eq!(files[0].bytes, 1);
    assert_eq!(files[1].name, "b.SC2Bank");
    assert_eq!(files[1].bytes, 2);

    // 空组是正常状态
    std::fs::create_dir_all(root(&data).join("空")).expect("建组");
    assert!(group_files(&data, "空").expect("列空组").is_empty());

    // 没有这个组要说清楚是哪一组
    let err = group_files(&data, "查无此组").expect_err("该报错");
    assert!(format!("{err}").contains("查无此组"), "{err}");
}

#[test]
fn moving_a_save_file_moves_it_between_groups_and_never_touches_banks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // 游戏目录那份 Banks 也放一个**同名**文件：绝不能被碰
    let banks = tmp.path().join("Banks");
    write(&banks, "a.SC2Bank", "游戏里那份");
    write(&root(&data).join("甲"), "a.SC2Bank", "进度");
    write(&root(&data).join("乙"), "b.SC2Bank", "别的");

    move_save_file(&data, "甲", "a.SC2Bank", "乙").expect("挪");

    assert!(
        !root(&data).join("甲").join("a.SC2Bank").exists(),
        "源组里没了"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join("乙").join("a.SC2Bank")).expect("读"),
        "进度"
    );
    assert!(root(&data).join("甲").is_dir(), "源组空了也不删组");
    assert_eq!(
        std::fs::read_to_string(banks.join("a.SC2Bank")).expect("读"),
        "游戏里那份",
        "只动 data/saves，绝不碰 Banks"
    );
}

#[test]
fn moving_refuses_to_overwrite_a_same_named_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    write(&root(&data).join("甲"), "a.SC2Bank", "旧的");
    write(&root(&data).join("乙"), "a.SC2Bank", "新的");

    let err = move_save_file(&data, "甲", "a.SC2Bank", "乙").expect_err("必须拒绝");
    assert!(format!("{err}").contains("同名"), "{err}");

    // 两边都没被动过
    assert_eq!(
        std::fs::read_to_string(root(&data).join("甲").join("a.SC2Bank")).expect("读"),
        "旧的"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join("乙").join("a.SC2Bank")).expect("读"),
        "新的"
    );
}

#[test]
fn moving_creates_the_target_group_and_rejects_path_like_names() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    write(&root(&data).join("甲"), "b.SC2Bank", "内容");

    // 目标组不存在就建
    move_save_file(&data, "甲", "b.SC2Bank", "新建的组").expect("目标组自动建");
    assert_eq!(
        std::fs::read_to_string(root(&data).join("新建的组").join("b.SC2Bank")).expect("读"),
        "内容"
    );

    // 文件名 / 组名都不许带路径（跳出 data/saves 的一道闸）
    write(&root(&data).join("甲"), "c.SC2Bank", "x");
    assert!(move_save_file(&data, "甲", "../c.SC2Bank", "新建的组").is_err());
    assert!(move_save_file(&data, "..", "c.SC2Bank", "新建的组").is_err());
    assert!(move_save_file(&data, "甲", "c.SC2Bank", "..").is_err());
}

#[test]
fn restore_safety_backup_is_marked_auto() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    write(&banks, "进度.SC2Bank", "原来的");

    let manual = backup(&banks, &data, "手动备份").expect("备份");
    write(&banks, "进度.SC2Bank", "新的");
    let safety = restore(&banks, &data, &manual).expect("还原");
    assert!(safety.starts_with(AUTO_BACKUP), "还原前留的那份：{safety}");

    let list = backups(&data).expect("列");
    let auto: Vec<&str> = list
        .iter()
        .filter(|item| item.auto)
        .map(|item| item.name.as_str())
        .collect();
    let groups: Vec<&str> = list
        .iter()
        .filter(|item| !item.auto)
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(auto, vec![safety.as_str()], "「还原前」要标成自动备份");
    assert_eq!(groups, vec![manual.as_str()], "组列表里只留用户自己的组");
}

#[test]
fn move_save_to_key_uses_the_existing_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let _library = library_with_lotv_mod(&data);
    write(&root(&data).join("旧组"), "a.SC2Bank", "进度");
    std::fs::create_dir_all(root(&data).join("现有组")).expect("建组");
    assign(&data, "现有组", Some("lotv/肉鸽")).expect("绑定");

    let moved = move_save_to_key(&data, "旧组", "a.SC2Bank", "lotv/肉鸽").expect("挪");
    assert_eq!(moved, "现有组", "已有组就用它，不该再造一个");
    assert_eq!(
        std::fs::read_to_string(root(&data).join("现有组").join("a.SC2Bank")).expect("读"),
        "进度"
    );
    assert_eq!(backups(&data).expect("列").len(), 2, "只有旧组和现有组");
}

#[test]
fn move_save_to_key_creates_and_binds_a_new_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let _library = library_with_lotv_mod(&data);
    write(&root(&data).join("旧组"), "a.SC2Bank", "进度");

    let moved = move_save_to_key(&data, "旧组", "a.SC2Bank", "lotv/肉鸽").expect("挪");
    // 组名和启用时造组一致「战役名 · 版本名」（空格会被折成 _）
    assert!(moved.starts_with("虚空之遗"), "{moved}");
    assert!(moved.contains("肉鸽"), "{moved}");
    assert_eq!(
        profile_for(&data, "lotv/肉鸽").expect("查"),
        Some(moved.clone()),
        "新组要自动绑定到这个键"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&moved).join("a.SC2Bank")).expect("读"),
        "进度"
    );
    assert!(
        !root(&data).join("旧组").join("a.SC2Bank").exists(),
        "源组里没了"
    );
    assert!(root(&data).join("旧组").is_dir(), "源组空了也不删");
}

#[test]
fn move_save_to_key_refuses_same_name_and_unknown_keys() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let _library = library_with_lotv_mod(&data);
    write(&root(&data).join("甲"), "a.SC2Bank", "旧的");
    std::fs::create_dir_all(root(&data).join("乙")).expect("建组");
    assign(&data, "乙", Some("lotv/肉鸽")).expect("绑定");
    write(&root(&data).join("乙"), "a.SC2Bank", "新的");

    let err = move_save_to_key(&data, "甲", "a.SC2Bank", "lotv/肉鸽").expect_err("必须拒绝");
    assert!(format!("{err}").contains("同名"), "{err}");
    assert_eq!(
        std::fs::read_to_string(root(&data).join("甲").join("a.SC2Bank")).expect("读"),
        "旧的"
    );

    // 不认识的键：不能凭空造一个名字都说不清的组
    let err = move_save_to_key(&data, "甲", "a.SC2Bank", "lotv/不存在的版本").expect_err("要拒绝");
    assert!(format!("{err}").contains("未知的归属"), "{err}");
    assert_eq!(backups(&data).expect("列").len(), 2, "只该有甲和乙两个组");
}

#[test]
fn move_save_to_key_never_touches_banks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let _library = library_with_lotv_mod(&data);
    // 游戏目录那份也放一份同名文件：挪动绝不能被它碰到
    let banks = tmp.path().join("Banks");
    write(&banks, "a.SC2Bank", "游戏里那份");
    write(&root(&data).join("旧组"), "a.SC2Bank", "进度");

    move_save_to_key(&data, "旧组", "a.SC2Bank", "lotv/肉鸽").expect("挪");
    assert_eq!(
        std::fs::read_to_string(banks.join("a.SC2Bank")).expect("读"),
        "游戏里那份",
        "只动 data/saves，绝不碰 Banks"
    );
}

#[test]
fn unbound_groups_are_marked_legacy() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    write(&root(&data).join("原版-1"), "a.SC2Bank", "老进度");

    let entry = backups(&data).expect("列").pop().expect("有组");
    assert!(entry.legacy, "没有绑定的组要标 legacy（名字不代表内容）");
    assert!(!entry.auto);

    // 绑上某个战役之后就不再是 legacy
    assign(&data, "原版-1", Some("wol")).expect("绑定");
    let entry = backups(&data).expect("列").pop().expect("有组");
    assert!(!entry.legacy);

    // 自动备份不算 legacy（它另有一套 auto 规则）
    let banks = tmp.path().join("Banks");
    write(&banks, "a.SC2Bank", "进度");
    let safety = backup(&banks, &data, AUTO_BACKUP).expect("自动备份");
    let entry = backups(&data)
        .expect("列")
        .into_iter()
        .find(|item| item.name == safety)
        .expect("有自动备份");
    assert!(entry.auto);
    assert!(!entry.legacy);
}

#[test]
fn two_backups_with_the_same_label_never_share_a_directory() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    write(&banks, "a.SC2Bank", "第一份");
    let first = backup(&banks, &data, "同名").expect("第一次");

    // 同一秒内的第二次备份：必须另开一个目录（重名 = 后一份倒进前一份里）
    write(&banks, "a.SC2Bank", "第二份");
    let second = backup(&banks, &data, "同名").expect("第二次");
    assert_ne!(first, second, "同一秒内的两次备份不能共用一个目录");
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&first).join("a.SC2Bank")).expect("读"),
        "第一份",
        "先备份的那份不许被后一份盖掉"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&second).join("a.SC2Bank")).expect("读"),
        "第二份"
    );
}
