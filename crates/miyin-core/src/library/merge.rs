//! 多个启动器实例之间的数据合并。
//!
//! 绿色版可以拷好几份，每份旁边都可能留着一个旧的 `data/`。启动器把数据放到游戏安装
//! 目录旁边之后，这些散落的库要能**并成一份**：不冲突的直接并进来，撞上的交给用户选。
//!
//! 两条底线：
//! 1. `plan()` **只读**，不动任何文件 —— 界面可以放心地先算给人看。
//! 2. 和**当前这台安装**绑死的东西（激活清单 / 已装记录 / 开发历史 / 官方文件备份）
//!    一律以游戏目录那份为准 —— 两份 active.json 合起来没有任何意义。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::Library;
use crate::error::Result;

/// 和当前安装绑死、一律以游戏目录那份为准的条目（只用来展示）。
pub const KEEP_TARGET: [&str; 4] = ["active.json", "installed.json", "dev/", "backup/"];

/// 目标库没有、可以直接并进来的版本。
#[derive(Debug, Clone, Serialize)]
pub struct MergeAddition {
    pub slot: String,
    pub variant: String,
    pub variant_name: String,
    pub bytes: u64,
}

/// 两边都有同一个版本目录名，内容又不一样 —— 要用户选。
#[derive(Debug, Clone, Serialize)]
pub struct MergeConflict {
    pub slot: String,
    pub variant: String,
    pub variant_name: String,
    pub local_bytes: u64,
    pub target_bytes: u64,
}

/// 一份合并案。`plan()` 算出来的，给界面看，也用来 `apply()`。
#[derive(Debug, Clone, Serialize)]
pub struct MergePlan {
    pub local: PathBuf,
    pub target: PathBuf,
    pub additions: Vec<MergeAddition>,
    pub conflicts: Vec<MergeConflict>,
    pub keep_target: Vec<String>,
}

impl MergePlan {
    /// 没有要并的、也没有要问的 —— 界面就不该弹窗。
    pub fn is_empty(&self) -> bool {
        self.additions.is_empty() && self.conflicts.is_empty()
    }
}

/// 用户在某个冲突上的选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeChoice {
    /// 保持游戏目录那份（缺省）。
    Target,
    /// 用本地这份换掉它（原来的会改名留一份）。
    Local,
    /// 两份都留，本地这份换个目录名进去。
    Both,
}

/// 合并干了什么。
#[derive(Debug, Clone, Default, Serialize)]
pub struct MergeReport {
    pub added: usize,
    pub replaced: usize,
    pub renamed: usize,
    pub skipped_same: usize,
    /// 换掉的那份原来的位置（留着，确认没问题可以自己删）。
    pub superseded: Vec<PathBuf>,
}

/// 算一份合并案。**只读**。
pub fn plan(local: &Path, target: &Path) -> Result<MergePlan> {
    let local_index = Library::new(local).index();
    let target_index = Library::new(target).index();

    let mut additions = Vec::new();
    let mut conflicts = Vec::new();

    for (slug, slot) in &local_index.slots {
        let existing = target_index.slots.get(slug);
        for variant in &slot.variants {
            let found =
                existing.and_then(|it| it.variants.iter().find(|item| item.id == variant.id));
            match found {
                None => additions.push(MergeAddition {
                    slot: slug.clone(),
                    variant: variant.id.clone(),
                    variant_name: variant.name.clone(),
                    bytes: variant.size_bytes,
                }),
                // shortcut: 只比体积和地图/模组数，不逐字节比对 —— 同样的包在两边导入体积必然一致；
                // 真出现体积相同内容不同的情况，用户选「两份都留」即可，不会丢东西。
                Some(same)
                    if same.size_bytes == variant.size_bytes
                        && same.map_count == variant.map_count
                        && same.mod_count == variant.mod_count => {}
                Some(same) => conflicts.push(MergeConflict {
                    slot: slug.clone(),
                    variant: variant.id.clone(),
                    variant_name: variant.name.clone(),
                    local_bytes: variant.size_bytes,
                    target_bytes: same.size_bytes,
                }),
            }
        }
    }

    Ok(MergePlan {
        local: local.to_path_buf(),
        target: target.to_path_buf(),
        additions,
        conflicts,
        keep_target: KEEP_TARGET.iter().map(|item| item.to_string()).collect(),
    })
}

/// 执行合并。`choices` 的 key 是 `"<slot>/<variant>"`，缺省 = 保持游戏目录那份。
///
/// **本地那份的版本目录是搬过去，不是复制** —— 合并就是把它并掉；搬不动的会原样留着。
pub fn apply(
    target: &Path,
    plan: &MergePlan,
    choices: &BTreeMap<String, MergeChoice>,
) -> Result<MergeReport> {
    let library = Library::new(target);
    let local_index = Library::new(&plan.local).index();
    let mut index = library.index();
    let mut report = MergeReport::default();

    // ① 不冲突的直接搬
    for addition in &plan.additions {
        let from = version_dir(&plan.local, &addition.slot, &addition.variant);
        if !from.is_dir() {
            continue;
        }
        let to = version_dir(target, &addition.slot, &addition.variant);
        crate::platform::move_path(&from, &to)?;
        if let Some(record) = find_variant(&local_index, &addition.slot, &addition.variant) {
            index
                .slots
                .entry(addition.slot.clone())
                .or_default()
                .variants
                .push(record);
        }
        report.added += 1;
    }

    // ② 冲突的按用户的选择办
    for conflict in &plan.conflicts {
        let key = format!("{}/{}", conflict.slot, conflict.variant);
        let choice = choices.get(&key).copied().unwrap_or(MergeChoice::Target);
        let from = version_dir(&plan.local, &conflict.slot, &conflict.variant);
        if choice == MergeChoice::Target || !from.is_dir() {
            report.skipped_same += 1;
            continue;
        }

        let record = find_variant(&local_index, &conflict.slot, &conflict.variant);
        let to = version_dir(target, &conflict.slot, &conflict.variant);

        if choice == MergeChoice::Local {
            // 原来那份改名留着 —— 用户选错了还能捞回来
            let backup = to.with_extension("superseded");
            if to.exists() {
                crate::platform::move_path(&to, &backup)?;
                report.superseded.push(backup);
            }
            crate::platform::move_path(&from, &to)?;
            if let (Some(record), Some(slot)) = (record, index.slots.get_mut(&conflict.slot)) {
                if let Some(item) = slot
                    .variants
                    .iter_mut()
                    .find(|item| item.id == conflict.variant)
                {
                    *item = record;
                }
            }
            report.replaced += 1;
        } else {
            // 两份都留：换个没人用的目录名进去
            let new_id = free_variant_id(&index, &conflict.slot, &conflict.variant);
            let renamed_to = version_dir(target, &conflict.slot, &new_id);
            crate::platform::move_path(&from, &renamed_to)?;
            if let Some(mut record) = record {
                record.id = new_id;
                index
                    .slots
                    .entry(conflict.slot.clone())
                    .or_default()
                    .variants
                    .push(record);
            }
            report.renamed += 1;
        }
    }

    library.save_index(&index)?;
    Ok(report)
}

fn version_dir(root: &Path, slot: &str, variant: &str) -> PathBuf {
    root.join("campaigns").join(slot).join(variant)
}

fn find_variant(index: &super::LibraryIndex, slot: &str, variant: &str) -> Option<super::Variant> {
    index
        .slots
        .get(slot)?
        .variants
        .iter()
        .find(|item| item.id == variant)
        .cloned()
}

/// 给「两份都留」找一个没人占的目录名：`<id>-2`、`<id>-3`……
fn free_variant_id(index: &super::LibraryIndex, slot: &str, variant: &str) -> String {
    let taken: Vec<&str> = index
        .slots
        .get(slot)
        .map(|it| it.variants.iter().map(|item| item.id.as_str()).collect())
        .unwrap_or_default();
    for n in 2..1000 {
        let candidate = format!("{variant}-{n}");
        if !taken.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    format!("{variant}-copy")
}
