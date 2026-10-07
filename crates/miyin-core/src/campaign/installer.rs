//! 事务化安装与卸载。
//!
//! # 设计原则
//!
//! 1. **先解压到暂存目录，校验通过后再整体切换**（rename 是原子操作）。
//!    参考实现是"先删旧目录，再原地解压"，中途失败就两头落空。
//! 2. **删除范围严格限定在包自己的目录内**，永远不碰父目录。
//!    参考实现的卸载会 `join(metadata_path, "..")`，WOL 战役会因此删掉整个 `@Maps`@。
//! 3. **每一次路径拼接都过一遍白名单校验**，即便前面已经校验过包内容。
//! 4. **失败可回滚**：切换失败时把备份改回原名。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::campaign::package::{self, PackageInspection};
use crate::campaign::sanitize::sanitize_dir_name;
use crate::campaign::{Campaign, scanner};
use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;

/// 安装用的暂存目录前缀（以点开头，扫描时会被跳过）。
const STAGING_PREFIX: &str = ".miyin-staging-";

/// 替换旧版本时的备份目录前缀。
const BACKUP_PREFIX: &str = ".miyin-backup-";

/// 安装计划：预演结果 + 落地位置。
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub inspection: PackageInspection,
    /// 战役将被安装到的目录。
    pub target_dir: PathBuf,
    /// 目录名（已安全化）。
    pub dir_name: String,
    /// 该名字是否已被占用（安装会替换旧版本，旧版本会被备份）。
    pub replaces_existing: bool,
}

/// 制定安装计划（不产生任何写入）。
pub fn plan(installation: &Installation, package_path: &Path) -> Result<InstallPlan> {
    let inspection = package::inspect(package_path)?;

    if inspection.has_blocking_issue() {
        let reason = inspection
            .issues
            .iter()
            .find(|issue| issue.level == crate::campaign::HealthLevel::Broken)
            .map(|issue| issue.message.clone())
            .unwrap_or_else(|| "未知原因".to_string());
        return Err(Error::PackageRejected(reason));
    }

    let dir_name = inspection
        .suggested_dir_name
        .clone()
        .filter(|name| sanitize_dir_name(name).is_some())
        .ok_or_else(|| Error::PackageRejected("无法推导出可用的安装目录名".to_string()))?;

    let root = &installation.custom_campaigns_root;
    let target_dir = safety::ensure_within(root, &root.join(&dir_name))?;

    Ok(InstallPlan {
        replaces_existing: target_dir.exists(),
        inspection,
        target_dir,
        dir_name,
    })
}

/// 解压并安装一个战役包。
pub fn install(installation: &Installation, package_path: &Path) -> Result<Campaign> {
    let plan = plan(installation, package_path)?;
    let root = installation.ensure_custom_campaigns_dir()?;

    // 清掉上一次异常退出留下的暂存目录
    cleanup_stale(&root);

    let staging = safety::ensure_within(
        &root,
        &root.join(format!("{STAGING_PREFIX}{}", unique_suffix())),
    )?;
    std::fs::create_dir_all(&staging)?;

    let extraction = package::extract_to(package_path, &plan.inspection.content_root, &staging);
    if let Err(error) = extraction {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    // 校验：至少要有内容，避免把一个空壳目录换上去
    if std::fs::read_dir(&staging)?.next().is_none() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::PackageRejected("压缩包内容为空".to_string()));
    }

    // 已存在同名战役 -> 先备份，切换成功后再删除备份
    let mut backup: Option<PathBuf> = None;
    if plan.target_dir.exists() {
        let backup_dir = safety::ensure_within(
            &root,
            &root.join(format!(
                "{BACKUP_PREFIX}{}-{}",
                unique_suffix(),
                plan.dir_name
            )),
        )?;
        std::fs::rename(&plan.target_dir, &backup_dir)?;
        backup = Some(backup_dir);
    }

    if let Err(error) = std::fs::rename(&staging, &plan.target_dir) {
        // 回滚
        if let Some(backup_dir) = &backup {
            let _ = std::fs::rename(backup_dir, &plan.target_dir);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error.into());
    }

    if let Some(backup_dir) = backup {
        let _ = std::fs::remove_dir_all(backup_dir);
    }

    scanner::inspect_dir(&plan.target_dir)
}

/// 卸载一个战役。
///
/// `campaign_id` 必须是 `CustomCampaigns` 下的**单层目录名**，
/// 这里会重新做一次安全化与包含性校验，杜绝 `..` 之类的输入把父目录带走。
pub fn uninstall(installation: &Installation, campaign_id: &str) -> Result<()> {
    let safe_id = sanitize_dir_name(campaign_id)
        .ok_or_else(|| Error::PackageRejected(format!("非法的战役标识：{campaign_id}")))?;

    let root = safety::resolve(&installation.custom_campaigns_root)?;
    if !root.is_dir() {
        return Err(Error::CampaignNotFound(campaign_id.to_string()));
    }

    let target = safety::ensure_within(&root, &root.join(&safe_id))?;

    // 绝不允许把根目录本身当作删除目标
    if target == root {
        return Err(Error::PackageRejected(
            "拒绝删除自制战役根目录本身".to_string(),
        ));
    }
    if !target.is_dir() {
        return Err(Error::CampaignNotFound(campaign_id.to_string()));
    }

    std::fs::remove_dir_all(&target)?;
    Ok(())
}

/// 清理本程序遗留的暂存目录。
///
/// 只处理我们自己前缀命名的目录，且要求它确实位于白名单根内。
fn cleanup_stale(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(STAGING_PREFIX) {
            continue;
        }
        if let Ok(path) = safety::ensure_within(root, &entry.path()) {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

/// 生成一个在目录内唯一的后缀。
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sc2::DiscoverySource;
    use std::io::Write;

    const MARKER: &str = "StarCraft II.exe";

    fn fake_install() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join(MARKER), b"stub").expect("marker");
        std::fs::write(
            root.join(".build.info"),
            "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
        )
        .expect("build info");
        dir
    }

    fn build_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).expect("create zip");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (entry_name, content) in files {
            writer.start_file(*entry_name, options).expect("start file");
            writer.write_all(content).expect("write");
        }
        writer.finish().expect("finish");
        path
    }

    fn setup() -> (tempfile::TempDir, Installation) {
        let dir = fake_install();
        let installation =
            Installation::from_root(dir.path(), DiscoverySource::Manual).expect("安装");
        (dir, installation)
    }

    #[test]
    fn installs_ccm_package_and_strips_content_root() {
        let (dir, installation) = setup();
        let archive = build_zip(
            dir.path(),
            "reborn.zip",
            &[
                (
                    "Reborn/metadata.txt",
                    "title=Reborn\nauthor=Creator\ncampaign=WOL\nversion=1.0\n".as_bytes(),
                ),
                ("Reborn/maps/01.SC2Map", b"stub"),
                ("Reborn/maps/02.SC2Map", b"stub"),
            ],
        );

        let campaign = install(&installation, &archive).expect("安装应成功");
        assert_eq!(campaign.name, "Reborn");
        assert_eq!(campaign.map_count, Some(2));

        // 关键：内容根被剥离，metadata.txt 直接落在战役目录下
        let installed_dir = installation.custom_campaigns_root.join("Reborn");
        assert!(installed_dir.join("metadata.txt").is_file());
        assert!(installed_dir.join("maps").join("01.SC2Map").is_file());
        // 不应留下暂存目录
        let leftovers: Vec<_> = std::fs::read_dir(&installation.custom_campaigns_root)
            .expect("read")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(STAGING_PREFIX)
            })
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn reinstalling_replaces_previous_version_without_leftovers() {
        let (dir, installation) = setup();
        let first = build_zip(
            dir.path(),
            "a.zip",
            &[
                (
                    "metadata.txt",
                    "title=Reborn\ncampaign=WOL\nversion=1.0\n".as_bytes(),
                ),
                ("maps/old.SC2Map", b"stub"),
            ],
        );
        install(&installation, &first).expect("首次安装");

        let second = build_zip(
            dir.path(),
            "b.zip",
            &[
                (
                    "metadata.txt",
                    "title=Reborn\ncampaign=WOL\nversion=2.0\n".as_bytes(),
                ),
                ("maps/new.SC2Map", b"stub"),
            ],
        );
        let campaign = install(&installation, &second).expect("覆盖安装");

        assert_eq!(campaign.version.as_deref(), Some("2.0"));
        let installed_dir = installation.custom_campaigns_root.join("Reborn");
        assert!(installed_dir.join("maps").join("new.SC2Map").is_file());
        assert!(!installed_dir.join("maps").join("old.SC2Map").exists());
        // 备份目录必须被清干净
        let backups: Vec<_> = std::fs::read_dir(&installation.custom_campaigns_root)
            .expect("read")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(BACKUP_PREFIX)
            })
            .collect();
        assert!(backups.is_empty());
    }

    #[test]
    fn refuses_package_with_traversal_entry() {
        let (dir, installation) = setup();
        let archive = build_zip(
            dir.path(),
            "evil.zip",
            &[
                ("metadata.txt", "title=Evil\ncampaign=WOL\n".as_bytes()),
                ("../../../../escaped.txt", b"gotcha"),
            ],
        );

        let error = install(&installation, &archive).expect_err("必须拒绝");
        assert!(matches!(error, Error::PackageRejected(_)));
        // 游戏根目录里绝不能出现逃逸文件
        assert!(!dir.path().join("escaped.txt").exists());
    }

    #[test]
    fn unusable_title_falls_back_to_archive_name_without_escaping() {
        let (dir, installation) = setup();
        // 参考实现会把这样的 title 直接当目录名：join(root, "..") 之后递归删除整个 Maps
        let archive = build_zip(
            dir.path(),
            "evil2.zip",
            &[("metadata.txt", b"title=..\ncampaign=WOL\n")],
        );

        let campaign = install(&installation, &archive).expect("应退回使用压缩包名完成安装");

        // 名字被安全化，落点严格在 CustomCampaigns 之内
        assert_eq!(campaign.id, "evil2");
        assert_eq!(
            campaign.path.parent(),
            Some(installation.custom_campaigns_root.as_path())
        );
        // 没有任何东西逃逸到游戏根
        assert!(!dir.path().join("metadata.txt").exists());
        assert!(dir.path().join("StarCraft II.exe").is_file());
    }

    #[test]
    fn uninstall_only_removes_the_campaign_directory() {
        let (dir, installation) = setup();
        let archive = build_zip(
            dir.path(),
            "c.zip",
            &[
                ("metadata.txt", "title=Reborn\ncampaign=WOL\n".as_bytes()),
                ("maps/01.SC2Map", b"stub"),
            ],
        );
        install(&installation, &archive).expect("安装");

        // 造一个"邻居"，确认卸载不会波及它
        let neighbour = installation.custom_campaigns_root.join("Neighbour");
        std::fs::create_dir_all(&neighbour).expect("neighbour");

        uninstall(&installation, "Reborn").expect("卸载");
        assert!(!installation.custom_campaigns_root.join("Reborn").exists());
        assert!(neighbour.is_dir(), "邻居目录不能被删除");
        assert!(
            installation.custom_campaigns_root.is_dir(),
            "根目录不能被删除"
        );
        let _ = dir;
    }

    #[test]
    fn uninstall_rejects_parent_directory_ids() {
        let (_dir, installation) = setup();
        std::fs::create_dir_all(&installation.custom_campaigns_root).expect("root");

        // 这些输入安全化后完全不可用，必须被直接拒绝
        for bad in ["..", ".", "", "   ", "CON"] {
            let error = uninstall(&installation, bad).expect_err("必须拒绝");
            assert!(matches!(error, Error::PackageRejected(_)), "输入：{bad}");
        }

        // 这类输入会被安全化成普通名字（a_.._.._b），因此结果是「找不到」而非越界删除
        let error = uninstall(&installation, "a/../../b").expect_err("应报找不到");
        assert!(matches!(error, Error::CampaignNotFound(_)));

        assert!(
            installation.custom_campaigns_root.is_dir(),
            "根目录必须完好"
        );
    }
}
