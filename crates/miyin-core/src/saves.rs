//! 存档管理：`Documents\StarCraft II\Banks` 的备份与还原（issue #20）。
//!
//! **存档是用户数据**，不是游戏本体 —— 所以它不走游戏目录那套写盘闸门
//! （那个闸门只管安装目录里的文件）。但同样守两条：**动之前先备份**、
//! **先给人看**（[`SaveSet`] 就是给人看的那一份）。
//!
//! 「存档隔离」现在给到的是最有用的一半：**每个包可以备份 / 还原整套 Banks**，
//! 想换战役组合时不会把上一部的进度冲掉。真正的按版本自动隔离要等游戏运行时探测。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;

/// 备份目录名里的时间戳：`20261009-181500`。
fn stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 不引时间库：只要单调、可读、能排序就够了
    format!("{now}")
}

/// 一份存档快照：有哪些文件、多大、最后改动是什么时候。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SaveSet {
    pub files: Vec<SaveFile>,
    pub bytes: u64,
    /// 目录不存在（还没产生过存档）时为 true —— 界面要按「可能缺失」处理
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveFile {
    pub name: String,
    pub bytes: u64,
}

/// 一份已备份的存档。
#[derive(Debug, Clone, Serialize)]
pub struct BackupEntry {
    /// 备份目录名，还原时拿它指认
    pub name: String,
    /// 备份时带的标签（战役 / 用户自己写的备注）
    pub label: String,
    pub bytes: u64,
    pub files: usize,
}

/// 列一份 Banks 里有什么。目录不存在不算错误（第一次玩之前它就是不存在的）。
pub fn snapshot(banks: &Path) -> Result<SaveSet> {
    if !banks.is_dir() {
        return Ok(SaveSet {
            missing: true,
            ..SaveSet::default()
        });
    }

    let mut files = Vec::new();
    let mut bytes = 0;
    for entry in std::fs::read_dir(banks)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }
        bytes += meta.len();
        files.push(SaveFile {
            name: entry.file_name().to_string_lossy().into_owned(),
            bytes: meta.len(),
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(SaveSet {
        files,
        bytes,
        missing: false,
    })
}

/// 备份根目录：`<启动器目录>/data/saves`。
pub fn root(library_root: &Path) -> PathBuf {
    library_root.join("saves")
}

/// 备份里的目录名规则。`label` 由调用方给（战役名、或用户写的备注）。
fn dir_name(label: &str) -> String {
    let clean: String = label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let clean = clean.trim_matches('_').to_string();
    let clean = if clean.is_empty() {
        "手动".to_string()
    } else {
        clean
    };
    // 截断，免得标签太长把路径撑爆
    let short: String = clean.chars().take(40).collect();
    format!("{}-{}", short, stamp())
}

/// 把现在的 Banks 备份一份。返回备份目录名。
pub fn backup(banks: &Path, library_root: &Path, label: &str) -> Result<String> {
    let set = snapshot(banks)?;
    if set.missing || set.files.is_empty() {
        // 没有存档就别建空目录 —— 还原一个空备份只会让人以为"存档回来了"
        return Err(crate::error::Error::PackageRejected(
            "还没有存档可以备份".to_string(),
        ));
    }

    let name = dir_name(label);
    let target = root(library_root).join(&name);
    std::fs::create_dir_all(&target)?;
    for file in &set.files {
        std::fs::copy(banks.join(&file.name), target.join(&file.name))?;
    }
    Ok(name)
}

/// 列已经备份了哪些。
pub fn backups(library_root: &Path) -> Result<Vec<BackupEntry>> {
    let root = root(library_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let set = snapshot(&entry.path())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // 目录名是 `<标签>-<时间戳>`，标签可能自带连字符，从右边切
        let label = name
            .rsplit_once('-')
            .map(|(left, _)| left.to_string())
            .unwrap_or_else(|| name.clone());
        out.push(BackupEntry {
            name,
            label,
            bytes: set.bytes,
            files: set.files.len(),
        });
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// 还原一份备份。
///
/// **还原之前先把现在的存档备份一份**（`还原前-<时间戳>`）—— 用户点错了还能退回来。
/// 返回那份安全备份的目录名。
pub fn restore(banks: &Path, library_root: &Path, name: &str) -> Result<String> {
    let source = root(library_root).join(name);
    if !source.is_dir() {
        return Err(crate::error::Error::PackageRejected(format!(
            "找不到存档备份：{name}"
        )));
    }

    // 先给现在这份留个后路。没有存档（第一次玩）就跳过。
    let safety = backup(banks, library_root, "还原前").ok();

    std::fs::create_dir_all(banks)?;
    // 清掉现有存档再铺 —— 否则会剩下备份里没有的旧文件，成了两份混在一起
    for entry in std::fs::read_dir(banks)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }
    for entry in std::fs::read_dir(&source)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), banks.join(entry.file_name()))?;
        }
    }

    Ok(safety.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &[u8]) {
        std::fs::create_dir_all(dir).expect("建目录");
        std::fs::write(dir.join(name), body).expect("写");
    }

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

        write(&banks, "ENS.SC2Bank", b"first");
        let name = backup(&banks, &library, "复刻战役").expect("备份");
        assert!(name.starts_with("复刻战役-"), "目录名要带标签：{name}");

        // 玩到一半，存档变了
        write(&banks, "ENS.SC2Bank", b"second");
        write(&banks, "新文件.SC2Bank", b"junk");

        let safety = restore(&banks, &library, &name).expect("还原");
        assert_eq!(
            std::fs::read(banks.join("ENS.SC2Bank")).expect("读"),
            b"first",
            "内容要回到备份那一刻"
        );
        assert!(
            !banks.join("新文件.SC2Bank").exists(),
            "备份里没有的文件要被清掉，不然两份混在一起"
        );
        assert!(safety.starts_with("还原前-"), "还原前要留后路：{safety}");

        // 后路里存的是「还原之前」那份
        let rescued = root(&library).join(&safety).join("ENS.SC2Bank");
        assert_eq!(std::fs::read(rescued).expect("读"), b"second");
    }

    #[test]
    fn listing_shows_labels() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "a.SC2Bank", b"x");
        backup(&banks, &library, "第一部").expect("备份");

        let list = backups(&library).expect("列备份");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label, "第一部");
        assert_eq!(list[0].files, 1);
    }
}
