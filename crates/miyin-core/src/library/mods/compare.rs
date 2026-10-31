//! 两个模组版本之间的对比：按文件、按字节，以及语义 diff。

use std::path::Path;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::safety;

use super::StandaloneMod;
use super::index::{get, mods_root};

/// 两个模组版本之间的一个文件。
#[derive(Debug, Clone, Serialize)]
pub struct ModFileDiff {
    /// 相对模组目录的路径。
    pub path: String,
    /// `same` / `changed` / `added` / `removed`。
    pub status: String,
    pub size_before: u64,
    pub size_after: u64,
}

/// 两个模组版本的对比结果。
#[derive(Debug, Clone, Serialize)]
pub struct ModComparison {
    pub before: StandaloneMod,
    pub after: StandaloneMod,
    /// 内容完全一样（指纹相同）。
    pub identical: bool,
    /// 逐文件的差异。
    pub files: Vec<ModFileDiff>,
    /// SC2Diff 给出的**语义 diff**（按文件）。
    ///
    /// 只有装了 SC2Diff、而且那个文件是**单文件形态**的 `.SC2Mod` 才有 ——
    /// SC2Diff 处理的是文档，解开成目录树的它得先 pack 回去，这里不做。
    /// 拿不到就留空，界面显示「未做语义 diff」而不是假装没有差异。
    pub semantic: Vec<ModSemanticDiff>,
    /// 没跑语义 diff 的原因。
    pub semantic_note: Option<String>,
}

/// 一份文件的语义 diff。
#[derive(Debug, Clone, Serialize)]
pub struct ModSemanticDiff {
    pub path: String,
    pub text: String,
}

/// 列一个模组目录里的文件（相对路径 -> (是否文件, 大小)）。
fn list_files(root: &Path) -> Result<std::collections::BTreeMap<String, (bool, u64)>> {
    let mut map = std::collections::BTreeMap::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        let key = relative.to_string_lossy().replace('\\', "/");
        if key.is_empty() {
            continue;
        }
        let is_file = entry.file_type().is_file();
        let size = if is_file {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        } else {
            0
        };
        map.insert(key, (is_file, size));
    }

    Ok(map)
}

/// 比两个模组版本。
///
/// 用途有两个：让用户看清「这两个版本差在哪」，也是**将来只存 diff** 的前置
/// —— 看不见差异就谈不上只存差异。
pub fn compare(data: &Path, before_id: &str, after_id: &str) -> Result<ModComparison> {
    let before =
        get(data, before_id).ok_or_else(|| Error::CampaignNotFound(before_id.to_string()))?;
    let after = get(data, after_id).ok_or_else(|| Error::CampaignNotFound(after_id.to_string()))?;

    let root = mods_root(data);
    let dir_before = safety::ensure_within(&root, &root.join(&before.id))?;
    let dir_after = safety::ensure_within(&root, &root.join(&after.id))?;

    let files_before = list_files(&dir_before)?;
    let files_after = list_files(&dir_after)?;

    let mut files: Vec<ModFileDiff> = Vec::new();
    let mut semantic: Vec<ModSemanticDiff> = Vec::new();

    let mut keys: Vec<&String> = files_before.keys().chain(files_after.keys()).collect();
    keys.sort();
    keys.dedup();

    for key in keys {
        let left = files_before.get(key);
        let right = files_after.get(key);

        let status = match (left, right) {
            (Some(_), None) => "removed",
            (None, Some(_)) => "added",
            (Some((_, a)), Some((_, b))) => {
                if a == b && same_bytes(&dir_before.join(key), &dir_after.join(key)) {
                    "same"
                } else {
                    "changed"
                }
            }
            (None, None) => continue,
        };

        files.push(ModFileDiff {
            path: key.clone(),
            status: status.to_string(),
            size_before: left.map(|(_, size)| *size).unwrap_or(0),
            size_after: right.map(|(_, size)| *size).unwrap_or(0),
        });
    }

    let identical = !before.fingerprint.is_empty() && before.fingerprint == after.fingerprint;

    // 语义 diff：只在装了 SC2Diff、且改动的是**单文件** .SC2Mod 时才做
    let mut note: Option<String> = None;
    if crate::tools::sc2diff::locate(data).is_none() {
        note = Some("没装 SC2Diff，只列出了文件级差异".to_string());
    } else if identical {
        note = Some("两个版本内容完全一样，没什么好比的".to_string());
    } else {
        let tmp = data
            .join("diff-work")
            .join(crate::library::now_seconds().to_string());

        for item in files.iter().filter(|item| item.status == "changed") {
            let left = dir_before.join(&item.path);
            let right = dir_after.join(&item.path);
            // SC2Diff 吃的是**打包好的文档**；解开成目录树的跳过
            if !left.is_file() || !right.is_file() {
                continue;
            }
            if !item.path.to_ascii_lowercase().ends_with(".sc2mod")
                && !item.path.to_ascii_lowercase().ends_with(".sc2map")
            {
                continue;
            }

            let repo = tmp.join(format!("r{}", semantic.len()));
            match crate::tools::sc2diff::semantic_diff(data, &left, &right, &repo) {
                Ok(text) => semantic.push(ModSemanticDiff {
                    path: item.path.clone(),
                    text,
                }),
                Err(error) => {
                    note = Some(format!("语义 diff 没跑通：{error}"));
                    break;
                }
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);

        if note.is_none() && semantic.is_empty() {
            note = Some("改动都不在可比较的文档上，只列出了文件级差异".to_string());
        }
    }

    Ok(ModComparison {
        before,
        after,
        identical,
        files,
        semantic,
        semantic_note: note,
    })
}

/// 两个文件是不是逐字节一样。
fn same_bytes(left: &Path, right: &Path) -> bool {
    match (std::fs::read(left), std::fs::read(right)) {
        (Ok(a), Ok(b)) => a == b,
        // 读不了（比如是目录）就当作「不一样」，反正上面已经比过大小了
        _ => false,
    }
}

// ---------- 来自 React 重构那条分支：换版本时把补丁的绑定改指过去 ----------
