//! 从游戏目录里撤东西：删掉我们放的、还原被挪走的官方文件。
//!
//! 从 `install.rs` 拆出来的 —— `Manifest` 只管记账，真正动磁盘的就这两步。

use std::path::Path;

use crate::error::{Error, Result};
use crate::safety;

use super::Manifest;

/// 删掉一份我们放的东西（文件或目录）；**删不掉就把路径报出来**。
///
/// 「目标不在了」不算失败（并发撤销 / 上一轮已经删过）；只要它还在盘上，
/// 就是真的没删掉 —— 这时候静默返回会让清单与磁盘永久分叉（R5）。
pub(super) fn remove_installed(path: &Path) -> Result<()> {
    let removed = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path.is_file() {
        std::fs::remove_file(path)
    } else {
        return Ok(());
    };

    match removed {
        Ok(()) => Ok(()),
        Err(_) if !path.exists() => Ok(()),
        Err(error) if crate::error::is_busy_io(&error) => Err(Error::FileBusy {
            path: path.to_path_buf(),
            reason: format!("文件正被其他程序占用（{error}）"),
        }),
        Err(error) => Err(Error::Io(std::io::Error::other(format!(
            "无法删除文件「{}」：{error}",
            path.display()
        )))),
    }
}

/// 把被挪走的官方文件还原回原位。
///
/// relative 是从清单里读出来的 —— **清单文件是可以被手改的**，所以这里
/// 和白名单路径一样要过 safety::ensure_within，否则一条
/// "backup": "../../../Windows/x" 就能把任意文件搬进游戏目录。
pub(super) fn restore_backup(data: &Path, target: &Path, relative: Option<&str>) -> Result<()> {
    let Some(relative) = relative else {
        return Ok(());
    };

    let root = Manifest::backup_root(data);
    let backup = safety::ensure_within(&root, &root.join(relative))?;
    if !backup.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::platform::move_path(&backup, target)?;
    Ok(())
}
