//! 隔离键、归属候选与绑定修复的测试。

use super::fixture::{
    library_with_custom, library_with_lotv_mod, library_with_lotv_mod_active, write,
};
use crate::library::Library;
use crate::saves::groups::write_isolation;
use crate::saves::*;

/// 造一份隔离状态：`active` 指着 `name` 那个组，那个组绑在 `key` 上。
fn state_with(active: Option<&str>, name: &str, key: &str) -> Isolation {
    let mut state = Isolation {
        active: active.map(str::to_string),
        ..Default::default()
    };
    state.assignments.insert(name.to_string(), key.to_string());
    state
}

#[test]
fn duplicate_names_are_disambiguated_in_targets() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // 两个不同注册 ID（= 两个组）、名字和版本都一样的变体
    let library = library_with_custom(
        &data,
        &[("甲", "一样的名字", None), ("乙", "一样的名字", None)],
    );
    let targets = targets(&library, &[]);
    let labels: Vec<String> = targets
        .iter()
        .filter(|item| item.key.starts_with("custom/"))
        .map(|item| item.label.clone())
        .collect();
    assert_eq!(labels.len(), 2, "两个组各一条：{labels:?}");
    assert_ne!(labels[0], labels[1], "重名必须分得开：{labels:?}");
}

#[test]
fn vanilla_uses_the_slot_key_and_each_variant_gets_its_own() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let library = library_with_custom(&tmp.path().join("data"), &[]);

    // 原版：键就是槽位（老数据 `lotv` / `wol` 语义不变，不用迁移）
    assert_eq!(isolation_key(&library, "wol", None), "wol");
    assert_eq!(isolation_key(&library, "lotv", None), "lotv");
    // 有版本（这里库是空的，退回版本目录名）：槽位/身份
    assert_eq!(isolation_key(&library, "lotv", Some("肉鸽")), "lotv/肉鸽");
}

#[test]
fn legacy_custom_assignment_is_kept_but_never_auto_migrated() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_custom(&data, &[("甲", "战役甲", None)]);

    // 老数据：组存在，归属是光秃秃的 `custom`
    let old = root(&data).join("旧的自制战役-1");
    std::fs::create_dir_all(&old).expect("建组");
    std::fs::write(old.join("旧.SC2Bank"), "老进度").expect("写");
    let mut state = Isolation::default();
    state
        .assignments
        .insert("旧的自制战役-1".to_string(), "custom".to_string());
    write_isolation(&data, &state).expect("写状态");

    // 新键不会匹配到它 —— 不猜版本
    assert_eq!(
        profile_for(&data, &isolation_key(&library, "custom", Some("甲"))).expect("查"),
        None,
        "绝不自动把老组迁到某个自制版本"
    );
    // 组本身还在（不丢）
    assert!(old.is_dir());
    assert_eq!(
        std::fs::read_to_string(old.join("旧.SC2Bank")).expect("读"),
        "老进度"
    );

    // 候选里不会冒出这个老键（界面会单独标成「旧的自制战役（未指定版本）」）
    assert!(
        !targets(&library, &[])
            .iter()
            .any(|item| item.key == "custom")
    );

    // 用户手动指过去：一次 assign 就归到新键上
    assign(&data, "旧的自制战役-1", Some("custom/甲")).expect("手改");
    assert_eq!(
        profile_for(&data, "custom/甲").expect("查"),
        Some("旧的自制战役-1".to_string())
    );
}

#[test]
fn targets_list_official_vanilla_then_each_version_then_custom() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_custom(
        &data,
        &[
            ("v1", "战役 1.0", Some("Team.Campaign")),
            ("v2", "战役 2.0", Some("team.campaign")),
            ("other", "另一部", None),
        ],
    );

    let list = targets(&library, &[]);
    let keys: Vec<&str> = list.iter().map(|item| item.key.as_str()).collect();

    // 官方每个槽位先「原版」（键 = 槽位），没有改版时只有这一条
    assert_eq!(
        &keys[..4],
        &["wol", "hots", "lotv", "nova"],
        "官方按固定菜单顺序，每个槽位先原版"
    );
    assert_eq!(list[0].label, "自由之翼（原版）");
    // 自制：同一部（共享注册 ID）只列一条，另一部单独一条
    assert_eq!(keys[4], "custom/team.campaign", "同一部只列一条");
    assert_eq!(keys[5], "custom/other");
    assert_eq!(list[4].label, "自制战役 · 战役 1.0");
}

#[test]
fn targets_include_official_variants_after_the_vanilla_entry() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
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
                    id: "另一个改版".to_string(),
                    name: "另一个改版".to_string(),
                    ..Default::default()
                },
            ],
            active: None,
        },
    );
    library.save_index(&index).expect("写索引");

    let list = targets(&library, &[]);
    let keys: Vec<&str> = list.iter().map(|item| item.key.as_str()).collect();
    // 官方顺序：wol（原版）、hots（原版）、lotv（原版 + 两个改版）、nova（原版）
    assert_eq!(
        &keys[..6],
        &[
            "wol",
            "hots",
            "lotv",
            "lotv/肉鸽",
            "lotv/另一个改版",
            "nova"
        ],
        "每个槽位先原版、再跟上它的版本：{keys:?}"
    );
    assert_eq!(list[2].label, "虚空之遗（原版）");
    assert_eq!(list[3].label, "虚空之遗 · 肉鸽");
}

#[test]
fn custom_target_label_prefers_the_active_version_then_the_newest() {
    // label 不能随遍历顺序漂：启用的那版优先，都没启用就取导入时间最新的
    fn variant(id: &str, name: &str, imported_at: u64) -> crate::library::Variant {
        crate::library::Variant {
            id: id.to_string(),
            name: name.to_string(),
            registration_id: Some("team.campaign".to_string()),
            imported_at,
            ..Default::default()
        }
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    index.slots.insert(
        "custom".to_string(),
        crate::library::CampaignSlot {
            variants: vec![
                variant("new", "战役 2.0", 20),
                variant("old", "战役 1.0", 10),
            ],
            active: None,
        },
    );
    library.save_index(&index).expect("写索引");

    // 都没启用 -> 取最新的那版
    let list = targets(&library, &[]);
    assert_eq!(list[4].label, "自制战役 · 战役 2.0");
    assert_eq!(list.len(), 5, "同一部只列一条");

    // 启用了旧那版 -> 取启用的那版（哪怕它不是最新）
    let mut index = library.index();
    index.slots.get_mut("custom").expect("槽位").active = Some("old".to_string());
    library.save_index(&index).expect("写索引");
    let list = targets(&library, &[]);
    assert_eq!(list[4].label, "自制战役 · 战役 1.0");
}

#[test]
fn reconcile_rebinds_a_campaign_level_group_to_the_active_version() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // 槽位当前启用的是改版「肉鸽」，但老数据把组绑在战役级键 `lotv` 上
    let library = library_with_lotv_mod_active(&data, Some("肉鸽"));
    let group = root(&data).join("虚空之遗-1");
    std::fs::create_dir_all(&group).expect("建组");
    std::fs::write(group.join("a.SC2Bank"), "进度").expect("写");
    let state = state_with(Some("虚空之遗-1"), "虚空之遗-1", "lotv");
    write_isolation(&data, &state).expect("写状态");

    assert!(
        reconcile_active_binding(&library, &data).expect("迁移"),
        "该重绑到版本级键"
    );
    let state = isolation(&data).expect("读");
    assert_eq!(
        state.assignments.get("虚空之遗-1").map(String::as_str),
        Some("lotv/肉鸽")
    );
    assert_eq!(state.active.as_deref(), Some("虚空之遗-1"), "active 不动");
    // 组名和文件都不动
    assert!(group.is_dir());
    assert_eq!(
        std::fs::read_to_string(group.join("a.SC2Bank")).expect("读"),
        "进度"
    );
}

#[test]
fn reconcile_leaves_vanilla_alone() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // 槽位正用着原版：战役级键 `lotv` 本来就是对的
    let library = library_with_lotv_mod(&data);
    let state = state_with(Some("虚空之遗-1"), "虚空之遗-1", "lotv");
    write_isolation(&data, &state).expect("写状态");

    assert!(
        !reconcile_active_binding(&library, &data).expect("迁移"),
        "原版不该动"
    );
    let state = isolation(&data).expect("读");
    assert_eq!(
        state.assignments.get("虚空之遗-1").map(String::as_str),
        Some("lotv")
    );
}

#[test]
fn reconcile_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_lotv_mod_active(&data, Some("肉鸽"));

    // 已经绑在版本级键上：不动
    let state = state_with(Some("甲"), "甲", "lotv/肉鸽");
    write_isolation(&data, &state).expect("写状态");
    assert!(!reconcile_active_binding(&library, &data).expect("迁移"));

    // 没有 active / 没有绑定：也不动（老组交给 legacy 标签处理）
    write_isolation(&data, &Isolation::default()).expect("写状态");
    assert!(!reconcile_active_binding(&library, &data).expect("迁移"));

    // 真的重绑一次之后再跑：什么都不做
    let state = state_with(Some("乙"), "乙", "lotv");
    write_isolation(&data, &state).expect("写状态");
    assert!(reconcile_active_binding(&library, &data).expect("迁移"));
    assert!(
        !reconcile_active_binding(&library, &data).expect("迁移"),
        "第二次必须无动作"
    );
}

#[test]
fn reconcile_binds_an_unbound_active_group_to_the_only_vanilla_slot() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_lotv_mod_active(&data, None);
    // active 指着当年收下的老组，它没有任何绑定
    write(&root(&data).join("原版-1"), "a.SC2Bank", "原版进度");
    write_isolation(
        &data,
        &Isolation {
            active: Some("原版-1".to_string()),
            ..Default::default()
        },
    )
    .expect("写状态");

    assert!(
        reconcile_active_binding(&library, &data).expect("迁移"),
        "唯一一个原版槽位 + 组没绑定 -> 该绑上"
    );
    assert_eq!(
        profile_for(&data, "lotv").expect("查"),
        Some("原版-1".to_string())
    );
    assert!(
        !reconcile_active_binding(&library, &data).expect("再迁移"),
        "第二次必须无动作"
    );
}

#[test]
fn reconcile_does_not_guess_when_several_slots_are_vanilla() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // 两个官方槽位都正用着原版：挑哪个都是猜 -> 不动
    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    for slot in ["wol", "lotv"] {
        index
            .slots
            .insert(slot.to_string(), crate::library::CampaignSlot::default());
    }
    library.save_index(&index).expect("写索引");

    write(&root(&data).join("原版-1"), "a.SC2Bank", "原版进度");
    write_isolation(
        &data,
        &Isolation {
            active: Some("原版-1".to_string()),
            ..Default::default()
        },
    )
    .expect("写状态");

    assert!(!reconcile_active_binding(&library, &data).expect("迁移"));
    assert_eq!(profile_for(&data, "wol").expect("查"), None);
    assert_eq!(profile_for(&data, "lotv").expect("查"), None);
}

#[test]
fn two_installed_custom_campaigns_are_both_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_custom(&data, &[("甲", "战役甲", None), ("乙", "战役乙", None)]);
    let installed = vec![
        ("custom".to_string(), "甲".to_string()),
        ("custom".to_string(), "乙".to_string()),
    ];

    let list = targets(&library, &installed);
    let custom: Vec<&SaveTarget> = list
        .iter()
        .filter(|item| item.key.starts_with("custom/"))
        .collect();
    assert_eq!(custom.len(), 2);
    assert!(
        custom.iter().all(|item| item.enabled),
        "两部自制都装着 -> 两个 true"
    );
}

#[test]
fn only_the_installed_custom_campaign_is_enabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_custom(&data, &[("甲", "战役甲", None), ("乙", "战役乙", None)]);
    let installed = vec![("custom".to_string(), "甲".to_string())];

    let list = targets(&library, &installed);
    let enabled: Vec<&str> = list
        .iter()
        .filter(|item| item.key.starts_with("custom/") && item.enabled)
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(enabled, vec!["自制战役 · 战役甲"]);
}

#[test]
fn installed_official_variant_disables_the_vanilla_target() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_lotv_mod_active(&data, None);
    let key = isolation_key(&library, "lotv", Some("肉鸽"));

    let list = targets(&library, &[("lotv".to_string(), "肉鸽".to_string())]);
    assert!(
        list.iter()
            .find(|item| item.key == key)
            .expect("有改版")
            .enabled,
        "装着的改版要算启用"
    );
    assert!(
        !list
            .iter()
            .find(|item| item.key == "lotv")
            .expect("有原版")
            .enabled,
        "装了改版，原版就不算装着"
    );
}

#[test]
fn uninstalled_targets_are_disabled() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    let library = library_with_lotv_mod_active(&data, None);

    // 什么都没装：改版全 false，原版全 true
    let list = targets(&library, &[]);
    assert!(
        list.iter()
            .filter(|item| item.key.contains('/'))
            .all(|item| !item.enabled)
    );
    assert!(
        list.iter()
            .filter(|item| !item.key.contains('/'))
            .all(|item| item.enabled)
    );
}

#[test]
fn reconcile_never_rebinds_a_legacy_custom_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");

    // 自制槽位当前选中「甲」，而「甲」已经有一组自己的存档（新键）
    let library = Library::new(&data);
    let mut index = crate::library::LibraryIndex::default();
    index.slots.insert(
        "custom".to_string(),
        crate::library::CampaignSlot {
            variants: vec![crate::library::Variant {
                id: "甲".to_string(),
                name: "战役甲".to_string(),
                ..Default::default()
            }],
            active: Some("甲".to_string()),
        },
    );
    library.save_index(&index).expect("写索引");

    // 老数据：active 指着当年收下的老组，绑定是光秃秃的 custom
    write(&root(&data).join("老的自制组-1"), "老.SC2Bank", "老进度");
    write(&root(&data).join("战役甲-1"), "甲的.SC2Bank", "甲的进度");
    let mut state = state_with(Some("老的自制组-1"), "老的自制组-1", "custom");
    state
        .assignments
        .insert("战役甲-1".to_string(), "custom/甲".to_string());
    write_isolation(&data, &state).expect("写状态");

    assert!(
        !reconcile_active_binding(&library, &data).expect("迁移"),
        "老的自制键说明不了是哪一部，不许自动迁移"
    );
    let state = isolation(&data).expect("读");
    assert_eq!(
        state.assignments.get("老的自制组-1").map(String::as_str),
        Some("custom")
    );
    assert_eq!(
        state.assignments.get("战役甲-1").map(String::as_str),
        Some("custom/甲"),
        "「甲」自己的组不许被抢走"
    );
    assert_eq!(
        std::fs::read_to_string(root(&data).join("战役甲-1").join("甲的.SC2Bank")).expect("读"),
        "甲的进度"
    );
}

#[test]
fn reconcile_never_steals_a_variant_key_from_another_group() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let data = tmp.path().join("data");
    // 槽位当前启用「肉鸽」，而「肉鸽」已经有自己的组
    let library = library_with_lotv_mod_active(&data, Some("肉鸽"));
    write(
        &root(&data).join("虚空之遗-1"),
        "vanilla.SC2Bank",
        "原版进度",
    );
    write(
        &root(&data).join("虚空之遗 · 肉鸽-1"),
        "rogue.SC2Bank",
        "肉鸽进度",
    );

    // active 还是那个绑在战役级键上的老组（比如上次切组失败 / 崩溃留下的状态）
    let mut state = state_with(Some("虚空之遗-1"), "虚空之遗-1", "lotv");
    state
        .assignments
        .insert("虚空之遗 · 肉鸽-1".to_string(), "lotv/肉鸽".to_string());
    write_isolation(&data, &state).expect("写状态");

    assert!(
        !reconcile_active_binding(&library, &data).expect("迁移"),
        "版本键已经有组认领，硬绑过来只是把两份进度换了标签"
    );
    let state = isolation(&data).expect("读");
    assert_eq!(
        state.assignments.get("虚空之遗-1").map(String::as_str),
        Some("lotv")
    );
    assert_eq!(
        state
            .assignments
            .get("虚空之遗 · 肉鸽-1")
            .map(String::as_str),
        Some("lotv/肉鸽"),
        "原来那一组的归属不许被抢"
    );
}
