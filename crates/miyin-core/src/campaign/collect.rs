//! 「收编」：把已经躺在游戏目录里的自制战役收进库（AGENTS.md §12.2 的 TODO）。
//!
//! 这条路的用户是**先玩后整理**的人：战役是别人给的压缩包解开的，直接扔进
//! `Maps/CustomCampaigns` 就能玩 —— 玩了一阵才想用启动器管起来。
//!
//! 这里**只负责看**：列出游戏目录里有什么、多大。导入本身走的是既有的
//! `library::import`（它认得目录形态的包，见 `campaign::contents`），
//! 所以收编和从压缩包导入是**同一条路径**，不会长出第二套逻辑。
//!
//! 收编**不删原目录** —— 用户自己放的文件夹，删之前得他自己点头。

use std::path::Path;

use serde::Serialize;

use crate::error::Result;
use crate::sc2::Installation;

/// 游戏目录里已经装着的一个自制战役。
#[derive(Debug, Clone, Serialize)]
pub struct InstalledCampaign {
    /// 绝对路径 —— 导入时直接拿它当包路径。
    pub dir: String,
    /// 目录名（界面上先按它显示；真正的名字等 inspect 完再说）。
    pub name: String,
    pub bytes: u64,
    pub files: usize,
}

/// 扫 `<游戏>/Maps/CustomCampaigns/*`。目录不存在就返回空 —— 不是错误。
pub fn installed_campaigns(installation: &Installation) -> Result<Vec<InstalledCampaign>> {
    let root = installation.maps_root.join("CustomCampaigns");
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let (bytes, files) = measure(&entry.path(), 0);
        out.push(InstalledCampaign {
            dir: entry.path().to_string_lossy().into_owned(),
            name: entry.file_name().to_string_lossy().into_owned(),
            bytes,
            files,
        });
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 算一个目录的体积与文件数。**限深** —— 战役目录再深也就几层，
/// 无限往下走碰上符号链接会转圈。
fn measure(dir: &Path, depth: usize) -> (u64, usize) {
    if depth > 6 {
        return (0, 0);
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };

    let mut bytes = 0;
    let mut files = 0;
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            let (inner_bytes, inner_files) = measure(&entry.path(), depth + 1);
            bytes += inner_bytes;
            files += inner_files;
        } else if kind.is_file() {
            bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            files += 1;
        }
    }
    (bytes, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_directories_under_custom_campaigns() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::write(root.join("StarCraft II.exe"), b"stub").expect("marker");
        let installation =
            Installation::from_root(root, crate::sc2::DiscoverySource::Manual).expect("安装");

        // 没有 CustomCampaigns 时是空，不是错误
        assert!(installed_campaigns(&installation).expect("扫").is_empty());

        let custom = installation.maps_root.join("CustomCampaigns");
        std::fs::create_dir_all(custom.join("复刻战役")).expect("建目录");
        std::fs::write(custom.join("复刻战役").join("a.SC2Map"), b"12345").expect("写");
        std::fs::create_dir_all(custom.join("另一个战役")).expect("建目录");
        // 根下的散文件不算一个"战役"
        std::fs::write(custom.join("说明.txt"), b"x").expect("写");

        let list = installed_campaigns(&installation).expect("扫");
        assert_eq!(list.len(), 2, "只数目录");
        assert_eq!(list[0].name, "另一个战役");
        let one = list
            .iter()
            .find(|item| item.name == "复刻战役")
            .expect("有它");
        assert_eq!(one.files, 1);
        assert_eq!(one.bytes, 5);
    }
}
