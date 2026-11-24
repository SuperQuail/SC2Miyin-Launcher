//! 写盘用的文件系统小工具：路径解析、写盘闸门、拷贝与体积统计。

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;

/// 清单里记的「来源」：**优先相对库根**（统一 `/` 分隔），库外的源退回绝对路径。
///
/// 库根会跟着数据迁移 / 游戏目录搬家而变（`AGENTS.md` §14.1）：记绝对路径的话，
/// 搬一次家反查就再也对不上装到哪了 —— 打开地图只能退回库里那份，
/// 而那份在游戏目录外，打不开（R3）。库外的源（老调用方）保留绝对路径。
pub(super) fn source_key(data: &Path, source: &Path) -> String {
    match source.strip_prefix(data) {
        Ok(rest) if !rest.as_os_str().is_empty() => rest.to_string_lossy().replace('\\', "/"),
        _ => source.to_string_lossy().to_string(),
    }
}

/// 把相对路径解析成游戏目录下的绝对路径，并过白名单校验。
pub(super) fn resolve(installation: &Installation, relative: &str) -> Result<PathBuf> {
    let path = installation
        .root
        .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    allowed_target(installation, &path)
}

/// 校验目标落在**星际争霸安装目录之内**。
///
/// 早先只放行 `Maps` / `Mods` / `Interfaces` 三个根，代价是复刻战役整包装不进去：
/// 地图要落在 `Maps/Starcraft Mass Recall/…`（启动器地图按这个相对 `Maps/` 的
/// 路径联动关卡），却被自己的闸门拒绝，用户看到的是「拒绝往 … 写东西」。
///
/// 现在放开成**目录内任意位置**：想覆盖 `SC2Data/`、想换 `Versions/` 里的东西
/// 都随作者 —— 自由度归作者，**出目录一律拒绝**。
///
/// 安全感不来自收窄路径，而来自**动之前先备份**：`apply` 会把要覆盖的原件挪进
/// `data/backup/` 并记账，`remove` 原样还原；`preview` 先把要动的东西列出来。
/// 见 `docs/developer-workflow.md` §2。
///
/// 这是写盘的最后一道闸门 —— 不依赖上游校验过没有。
fn allowed_target(installation: &Installation, path: &Path) -> Result<PathBuf> {
    safety::ensure_within(&installation.root, path).map_err(|_| {
        Error::PackageRejected(format!(
            "无法写入 {}：只能写入星际争霸 II 安装目录内的文件",
            path.display()
        ))
    })
}

/// 一个文件或一棵目录树有多大；不存在就是 0。
pub(super) fn path_bytes(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }

    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum()
}

/// 拷一个文件或一棵目录树。
pub(super) fn copy_entry(source: &Path, target: &Path) -> Result<()> {
    if source.is_dir() {
        std::fs::create_dir_all(target)?;
        for entry in walkdir::WalkDir::new(source)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
            let dest = target.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dest)?;
            } else {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(entry.path(), &dest)?;
            }
        }
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, target)?;
    Ok(())
}
