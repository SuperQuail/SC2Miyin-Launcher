//! 切组与隔离开关的测试。

use std::path::Path;

use super::fixture::{library_with_custom, library_with_lotv_mod_active, write};
use crate::library::Library;
use crate::saves::groups::{state_file, write_isolation};
use crate::saves::*;

/// 直接把隔离开着写进 state.json（模拟"打开过开关、但还没归过档"）。
///
/// 这几个用例要的是「Banks 里已经有进度、但还没有任何组」的状态；走
/// `set_isolation(true)` 会先把它收成「原版」，正是这些用例要避开的。
fn enable_isolation(root: &Path) {
    let state = Isolation {
        enabled: true,
        ..Default::default()
    };
    write_isolation(root, &state).expect("开隔离");
}

#[test]
fn isolation_defaults_to_disabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let library = tmp.path().join("data");

    // 没有 state.json 时：默认**关着**，什么归属都没有
    let state = isolation(&library).expect("读");
    assert!(
        !state.enabled,
        "没有 state.json 时默认关着（不替用户动 Banks）"
    );
    assert!(state.active.is_none(), "还没归过档");
    assert!(state.assignments.is_empty(), "也不该有归属");
}

#[test]
fn opening_isolation_stores_the_current_progress_first() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");
    write(&banks, "原版.SC2Bank", "playthrough");

    let state = set_isolation(&library, &banks, true).expect("打开");
    assert!(state.enabled);
    let active = state.active.clone().expect("要归过档");
    assert!(active.starts_with(ORIGINAL), "初始那份算原版：{active}");
    assert_eq!(
        std::fs::read_to_string(root(&library).join(&active).join("原版.SC2Bank")).expect("读"),
        "playthrough",
        "打开隔离不能把用户原有的进度弄丢"
    );
}

#[test]
fn opening_isolation_without_any_saves_still_works() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");

    // 还没玩过（Banks 不存在）：打开开关不能被"没有存档可备份"挡回去
    let state = set_isolation(&library, &banks, true).expect("打开");
    assert!(state.enabled);
    assert!(state.active.is_none(), "没东西可收，先空着");
}

#[test]
fn official_slot_without_record_inherits_the_current_saves() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");
    write(&banks, "进度.SC2Bank", "玩到第三章");
    enable_isolation(&library);

    // 没有记录时启用官方战役：当前这份收成它的组（不是空组）
    let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
        .expect("切换")
        .expect("开着隔离就该切");
    assert!(profile.contains("自由之翼"), "组名带战役名：{profile}");

    let group = snapshot(&root(&library).join(&profile)).expect("快照");
    assert_eq!(group.files.len(), 1, "新组不该是空的：{group:?}");
    assert_eq!(
        std::fs::read_to_string(root(&library).join(&profile).join("进度.SC2Bank")).expect("读"),
        "玩到第三章",
        "没有记录时默认归当前启用的战役"
    );

    // 「原版」组照旧保留，而且是同一份 —— 刻意的：谁也没被顶掉
    let original = backups(&library)
        .expect("列")
        .into_iter()
        .find(|item| item.label == ORIGINAL)
        .expect("原版组要留着");
    assert_eq!(
        std::fs::read_to_string(root(&library).join(&original.name).join("进度.SC2Bank"))
            .expect("读"),
        "玩到第三章",
        "老用户升级后第一份进度不能被当成空组切走"
    );
}

#[test]
fn custom_slot_without_record_gets_an_empty_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let data = tmp.path().join("data");
    // 每部自制战役一个键（custom/<身份>）—— 没有「自制战役原版」这种东西
    let library = library_with_custom(&data, &[("甲", "战役甲", None)]);
    write(&banks, "进度.SC2Bank", "原版进度");
    enable_isolation(&data);

    // 自制战役各自独立：没有记录时**不继承**当前存档
    let key = isolation_key(&library, "custom", Some("甲"));
    let profile = ensure_for_slot(&data, &banks, &key, "战役甲")
        .expect("切换")
        .expect("开着隔离就该切");
    assert!(
        snapshot(&banks).expect("快照").files.is_empty(),
        "自制战役该从零开始"
    );
    assert!(
        snapshot(&root(&data).join(&profile))
            .expect("快照")
            .files
            .is_empty(),
        "组本身也是空的"
    );

    // 原来那份还在「原版」里
    let original = backups(&data)
        .expect("列")
        .into_iter()
        .find(|item| item.label == ORIGINAL)
        .expect("原版组要留着");
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&original.name).join("进度.SC2Bank")).expect("读"),
        "原版进度"
    );
}

#[test]
fn switching_swaps_the_saves_and_remembers_where_we_were() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");
    write(&banks, "进度.SC2Bank", "原版第一章");
    enable_isolation(&library);

    // 官方战役第一次启用：接着当前这份进度（见上一个用例）
    let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
        .expect("切换")
        .expect("开了隔离就该切");
    assert!(profile.contains("自由之翼"), "组名带战役名：{profile}");
    assert_eq!(
        std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
        "原版第一章"
    );
    assert_eq!(
        profile_for(&library, "wol").expect("查"),
        Some(profile.clone())
    );

    // 在这个战役里玩一会儿
    write(&banks, "进度.SC2Bank", "自由之翼第一章");

    // 切回原版：原版的档要回来，新战役这份要存好
    let original = isolation(&library)
        .expect("读")
        .assignments
        .get(&profile)
        .map(|_| ())
        .map(|_| ORIGINAL.to_string());
    let name = backups(&library)
        .expect("列")
        .into_iter()
        .find(|item| item.label == ORIGINAL)
        .expect("原版组")
        .name;
    switch(&library, &banks, &name).expect("切回原版");
    assert_eq!(
        std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
        "原版第一章"
    );
    let _ = original;

    // 再切回那个战役：它自己的进度还在
    switch(&library, &banks, &profile).expect("切回战役");
    assert_eq!(
        std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
        "自由之翼第一章"
    );
}

#[test]
fn assigning_a_profile_steals_it_from_the_other_campaign() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let library = tmp.path().join("data");
    std::fs::create_dir_all(root(&library)).expect("建目录");

    assign(&library, "手头这份", Some("wol")).expect("指派");
    assign(&library, "手头这份", Some("hots")).expect("改派");
    assert_eq!(
        profile_for(&library, "hots").expect("查"),
        Some("手头这份".to_string())
    );
    assert_eq!(
        profile_for(&library, "wol").expect("查"),
        None,
        "一个战役只占一个组"
    );

    assign(&library, "手头这份", None).expect("摘掉");
    assert_eq!(profile_for(&library, "hots").expect("查"), None);
}

#[test]
fn saving_into_a_named_group_targets_that_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");
    write(&banks, "进度.SC2Bank", "原版");
    enable_isolation(&library);
    let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
        .expect("切换")
        .expect("开着隔离就该切");

    // 现在游戏里换成另一份内容，把它存进那个组
    write(&banks, "进度.SC2Bank", "最新进度");
    write(&banks, "新档.SC2Bank", "新");
    let state = save_into(&library, &banks, &profile).expect("存进去");
    assert_eq!(state.active.as_deref(), Some(profile.as_str()));
    assert_eq!(
        std::fs::read_to_string(root(&library).join(&profile).join("进度.SC2Bank")).expect("读"),
        "最新进度"
    );
    assert!(
        root(&library).join(&profile).join("新档.SC2Bank").is_file(),
        "新文件也要进去"
    );

    // 不存在的组要报错，别悄悄建一个
    assert!(save_into(&library, &banks, "没有这个组").is_err());
}

#[test]
fn default_off_leaves_banks_and_state_alone() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let banks = tmp.path().join("Banks");
    let library = tmp.path().join("data");
    write(&banks, "进度.SC2Bank", "玩到第三章");
    let before = std::fs::read(banks.join("进度.SC2Bank")).expect("读");

    // 用户没打开隔离（没有 state.json）：启用战役一律不碰 Banks
    assert_eq!(
        ensure_for_slot(&library, &banks, "wol", "自由之翼").expect("调"),
        None,
        "默认关着就不该切档"
    );
    assert_eq!(
        std::fs::read(banks.join("进度.SC2Bank")).expect("读"),
        before,
        "Banks 一个字节都不能动"
    );
    assert_eq!(
        snapshot(&banks).expect("快照").files.len(),
        1,
        "文件不多不少"
    );
    assert!(
        !state_file(&library).exists(),
        "默认关着时连 state.json 都不写"
    );
    assert!(backups(&library).expect("列").is_empty(), "也不该凭空建组");
}

#[test]
fn two_custom_campaigns_get_separate_groups() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    let library = library_with_custom(&data, &[("甲", "战役甲", None), ("乙", "战役乙", None)]);
    set_isolation(&data, &banks, true).expect("开隔离");

    let a = ensure_for_slot(
        &data,
        &banks,
        &isolation_key(&library, "custom", Some("甲")),
        "战役甲",
    )
    .expect("切甲")
    .expect("开着");
    let b = ensure_for_slot(
        &data,
        &banks,
        &isolation_key(&library, "custom", Some("乙")),
        "战役乙",
    )
    .expect("切乙")
    .expect("开着");

    assert_ne!(a, b, "两部自制战役必须是两个组：{a} / {b}");
    assert_eq!(
        profile_for(&data, "custom/甲").expect("查"),
        Some(a.clone())
    );
    assert_eq!(
        profile_for(&data, "custom/乙").expect("查"),
        Some(b.clone())
    );

    // 各自的组是空的（自制战役不继承当前存档）
    assert!(
        snapshot(&root(&data).join(&a))
            .expect("快照")
            .files
            .is_empty()
    );
    assert!(
        snapshot(&root(&data).join(&b))
            .expect("快照")
            .files
            .is_empty()
    );

    // 名字不撞（组名带战役名）
    assert!(a.contains("战役甲"), "{a}");
    assert!(b.contains("战役乙"), "{b}");
}

#[test]
fn versions_of_one_custom_campaign_share_a_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    // 同一部战役的两个版本：包内声明的注册 ID 相同（大小写不同也算同一个）
    let library = library_with_custom(
        &data,
        &[
            ("v1", "战役 1.0", Some("Team.Campaign")),
            ("v2", "战役 2.0", Some("team.campaign")),
        ],
    );

    set_isolation(&data, &banks, true).expect("开隔离");
    let key1 = isolation_key(&library, "custom", Some("v1"));
    let key2 = isolation_key(&library, "custom", Some("v2"));
    assert_eq!(key1, key2, "注册 ID 一样就得是同一个键");

    let first = ensure_for_slot(&data, &banks, &key1, "战役 1.0")
        .expect("切 1.0")
        .expect("开着");
    let second = ensure_for_slot(&data, &banks, &key2, "战役 2.0")
        .expect("切 2.0")
        .expect("开着");
    assert_eq!(first, second, "换版本要共用同一个组");
    assert_eq!(backups(&data).expect("列").len(), 1, "不该多出一个组");
}

#[test]
fn official_vanilla_and_mod_versions_get_separate_groups() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    write(&banks, "原版进度.SC2Bank", "玩到第三章");
    let library = library_with_custom(&data, &[]);
    set_isolation(&data, &banks, true).expect("开隔离");

    // 原版一份（键 = 槽位），改版一份（键 = 槽位/身份）
    let vanilla = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("切原版")
        .expect("开着");
    let modded = ensure_for_slot(&data, &banks, "lotv/肉鸽", "虚空之遗 · 肉鸽")
        .expect("切改版")
        .expect("开着");

    assert_ne!(vanilla, modded, "原版和改版必须是两个组");
    assert_eq!(
        profile_for(&data, "lotv").expect("查"),
        Some(vanilla.clone())
    );
    assert_eq!(
        profile_for(&data, "lotv/肉鸽").expect("查"),
        Some(modded.clone())
    );
    // 原版组有那份进度（第一次启用时收进去的），改版组是空的（不串档）
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&vanilla).join("原版进度.SC2Bank")).expect("读"),
        "玩到第三章"
    );
    assert!(
        snapshot(&root(&data).join(&modded))
            .expect("快照")
            .files
            .is_empty(),
        "已经有组在用的时候，改版从空组开始，别把原版进度抄过来"
    );
    let _ = &library;
}

#[test]
fn official_mod_versions_sharing_registration_share_a_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    index.slots.insert(
        "lotv".to_string(),
        crate::library::CampaignSlot {
            variants: vec![
                crate::library::Variant {
                    id: "肉鸽 v1".to_string(),
                    name: "肉鸽 1.0".to_string(),
                    registration_id: Some("Team.Rogue".to_string()),
                    imported_at: 10,
                    ..Default::default()
                },
                crate::library::Variant {
                    id: "肉鸽 v2".to_string(),
                    name: "肉鸽 2.0".to_string(),
                    registration_id: Some("team.rogue".to_string()),
                    imported_at: 20,
                    ..Default::default()
                },
            ],
            active: None,
        },
    );
    library.save_index(&index).expect("写索引");

    set_isolation(&data, &banks, true).expect("开隔离");
    let key1 = isolation_key(&library, "lotv", Some("肉鸽 v1"));
    let key2 = isolation_key(&library, "lotv", Some("肉鸽 v2"));
    assert_eq!(key1, key2, "同一部改版换版本 -> 同一个键");

    let first = ensure_for_slot(&data, &banks, &key1, "虚空之遗 · 肉鸽 1.0")
        .expect("切 1.0")
        .expect("开着");
    let second = ensure_for_slot(&data, &banks, &key2, "虚空之遗 · 肉鸽 2.0")
        .expect("切 2.0")
        .expect("开着");
    assert_eq!(first, second, "换版本要共用同一个组");
    assert_eq!(backups(&data).expect("列").len(), 1, "不该多出一个组");
    // 原版那个键没被牵连
    assert_eq!(profile_for(&data, "lotv").expect("查"), None);
}

#[test]
fn two_different_official_mods_get_two_groups() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    index.slots.insert(
        "lotv".to_string(),
        crate::library::CampaignSlot {
            variants: vec![
                crate::library::Variant {
                    id: "肉鸽".to_string(),
                    name: "肉鸽".to_string(),
                    ..Default::default()
                },
                crate::library::Variant {
                    id: "残卷".to_string(),
                    name: "残卷".to_string(),
                    ..Default::default()
                },
            ],
            active: None,
        },
    );
    library.save_index(&index).expect("写索引");

    set_isolation(&data, &banks, true).expect("开隔离");
    let a = ensure_for_slot(
        &data,
        &banks,
        &isolation_key(&library, "lotv", Some("肉鸽")),
        "虚空之遗 · 肉鸽",
    )
    .expect("切肉鸽")
    .expect("开着");
    let b = ensure_for_slot(
        &data,
        &banks,
        &isolation_key(&library, "lotv", Some("残卷")),
        "虚空之遗 · 残卷",
    )
    .expect("切残卷")
    .expect("开着");
    assert_ne!(a, b, "两部不同改版必须是两个组：{a} / {b}");
    assert!(a.contains("肉鸽"), "{a}");
    assert!(b.contains("残卷"), "{b}");
}

#[test]
fn vanilla_slot_reuses_the_legacy_original_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    // 只有一个 lotv 槽位、正用着原版
    let _library = library_with_lotv_mod_active(&data, None);
    // 当年开隔离时自动收下的老组：有内容、没有任何绑定
    write(&root(&data).join("原版-1"), "a.SC2Bank", "原版进度");
    set_isolation(&data, &banks, true).expect("开隔离");

    let name = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("切原版")
        .expect("开着");
    assert_eq!(name, "原版-1", "要复用老组，别新建空组");
    assert_eq!(
        profile_for(&data, "lotv").expect("查"),
        Some("原版-1".to_string()),
        "复用的同时要把它绑到槽位键上"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join("原版-1").join("a.SC2Bank")).expect("读"),
        "原版进度"
    );
    // 切组时 restore() 会留一份「还原前」的自动备份，所以这里只数组
    assert_eq!(
        backups(&data)
            .expect("列")
            .iter()
            .filter(|item| !item.auto)
            .count(),
        1,
        "不该多出一个组"
    );

    // 再走一次：还是同一个组（幂等）
    let again = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("再切")
        .expect("开着");
    assert_eq!(again, "原版-1");
    assert_eq!(
        backups(&data)
            .expect("列")
            .iter()
            .filter(|item| !item.auto)
            .count(),
        1
    );
}

#[test]
fn vanilla_slot_without_legacy_creates_a_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    let _library = library_with_lotv_mod_active(&data, None);
    set_isolation(&data, &banks, true).expect("开隔离");

    let name = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("切原版")
        .expect("开着");
    assert!(name.starts_with("虚空之遗"), "{name}");
    assert_eq!(profile_for(&data, "lotv").expect("查"), Some(name));
}
#[test]
fn another_campaign_never_reuses_a_group_that_is_already_bound() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    write(&banks, "原版进度.SC2Bank", "玩到第三章");

    // 两个官方槽位都正用着原版
    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    for slot in ["wol", "lotv"] {
        index
            .slots
            .insert(slot.to_string(), crate::library::CampaignSlot::default());
    }
    library.save_index(&index).expect("写索引");

    // 开隔离：当前这份被收成「原版」组，第一个启用的战役复用它
    set_isolation(&data, &banks, true).expect("开隔离");
    let lotv = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("切虚空之遗")
        .expect("开着");
    assert_eq!(profile_for(&data, "lotv").expect("查"), Some(lotv.clone()));

    // 第二个官方战役（也是原版）**不能**把这一组抢过去：那会把两部的进度并成一份
    let wol = ensure_for_slot(&data, &banks, "wol", "自由之翼")
        .expect("切自由之翼")
        .expect("开着");
    assert_ne!(wol, lotv, "已经归属虚空之遗的组不能被自由之翼复用");
    assert_eq!(
        profile_for(&data, "lotv").expect("查"),
        Some(lotv.clone()),
        "虚空之遗的绑定不许被抢走"
    );
    assert_eq!(profile_for(&data, "wol").expect("查"), Some(wol.clone()));
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&lotv).join("原版进度.SC2Bank")).expect("读"),
        "玩到第三章",
        "虚空之遗那份要留在它自己的组里"
    );
    assert!(
        snapshot(&root(&data).join(&wol))
            .expect("快照")
            .files
            .is_empty(),
        "自由之翼原版没有存档，别从前一组抄过来"
    );
}

#[test]
fn reopening_isolation_never_writes_untracked_saves_into_the_old_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    write(&banks, "进度.SC2Bank", "虚空进度");

    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    index
        .slots
        .insert("lotv".to_string(), crate::library::CampaignSlot::default());
    library.save_index(&index).expect("写索引");

    set_isolation(&data, &banks, true).expect("开隔离");
    let lotv = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("切")
        .expect("开着");

    // 关掉隔离之后，用户直接进游戏玩了别的战役 —— 启动器不知道那是谁的进度
    let off = set_isolation(&data, &banks, false).expect("关隔离");
    assert!(off.active.is_none(), "关掉之后 active 不能再指着某个组");
    write(&banks, "进度.SC2Bank", "关着时玩的进度");

    // 重新打开：当前这份要当成「原版」收进**新**组，不能算到虚空之遗头上
    let on = set_isolation(&data, &banks, true).expect("再开");
    let reopened = on.active.clone().expect("要收下当前这份");
    assert_ne!(reopened, lotv, "重新打开不能沿用上次的组");
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&reopened).join("进度.SC2Bank")).expect("读"),
        "关着时玩的进度"
    );

    // 再启用虚空之遗：它自己的组原样回去，关着时那份留在新组
    ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
        .expect("再切")
        .expect("开着");
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&lotv).join("进度.SC2Bank")).expect("读"),
        "虚空进度",
        "虚空之遗的进度不许被关着时那份覆盖"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join(&reopened).join("进度.SC2Bank")).expect("读"),
        "关着时玩的进度"
    );
    assert_eq!(
        std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
        "虚空进度"
    );
}

#[test]
fn switching_never_wipes_a_group_when_the_banks_folder_is_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // Banks 目录整个不在（换了机器 / 文档目录没同步）
    let banks = tmp.path().join("Banks");
    write(&root(&data).join("甲"), "进度.SC2Bank", "上一份");
    std::fs::create_dir_all(root(&data).join("乙")).expect("建乙");
    let mut state = Isolation {
        active: Some("甲".to_string()),
        ..Default::default()
    };
    state
        .assignments
        .insert("甲".to_string(), "lotv".to_string());
    write_isolation(&data, &state).expect("写状态");

    let err = switch(&data, &banks, "乙").expect_err("Banks 不存在时要报错，不能当成「空」");
    assert!(format!("{err}").contains("Banks"), "{err}");
    assert_eq!(
        std::fs::read_to_string(root(&data).join("甲").join("进度.SC2Bank")).expect("读"),
        "上一份",
        "组里那份不能因为 Banks 不见了就被清空"
    );
    assert_eq!(
        isolation(&data).expect("读").active.as_deref(),
        Some("甲"),
        "没切成功就不许改 active"
    );

    // 同一个底线：直接「存进这一组」也不许清空
    assert!(save_into(&data, &banks, "甲").is_err());
    assert_eq!(
        std::fs::read_to_string(root(&data).join("甲").join("进度.SC2Bank")).expect("读"),
        "上一份"
    );
}

#[test]
fn the_bare_custom_key_never_creates_a_group_or_clears_banks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let banks = tmp.path().join("Banks");
    let _library = library_with_custom(&data, &[("甲", "战役甲", None)]);
    write(&banks, "自制进度.SC2Bank", "玩到一半");
    set_isolation(&data, &banks, true).expect("开隔离");

    // 自制战役没有「原版」：光秃秃的 custom 是老数据，不能自动建组 / 切过去
    assert_eq!(
        ensure_for_slot(&data, &banks, "custom", "自制战役").expect("调"),
        None,
        "自制战役没有「原版」组"
    );
    assert_eq!(
        profile_for(&data, "custom").expect("查"),
        None,
        "不许绑这个老键"
    );
    assert_eq!(
        std::fs::read_to_string(banks.join("自制进度.SC2Bank")).expect("读"),
        "玩到一半",
        "Banks 一个字节都不该动"
    );
    let groups = backups(&data)
        .expect("列")
        .into_iter()
        .filter(|item| !item.auto)
        .count();
    assert_eq!(groups, 1, "只该有开隔离时那份「原版」");
}
