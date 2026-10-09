//! 开发者页的**版本历史**：把「这一版包含哪些文件」记下来，好回答"跟上一版差在哪"。
//!
//! 这**不是**完整的 VCS：内容本来就在游戏目录里，所以只记路径 + 大小 + 修改时间，
//! 不算内容哈希、不存副本。够用来做提交列表和两版之间的差异；不够用来还原内容
//! （那件事的底座是 SC2Diff，见 docs/developer-workflow.md §4.3）。
//!
//! 存在 `<data>/dev/<包名>/commits.json`，一个文件装完整条历史 —— 几百条提交也就几百 KB。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// 提交里的一条：一个文件在提交那一刻的样子。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// 给用户看的路径（一般是相对游戏目录的路径）
    pub path: String,
    pub bytes: u64,
    /// 修改时间（Unix 毫秒）。用它代替内容哈希 —— 够用且便宜。
    pub modified_ms: u64,
}

/// 一次提交。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// 序号，从 1 开始；列表里显示成 `#1`。
    pub id: u32,
    /// 用户给的版本号（`v8.1` 这种），可以没有。
    #[serde(default)]
    pub label: Option<String>,
    pub message: String,
    /// Unix 秒。
    pub at: u64,
    pub files: Vec<Snapshot>,
}

/// 两版之间的差异。
#[derive(Debug, Clone, Default, Serialize)]
pub struct Diff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// 大小或修改时间变了 —— 这一层看得出"动过"，看不出"动了什么"。
    pub changed: Vec<String>,
    pub unchanged: usize,
}

fn store_dir(root: &Path, pkg: &str) -> PathBuf {
    let safe: String = pkg
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    root.join("dev").join(safe)
}

fn store_file(root: &Path, pkg: &str) -> PathBuf {
    store_dir(root, pkg).join("commits.json")
}

/// 读整条历史。没有就是空的 —— 第一次提交之前它本来就不存在。
pub fn log(root: &Path, pkg: &str) -> Result<Vec<Commit>> {
    let file = store_file(root, pkg);
    if !file.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&file)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn stat(path: &Path) -> (u64, u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, 0);
    };
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    (meta.len(), modified_ms)
}

/// 提交一次：把当前这批文件记成一个版本。返回这次提交。
///
/// `files` 是 `(给用户看的路径, 绝对路径)`。空列表照样允许提交 ——
/// 有时候"这次什么都没加"本身就是要记的事。
pub fn commit(
    root: &Path,
    pkg: &str,
    message: &str,
    label: Option<&str>,
    files: &[(String, PathBuf)],
) -> Result<Commit> {
    let mut history = log(root, pkg)?;

    let mut snapshots: Vec<Snapshot> = files
        .iter()
        .map(|(key, path)| {
            let (bytes, modified_ms) = stat(path);
            Snapshot {
                path: key.clone(),
                bytes,
                modified_ms,
            }
        })
        .collect();
    snapshots.sort_by(|a, b| a.path.cmp(&b.path));

    let item = Commit {
        id: history.len() as u32 + 1,
        label: label
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty()),
        message: message.trim().to_string(),
        at: now_seconds(),
        files: snapshots,
    };
    history.push(item.clone());

    let dir = store_dir(root, pkg);
    std::fs::create_dir_all(&dir)?;
    // 原子写：崩在半路也不会留下半个 JSON 把整条历史读废
    let text = serde_json::to_string_pretty(&history)?;
    let tmp = dir.join("commits.json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, store_file(root, pkg))?;

    Ok(item)
}

/// 比两版。`from` 是旧的那次，`to` 是新的。
pub fn diff(from: &Commit, to: &Commit) -> Diff {
    use std::collections::BTreeMap;

    let old: BTreeMap<&str, &Snapshot> = from.files.iter().map(|f| (f.path.as_str(), f)).collect();
    let new: BTreeMap<&str, &Snapshot> = to.files.iter().map(|f| (f.path.as_str(), f)).collect();

    let mut out = Diff::default();
    for (path, item) in &new {
        match old.get(path) {
            None => out.added.push((*path).to_string()),
            Some(previous) => {
                if previous.bytes != item.bytes || previous.modified_ms != item.modified_ms {
                    out.changed.push((*path).to_string());
                } else {
                    out.unchanged += 1;
                }
            }
        }
    }
    for path in old.keys() {
        if !new.contains_key(path) {
            out.removed.push((*path).to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str, body: &[u8]) -> PathBuf {
        std::fs::create_dir_all(dir).expect("建目录");
        let path = dir.join(name);
        std::fs::write(&path, body).expect("写");
        path
    }

    #[test]
    fn commits_accumulate_and_diff_tells_what_moved() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("data");
        let files_dir = tmp.path().join("游戏目录");
        let pkg = "复刻战役";

        let a = touch(&files_dir, "a.SC2Map", "第一版".as_bytes());
        let b = touch(&files_dir, "b.SC2Mod", "模组".as_bytes());

        let first = commit(
            &root,
            pkg,
            "基线",
            Some("v8.0"),
            &[
                ("Maps/a.SC2Map".to_string(), a.clone()),
                ("Mods/b.SC2Mod".to_string(), b.clone()),
            ],
        )
        .expect("第一次提交");
        assert_eq!(first.id, 1);
        assert_eq!(first.label.as_deref(), Some("v8.0"));
        assert_eq!(first.files.len(), 2);

        // 改一个、删一个、加一个
        std::fs::write(&a, "第二版，长了一点").expect("改写");
        let c = touch(&files_dir, "c.txt", "新增".as_bytes());
        let second = commit(
            &root,
            pkg,
            "改了地图，去掉模组",
            Some("v8.1"),
            &[
                ("Maps/a.SC2Map".to_string(), a.clone()),
                ("说明/c.txt".to_string(), c),
            ],
        )
        .expect("第二次提交");
        assert_eq!(second.id, 2);

        let history = log(&root, pkg).expect("读历史");
        assert_eq!(history.len(), 2, "两次都记下来了");

        let delta = diff(&history[0], &history[1]);
        assert_eq!(delta.added, vec!["说明/c.txt".to_string()]);
        assert_eq!(delta.removed, vec!["Mods/b.SC2Mod".to_string()]);
        assert_eq!(delta.changed, vec!["Maps/a.SC2Map".to_string()]);
        assert_eq!(delta.unchanged, 0);
    }

    #[test]
    fn no_history_is_empty_not_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(log(tmp.path(), "没提交过").expect("读").is_empty());
    }
}
