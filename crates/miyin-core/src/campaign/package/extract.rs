//! 解压：把包里（剥掉内容根之后的）内容铺到目标目录。
//!
//! 这里也放「读一个条目 / 去掉内容根前缀」这两个最小的访问助手 ——
//! 预检（元数据解析、地图依赖扫描）和解压都要用。

use std::path::Path;

use crate::campaign::contents;
use crate::error::Result;

use super::paths::{safe_entry_path, strip_prefix};

/// 解压统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtractStats {
    pub files: usize,
    pub maps: usize,
    pub mods: usize,
    pub bytes: u64,
}

/// 把压缩包内容解压到目标目录（自动剥离内容根）。
///
/// 每一条落盘路径都会**再做一次包含性校验**：这是写盘的最后一道闸门，
/// 不依赖上游已经校验过。
pub fn extract_to(package: &Path, content_root: &str, destination: &Path) -> Result<ExtractStats> {
    extract_to_with_password(package, content_root, destination, None)
}

/// 同上，带密码（加密包预检通过之后才走到这里）。
///
/// 来源是目录时直接 `std::fs::copy`（见 `Contents` 的 Dir 后端），不从临时目录过一道。
pub fn extract_to_with_password(
    package: &Path,
    content_root: &str,
    destination: &Path,
    password: Option<&str>,
) -> Result<ExtractStats> {
    let mut archive = contents::Contents::open_source(package, password)?;
    let mut stats = ExtractStats::default();

    // **外部格式一次解完**，别一个条目起一个进程。
    //
    // 逐条 `copy_to` 对 zip 是对的（流式，不用先整包落一遍盘），
    // 但对 rar/7z 就是 164 个条目 = 164 次 tar —— 比整体解一次还慢。
    //
    // `strip` 用内容根有几段目录来算：包常常多套一层「包名」目录，
    // 那一层不该出现在库目录里。
    let strip = content_root
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .count();

    if archive.unpack_all(destination, strip)? {
        // 文件已经在盘上了，统计按条目清单算
        for entry in archive.entries() {
            if entry.is_dir {
                continue;
            }
            stats.files += 1;
            stats.bytes += entry.size;
            match extension_of(Path::new(&entry.name)).as_str() {
                "sc2map" => stats.maps += 1,
                "sc2mod" => stats.mods += 1,
                _ => {}
            }
        }
        return Ok(stats);
    }

    let listing: Vec<(String, bool)> = archive
        .entries()
        .iter()
        .map(|item| (item.name.clone(), item.is_dir))
        .collect();

    for (index, (name, is_dir)) in listing.into_iter().enumerate() {
        let Some(relative) = safe_entry_path(&name) else {
            continue;
        };
        let Some(stripped) = strip_prefix(&relative, content_root) else {
            continue;
        };
        if stripped.as_os_str().is_empty() {
            continue;
        }

        let target = crate::safety::ensure_within(destination, &destination.join(&stripped))?;

        if is_dir {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        archive.copy_to(index, &target)?;
        stats.files += 1;
        stats.bytes += std::fs::metadata(&target)
            .map(|meta| meta.len())
            .unwrap_or(0);
        match extension_of(&stripped).as_str() {
            "sc2map" => stats.maps += 1,
            "sc2mod" => stats.mods += 1,
            _ => {}
        }
    }

    Ok(stats)
}

/// 读取包内某个条目的全部字节。
pub(crate) fn read_entry(archive: &mut contents::Contents, index: usize) -> Option<Vec<u8>> {
    archive.read(index)
}

/// 小写扩展名。
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}
