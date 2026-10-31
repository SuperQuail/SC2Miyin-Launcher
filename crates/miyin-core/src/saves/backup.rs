//! 备份、还原，以及存档组内部的文件搬运。
//!
//! 「存档集合」不是**一个**目录：`SavesRoots` 把所有
//! `Accounts/<账号>/<地域>/Banks` 合在一起看 —— 快照、备份、还原、切换都对全部根
//! 整体操作，组里保留账号 / 地域层级，切回时原样铺回去。

use std::path::{Path, PathBuf};

use super::groups::{assign, fresh_group, group_label, isolation, profile_for, targets};
use super::roots::{FileRef, SavesRoots};
use super::{AUTO_BACKUP, BackupEntry, ORIGINAL, SaveFile, SaveSet};
use crate::error::{Error, Result};
use crate::library::Library;
use crate::safety;

/// 备份目录名里的时间戳（Unix 秒）。
///
/// 同一秒内要建第二个组时**往后顺延一秒**，绝不重名：目录名撞车的后果是
/// 后一份内容被倒进前一份的目录里（旧文件还留着），两份进度混在一起 ——
/// 比直接报错糟得多（切换 / 重新打开隔离都可能连着建两组）。
pub(super) fn stamp() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};

    static LAST: AtomicU64 = AtomicU64::new(0);
    let mut last = LAST.load(Ordering::Relaxed);
    loop {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // 不引时间库：只要单调、可读、能排序就够了
        let next = now.max(last + 1);
        match LAST.compare_exchange_weak(last, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return format!("{next}"),
            Err(actual) => last = actual,
        }
    }
}

/// 一个目录树里的所有文件：相对它的路径（`/` 分隔）、绝对路径、字节数。
fn files_of(dir: &Path) -> Result<Vec<FileRef>> {
    let mut out: Vec<FileRef> = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|item| item.map_err(std::io::Error::from))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(dir)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        out.push((rel, entry.path().to_path_buf(), bytes));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// 把一组文件按相对路径铺到 `target` 下（需要时建目录）。
pub(super) fn copy_into(files: &[FileRef], target: &Path) -> Result<()> {
    for (rel, source, _) in files {
        let dest = target.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, &dest)?;
    }
    Ok(())
}

/// 清空一个目录里的东西（文件与随后的空目录），目录本身留着。
///
/// 不用 `remove_dir_all`：存档目录里可能有 junction / 符号链接，
/// 逐条删更可控（链接本身不动，也就碰不到它指向的地方）。
pub(super) fn clear(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            clear(&path)?;
            std::fs::remove_dir(&path)?;
        } else if kind.is_file() {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// 列一份 Banks 里有什么（`banks` 旁边有 `Accounts/` 就是全部真实根，见
/// [`SavesRoots::from_banks`]）。目录不存在不算错误（第一次玩之前它就是不存在的）。
pub fn snapshot(banks: &Path) -> Result<SaveSet> {
    snapshot_paths(&SavesRoots::from_banks(banks))
}

/// 列**所有真实存档根**里有什么。
///
/// `name` 是相对 `documents` 的路径（`Accounts/<账号>/<地域>/Banks/…`；
/// 单目录模式下就是相对那个目录的路径）—— 组里按它存，所以切回时账号 / 地域层级还在。
pub fn snapshot_paths(roots: &SavesRoots) -> Result<SaveSet> {
    let (missing, files) = roots.files()?;
    if missing {
        return Ok(SaveSet {
            missing: true,
            ..SaveSet::default()
        });
    }

    let mut set = SaveSet::default();
    for (name, _, bytes) in files {
        set.bytes += bytes;
        set.files.push(SaveFile { name, bytes });
    }
    Ok(set)
}

/// 备份根目录：`<启动器目录>/data/saves`。
pub fn root(library_root: &Path) -> PathBuf {
    library_root.join("saves")
}

/// 备份里的目录名规则。`label` 由调用方给（战役名、或用户写的备注）。
pub(super) fn dir_name(label: &str) -> String {
    let clean: String = label
        .chars()
        .map(|c| {
            // 保留间隔号：组名要能一眼分辨「自由之翼 · 重生 v1.5.0」
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '·' {
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
    format!("{short}-{}", stamp())
}

/// 把现在的 Banks 备份一份（老 API：`banks` 旁边有 `Accounts/` 就是全部真实根）。
/// 返回备份目录名。
pub fn backup(banks: &Path, library_root: &Path, label: &str) -> Result<String> {
    backup_paths(&SavesRoots::from_banks(banks), library_root, label)
}

/// 把**所有真实存档根**整体备份成一组（组里保留账号 / 地域层级）。返回组名。
pub fn backup_paths(roots: &SavesRoots, library_root: &Path, label: &str) -> Result<String> {
    let (missing, files) = roots.files()?;
    if missing || files.is_empty() {
        // 没有存档就别建空目录 —— 还原一个空备份只会让人以为"存档回来了"
        return Err(Error::PackageRejected("还没有存档可以备份".to_string()));
    }

    let name = dir_name(label);
    copy_into(&files, &root(library_root).join(&name))?;
    Ok(name)
}

/// 列已经备份了哪些。
pub fn backups(library_root: &Path) -> Result<Vec<BackupEntry>> {
    let root = root(library_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    // 迁移期：没有绑定的组标成 legacy（名字不代表内容，界面别叫它「原版」）
    let state = isolation(library_root)?;
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
            auto: label == AUTO_BACKUP,
            // 只有「开隔离时自动收下的那份」才叫未分类 —— 用户手动备份、只是没绑战役的组
            // 有自己的名字，别也给它们贴「升级前的存档」。
            legacy: label != AUTO_BACKUP
                && label == ORIGINAL
                && !state.assignments.contains_key(&name),
            name,
            label,
            bytes: set.bytes,
            files: set.files.len(),
        });
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// 还原一份备份（老 API，见 [`backup`]）。
///
/// **还原之前先把现在的存档备份一份**（`还原前-<时间戳>`）—— 用户点错了还能退回来。
/// 返回那份安全备份的目录名。
pub fn restore(banks: &Path, library_root: &Path, name: &str) -> Result<String> {
    restore_paths(&SavesRoots::from_banks(banks), library_root, name)
}

/// 把所有真实存档根一起换成某个组的内容（先把现在这份另存为安全备份）。
///
/// 先清空**每个**根再铺 —— 否则备份里没有的旧文件会留下来，两份混在一起。
pub fn restore_paths(roots: &SavesRoots, library_root: &Path, name: &str) -> Result<String> {
    let source = root(library_root).join(name);
    if !source.is_dir() {
        return Err(Error::PackageRejected(format!("找不到存档备份：{name}")));
    }

    // 先给现在这份留个后路。没有存档（第一次玩）就跳过。
    let safety = backup_paths(roots, library_root, AUTO_BACKUP).ok();

    // 每个根都建出来再清空：切到一个空组 = 这个战役从头开始，Banks 空着等游戏写
    for banks in roots.banks() {
        std::fs::create_dir_all(banks)?;
        clear(banks)?;
    }
    for (rel, source, _) in files_of(&source)? {
        let dest = roots.place(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, &dest)?;
    }

    Ok(safety.unwrap_or_default())
}

// ---------------------------------------------------------------------------
// 存档组里的**单个文件**（组详情：可以打开看、把某一份挪到别的组）
// ---------------------------------------------------------------------------

/// 一个组目录。组名只许**裸名字** —— 这条边界保证下面所有操作都只在
/// `<data>/saves/` 里面动，跳不出去。
fn group_dir(root: &Path, name: &str) -> Result<PathBuf> {
    let saves = self::root(root);
    safety::ensure_within(&saves, &saves.join(name))
}

/// 组内相对路径：可以带子目录（新格式的组按 `Accounts/<账号>/<地域>/Banks/…` 存），
/// 但不许绝对路径 / `..` / 反斜杠。返回带平台分隔符的路径。
fn checked_name(root: &Path, group: &str, file: &str) -> Result<PathBuf> {
    let bad = file.is_empty()
        || file.contains('\\')
        || file
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..");
    if bad {
        return Err(Error::PackageRejected(format!("存档文件名不合法：{file}")));
    }
    let saves = self::root(root);
    let path = group_dir(root, group)?.join(file.replace('/', std::path::MAIN_SEPARATOR_STR));
    safety::ensure_within(&saves, &path)
}

/// 列出一个存档组里的存档文件（相对路径 + 字节数），按名字排序。
///
/// 组不存在就说清楚是哪一组；空组给空数组（空组是正常状态，不是错误）。
pub fn group_files(root: &Path, name: &str) -> Result<Vec<SaveFile>> {
    let dir = group_dir(root, name)?;
    if !dir.is_dir() {
        return Err(Error::PackageRejected(format!("没有这个存档组：{name}")));
    }
    Ok(snapshot(&dir)?.files)
}

/// 把某个组里的**一份**存档挪到另一个组 —— 也就是改它归到哪个战役。
///
/// 三条底线（动的都是用户数据）：
/// - 目标组不存在就建；
/// - **同名不覆盖**，直接报错让用户自己决定（宁可报错，也不替他做决定）；
/// - 只动 `<data>/saves/` 里的文件，**绝不碰游戏目录的 Banks**（Banks 是另一套路径，
///   这个函数自始至终没有 banks 参数）；源组挪空了也不删组（用户可能还想切回来）。
pub fn move_save_file(root: &Path, from: &str, file: &str, to: &str) -> Result<()> {
    if from == to {
        return Err(Error::PackageRejected("源组和目标组是同一个".to_string()));
    }

    let source = checked_name(root, from, file)?;
    if !source.is_file() {
        return Err(Error::PackageRejected(format!("找不到这份存档：{file}")));
    }

    let target = checked_name(root, to, file)?;
    if target.exists() {
        return Err(Error::PackageRejected(format!(
            "「{to}」里已经有同名存档 {file}，没有覆盖"
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 同盘 rename 一步到位；跨盘 rename 会失败，退回复制 + 删
    if std::fs::rename(&source, &target).is_err() {
        std::fs::copy(&source, &target)?;
        std::fs::remove_file(&source)?;
    }
    Ok(())
}

/// 把某一份存档挪到**某个战役 / 版本名下**（键来自 [`targets`]）。
///
/// 用户改的是「**这一份**存档属于谁」，不是把整个组改绑到别的战役 ——
/// 组是**自动生成**的，它的名字就是它的意义（「虚空之遗」不该装 AI 盟友的档）。所以：
/// - 已经有组绑着这个键：挪进那个组；
/// - 没有：按 [`group_label`] 的规则建一个新组、自动绑上这个键，再把文件挪进去。
///
/// 底线不变：**同名不覆盖**、只动 `<data>/saves/`、**绝不碰 Banks**。
/// 返回挪进去的组名。
pub fn move_save_to_key(root: &Path, from: &str, file: &str, key: &str) -> Result<String> {
    let library = Library::new(root);
    let target = match profile_for(root, key)? {
        Some(name) => name,
        None => {
            // 新建之前先把两件事确认掉：这个键是不是真的目标、源文件在不在。
            // 不然失败时账上会多一条指向空组的绑定。
            if !targets(&library, &[]).iter().any(|item| item.key == key) {
                return Err(Error::PackageRejected(format!("未知的归属：{key}")));
            }
            if !checked_name(root, from, file)?.is_file() {
                return Err(Error::PackageRejected(format!("找不到这份存档：{file}")));
            }
            let name = fresh_group(root, &group_label(&library, key))?;
            assign(root, &name, Some(key))?;
            name
        }
    };

    move_save_file(root, from, file, &target)?;
    Ok(target)
}
