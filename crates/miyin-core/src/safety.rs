//! 路径安全校验。
//!
//! 启动器会对游戏目录做写入与删除操作。一旦目标路径被构造错误（相对路径、
//! `..`、符号链接 / junction 重定向），就可能破坏用户的游戏安装。
//! 因此**所有写操作都必须先经过本模块校验**（见 `AGENTS.md` §10.5）。

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// 词法规范化：消除 `.` 与 `..`，**不访问文件系统**。
///
/// 无法回退的 `..`（例如根目录之上）会被原样保留，从而在校验阶段暴露为越界。
pub fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = matches!(out.components().next_back(), Some(Component::Normal(_)));
                if can_pop {
                    out.pop();
                } else {
                    out.push(Component::ParentDir.as_os_str());
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// 把路径解析为「尽量贴近真实的绝对路径」。
///
/// 与 [`std::fs::canonicalize`] 不同，本函数允许路径本身（及其尾部组件）尚不存在：
/// 它会向上寻找最近的**已存在**祖先并对其做 `canonicalize`（这会顺带解析符号链接
/// 与 junction），再把剩余组件拼回，最后做词法规范化。
///
/// 这样 `Maps` 下的 `CustomCampaigns` 这类「按需创建」的目录也能被安全校验。
pub fn resolve(path: &Path) -> Result<PathBuf> {
    let mut tail: Vec<&OsStr> = Vec::new();
    let mut cursor = path;

    while !cursor.exists() {
        match (cursor.parent(), cursor.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name);
                cursor = parent;
            }
            // 已到根（如 `C:\`）仍然不存在，交给 canonicalize 报错
            _ => break,
        }
    }

    let mut base = cursor.canonicalize()?;
    for name in tail.iter().rev() {
        base.push(name);
    }
    Ok(strip_verbatim(lexical_normalize(&base)))
}

/// 去掉 Windows 上 canonicalize 产生的逐字（verbatim）前缀，
/// 否则界面上会显示成难以阅读、且部分 API 不接受的形式。
fn strip_verbatim(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    if let Some(stripped) = strip_verbatim_windows(&path) {
        return stripped;
    }
    path
}

#[cfg(windows)]
fn strip_verbatim_windows(path: &Path) -> Option<PathBuf> {
    use std::path::Prefix;

    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(disk) => {
                let mut out = PathBuf::from(format!("{}:\\", disk as char));
                out.push(components.as_path());
                Some(out)
            }
            _ => None,
        },
        _ => None,
    }
}

/// 校验 `target` 是否落在 `root` 之内（含 `root` 自身），并返回解析后的绝对路径。
///
/// 比较是**按路径组件**进行的，因此 `a\bc` 不会被误判为 `a\b` 的子路径。
pub fn ensure_within(root: &Path, target: &Path) -> Result<PathBuf> {
    let root = resolve(root)?;
    let target = resolve(target)?;
    if target == root || target.starts_with(&root) {
        Ok(target)
    } else {
        Err(Error::PathEscapesRoot { root, target })
    }
}

/// 校验 `target` 是否落在**任意一个**白名单根目录内。
///
/// 尚不存在的根目录会被跳过——它不可能包含一个已存在的目标。
pub fn ensure_within_any<'a, I>(roots: I, target: &Path) -> Result<PathBuf>
where
    I: IntoIterator<Item = &'a Path>,
{
    let target_resolved = resolve(target)?;
    let mut last_root: Option<PathBuf> = None;

    for root in roots {
        if !root.exists() {
            continue;
        }
        let root_resolved = resolve(root)?;
        if target_resolved == root_resolved || target_resolved.starts_with(&root_resolved) {
            return Ok(target_resolved);
        }
        last_root = Some(root_resolved);
    }

    Err(Error::PathEscapesRoot {
        root: last_root.unwrap_or_else(|| PathBuf::from("(无)")),
        target: target_resolved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_normalize_removes_dot_and_parent() {
        assert_eq!(
            lexical_normalize(Path::new("a/./b/../c")),
            PathBuf::from("a/c")
        );
    }

    #[test]
    fn lexical_normalize_keeps_unresolvable_parent() {
        assert_eq!(lexical_normalize(Path::new("../a")), PathBuf::from("../a"));
    }

    #[test]
    fn ensure_within_accepts_nested_not_yet_existing_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("Maps").join("CustomCampaigns");
        let target = root.join("MyCampaign").join("metadata.json");

        // root 与 target 都不存在，也应能通过校验
        let resolved = ensure_within(&root, &target).expect("应判定为在根目录内");
        assert!(resolved.ends_with("MyCampaign/metadata.json"));
    }

    #[test]
    fn ensure_within_rejects_parent_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("CustomCampaigns");
        std::fs::create_dir_all(&root).expect("create root");
        let target = root.join("..").join("Evil");

        let err = ensure_within(&root, &target).expect_err("越界路径必须被拒绝");
        assert!(matches!(err, Error::PathEscapesRoot { .. }));
    }

    #[test]
    fn ensure_within_rejects_sibling_with_shared_prefix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let base = tmp.path().join("a");
        let root = base.join("b");
        let sibling = base.join("bc");
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::create_dir_all(&sibling).expect("create sibling");

        assert!(ensure_within(&root, &sibling).is_err());
    }
}
