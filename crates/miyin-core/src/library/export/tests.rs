//! 导出模块的单元测试（端到端用例在 `library/tests/export.rs`）。

use super::miyin::{ccm_metadata, sub_to_campaign};
use super::scnexus::safe_target;
use super::unique_name;
use crate::library::Variant;

#[test]
fn ccm_metadata_is_key_value_and_single_line() {
    let variant = Variant {
        id: "a".to_string(),
        name: "黄金之遗".to_string(),
        author: Some("HTXL".to_string()),
        version: Some("1.32".to_string()),
        description: Some("第一行\n第二行".to_string()),
        target_sub: Some("void".to_string()),
        registration_id: Some("HTXL.golden".to_string()),
        tags: vec!["重制".to_string()],
        ..Variant::default()
    };

    let text = ccm_metadata(&variant);
    assert!(text.contains("title=黄金之遗"));
    assert!(text.contains("author=HTXL"));
    assert!(text.contains("campaign=LOTV"));
    assert!(text.contains("id=HTXL.golden"));
    assert!(text.contains("tags=重制"));
    // 值里不能有换行，否则会破坏 键=值 结构
    assert!(text.contains("desc=第一行 第二行"));
}

#[test]
fn sub_directories_map_back_to_campaign_values() {
    assert_eq!(sub_to_campaign("void"), Some("LOTV"));
    assert_eq!(sub_to_campaign("swarm/evolution"), Some("HOTSEVO"));
    assert_eq!(sub_to_campaign("voidprologue"), Some("LOTVPROLOGUE"));
}

#[test]
fn duplicate_names_get_a_suffix() {
    let mut used = Vec::new();
    assert_eq!(unique_name(&mut used, "a.SC2Map"), "a.SC2Map");
    assert_eq!(unique_name(&mut used, "a.SC2Map"), "a (2).SC2Map");
    assert_eq!(unique_name(&mut used, "a.SC2Map"), "a (3).SC2Map");
    // 大小写不敏感：a.SC2Map / a (2) / a (3) 都已被占用，所以跳到 (4)
    assert_eq!(unique_name(&mut used, "A.sc2map"), "A (4).sc2map");
}

#[test]
fn unsafe_zip_paths_are_rejected() {
    assert_eq!(
        safe_target("Maps/Campaign/a.SC2Map").as_deref(),
        Some("Maps/Campaign/a.SC2Map")
    );
    assert_eq!(
        safe_target("Maps//Campaign\\a.SC2Map").as_deref(),
        Some("Maps/Campaign/a.SC2Map")
    );
    assert_eq!(safe_target("../outside.txt"), None);
    assert_eq!(safe_target("Maps/../../x"), None);
    assert_eq!(safe_target("C:/Windows/x"), None);
    assert_eq!(safe_target(""), None);
    assert_eq!(safe_target("./"), None);
}
