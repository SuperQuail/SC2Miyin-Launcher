//! 导入冲突与版本比较。

use serde::{Deserialize, Serialize};

use crate::campaign::metadata::CampaignType;

use super::Library;

/// 由包内声明的资料片推断应该导入到哪个槽位。
///
/// 规则本体在 [`CampaignType::main_slot`]，这里只是给调用方一个好找的入口。
pub fn slot_for(kind: &CampaignType) -> Option<&'static str> {
    kind.main_slot()
}

/// 导入时与库里已有版本的冲突。
#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    /// 冲突的已有版本（库内目录名）。
    pub existing_id: String,
    pub existing_name: String,
    pub existing_version: Option<String>,
    pub incoming_version: Option<String>,
    /// 是否命中同一个**注册 ID**（比同名更强的信号）。
    pub same_id: bool,
    /// 新旧版本对比结论。
    pub relation: VersionRelation,
}

/// 新旧版本的对比结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionRelation {
    /// 新包更新。
    Newer,
    /// 版本号相同。
    Same,
    /// 新包更旧。
    Older,
    /// 版本号缺失或格式认不出来。
    Unknown,
}

impl VersionRelation {
    /// 面向用户的说法。
    pub fn label(self) -> &'static str {
        match self {
            Self::Newer => "更新的版本",
            Self::Same => "相同的版本",
            Self::Older => "更旧的版本",
            Self::Unknown => "无法比较版本",
        }
    }
}

/// 导入时遇到已有同名 / 同 ID 版本的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// **重命名后导入**：保留已有版本，新版本另起一个目录名，两者并存。
    #[default]
    Rename,
    /// **覆盖更新**：用新版本替换已有的同 ID（或同名）版本，目录名与已挂的补丁都不受影响。
    Overwrite,
}

/// 宽松版本比较：把两边的数字段抽出来按数值比。
///
/// 只认得"数字点分"这一大类写法（`1.4.2`、`v0.53`、`141版` 都行）；
/// 认不出来时返回 `None`，由调用方决定怎么提示，而不是瞎猜。
pub fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    fn numbers(text: &str) -> Vec<u64> {
        text.split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect()
    }

    let left_numbers = numbers(left);
    let right_numbers = numbers(right);
    if left_numbers.is_empty() || right_numbers.is_empty() {
        return None;
    }

    for (a, b) in left_numbers.iter().zip(right_numbers.iter()) {
        match a.cmp(b) {
            std::cmp::Ordering::Equal => continue,
            other => return Some(other),
        }
    }

    Some(left_numbers.len().cmp(&right_numbers.len()))
}

/// 检查往某个槽位导入时会不会与已有版本冲突。
///
/// 判定顺序：先按**注册 ID** 找，找不到再按**战役名**找 —— 前者是包作者声明的稳定标识，
/// 后者只是兜底（现实里大量包根本没有 ID）。
pub fn conflict_for(
    library: &Library,
    slot_slug: &str,
    incoming_id: Option<&str>,
    incoming_name: &str,
    incoming_version: Option<&str>,
) -> Option<Conflict> {
    let index = library.index();
    let slot = index.slots.get(slot_slug)?;

    let by_id = incoming_id.and_then(|incoming| {
        slot.variants.iter().find(|variant| {
            variant
                .registration_id
                .as_deref()
                .is_some_and(|existing| existing.eq_ignore_ascii_case(incoming))
        })
    });
    let existing = by_id.or_else(|| {
        slot.variants
            .iter()
            .find(|variant| variant.name.eq_ignore_ascii_case(incoming_name))
    })?;

    let relation = match (incoming_version, existing.version.as_deref()) {
        (Some(incoming), Some(existing)) => compare_versions(incoming, existing)
            .map(|ordering| match ordering {
                std::cmp::Ordering::Greater => VersionRelation::Newer,
                std::cmp::Ordering::Equal => VersionRelation::Same,
                std::cmp::Ordering::Less => VersionRelation::Older,
            })
            .unwrap_or(VersionRelation::Unknown),
        _ => VersionRelation::Unknown,
    };

    Some(Conflict {
        existing_id: existing.id.clone(),
        existing_name: existing.name.clone(),
        existing_version: existing.version.clone(),
        incoming_version: incoming_version.map(str::to_string),
        same_id: by_id.is_some(),
        relation,
    })
}
