//! 测试共享夹具。

use std::path::Path;

use crate::library::Library;

pub(super) fn write(dir: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(dir).expect("建目录");
    std::fs::write(dir.join(name), body).expect("写");
}

/// 造一个库，里面只有一个自制槽位、若干版本。
pub(super) fn library_with_custom(data: &Path, variants: &[(&str, &str, Option<&str>)]) -> Library {
    let library = Library::new(data);
    let mut index = crate::library::LibraryIndex::default();
    let slot = crate::library::CampaignSlot {
        variants: variants
            .iter()
            .map(|(id, name, registration)| crate::library::Variant {
                id: id.to_string(),
                name: name.to_string(),
                registration_id: registration.map(str::to_string),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    index.slots.insert("custom".to_string(), slot);
    library.save_index(&index).expect("写索引");
    library
}

/// 造一个只有 lotv 槽位（带一个「肉鸽」改版）的库；`active` 是这个槽位当前启用的版本。
pub(super) fn library_with_lotv_mod_active(data: &Path, active: Option<&str>) -> Library {
    let library = Library::new(data);
    let mut index = crate::library::LibraryIndex::default();
    index.slots.insert(
        "lotv".to_string(),
        crate::library::CampaignSlot {
            variants: vec![crate::library::Variant {
                id: "肉鸽".to_string(),
                name: "肉鸽".to_string(),
                ..Default::default()
            }],
            active: active.map(str::to_string),
        },
    );
    library.save_index(&index).expect("写索引");
    library
}

pub(super) fn library_with_lotv_mod(data: &Path) -> Library {
    library_with_lotv_mod_active(data, None)
}
