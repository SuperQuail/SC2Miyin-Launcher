//! 星际争霸 II 安装的发现、校验与路径推导。
//!
//! 发现顺序：Windows 注册表 →（失败则）由用户手动指定。
//! 校验以安装根目录下是否存在 `StarCraft II.exe` 为准。
//!
//! 本模块刻意规避了旧实现（星际枢纽）中几处已知的脆弱假设：
//!
//! 1. **不假设 `Maps` / `Mods` 存在**——它们只在用户首次使用自制内容后才出现，
//!    因此这里只推导路径，「按需创建」交给 ensure_dir。
//! 2. **不硬编码版本号**——版本一律从 `.build.info` 读取，版本目录由 `Versions/Base*` 扫描得到。
//! 3. 注册表同时尝试 32 位与 64 位视图，避免因安装方式不同而找不到游戏。

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::build_info::BuildInfo;
use crate::error::{Error, Result};

/// 判定目录是否为有效安装的标记文件。
pub const MARKER: &str = "StarCraft II.exe";

/// 安装信息的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    /// 由 Windows 注册表发现。
    Registry,
    /// 由用户在设置中手动指定。
    Manual,
}

/// 一份可用的星际争霸 II 安装。
#[derive(Debug, Clone, Serialize)]
pub struct Installation {
    /// 安装根目录（已解析为绝对路径）。
    pub root: PathBuf,
    /// 该安装是如何被找到的。
    pub source: DiscoverySource,
    /// 完整版本号，如 `5.0.16.97579`。
    pub version: Option<String>,
    /// 构建号，如 `97579`。
    pub build: Option<u32>,
    /// 安装分支，如 `cn`。
    pub branch: Option<String>,

    /// 根目录下的启动器 `StarCraft II.exe`。
    pub executable: PathBuf,
    /// `Support64/SC2Switcher_x64.exe`（版本切换器，建议的启动入口）。
    pub switcher: Option<PathBuf>,
    /// `Support64/SC2Editor_x64.exe`（地图编辑器）。
    pub editor: Option<PathBuf>,

    /// `Versions`（每个构建号一个 `Base<build>` 目录）。
    pub versions_root: PathBuf,
    /// `Maps`。
    pub maps_root: PathBuf,
    /// `Mods`。
    pub mods_root: PathBuf,
    /// `Interfaces`（界面 Mod）。
    pub interfaces_root: PathBuf,
    /// `Maps/CustomCampaigns`（CCM 自制战役根目录）。
    pub custom_campaigns_root: PathBuf,
    /// `Maps/Campaign`（官方战役地图）。
    pub campaign_maps_root: PathBuf,

    /// 用户文档目录下的 `StarCraft II`。
    pub documents_root: Option<PathBuf>,
    /// 文档目录下的 `Maps`。
    pub user_maps_root: Option<PathBuf>,
    /// 文档目录下的 `Banks`（战役存档，可能不存在）。
    pub banks_root: Option<PathBuf>,
    /// 文档目录下的 `ArcadeBanks`（大厅存档，可能不存在）。
    pub arcade_banks_root: Option<PathBuf>,
}

impl Installation {
    /// 校验目录是否为有效的游戏安装。
    pub fn validate(root: &Path) -> Result<()> {
        if root.join(MARKER).is_file() {
            Ok(())
        } else {
            Err(Error::InvalidInstallation {
                path: root.to_path_buf(),
                marker: MARKER,
            })
        }
    }

    /// 从已知根目录构造安装信息（用于「手动指定游戏目录」）。
    pub fn from_root(root: impl AsRef<Path>, source: DiscoverySource) -> Result<Self> {
        let root = crate::safety::resolve(root.as_ref())?;
        Self::validate(&root)?;
        Ok(Self::build(root, source))
    }

    /// 自动发现安装：先查注册表，失败即报 `InstallationNotFound`。
    pub fn discover() -> Result<Self> {
        if let Some(root) = registry_install_location()? {
            if let Ok(installation) = Self::from_root(&root, DiscoverySource::Registry) {
                return Ok(installation);
            }
        }
        Err(Error::InstallationNotFound)
    }

    /// 组装各推导路径。
    fn build(root: PathBuf, source: DiscoverySource) -> Self {
        let support64 = root.join("Support64");
        let maps_root = root.join("Maps");
        let documents_root = dirs::document_dir().map(|documents| documents.join("StarCraft II"));

        let build_info = BuildInfo::read(&root.join(".build.info")).ok();

        Self {
            version: build_info
                .as_ref()
                .and_then(|info| info.version().map(str::to_string)),
            build: build_info.as_ref().and_then(BuildInfo::build_number),
            branch: build_info
                .as_ref()
                .and_then(|info| info.branch().map(str::to_string)),

            executable: root.join(MARKER),
            switcher: existing(support64.join("SC2Switcher_x64.exe")),
            editor: existing(support64.join("SC2Editor_x64.exe")),

            versions_root: root.join("Versions"),
            mods_root: root.join("Mods"),
            interfaces_root: root.join("Interfaces"),
            custom_campaigns_root: maps_root.join("CustomCampaigns"),
            campaign_maps_root: maps_root.join("Campaign"),
            maps_root,
            user_maps_root: documents_root.as_ref().map(|dir| dir.join("Maps")),
            banks_root: documents_root.as_ref().map(|dir| dir.join("Banks")),
            arcade_banks_root: documents_root.as_ref().map(|dir| dir.join("ArcadeBanks")),
            documents_root,
            root,
            source,
        }
    }

    /// 真正执行游戏的客户端，即 `Versions/Base<build>/SC2_x64.exe`。
    ///
    /// 优先匹配与当前构建号一致的目录，找不到时退回编号最大的那个。
    pub fn client_executable(&self) -> Option<PathBuf> {
        let version_dir = self.version_dir()?;
        ["SC2_x64.exe", "SC2.exe"]
            .iter()
            .map(|name| version_dir.join(name))
            .find(|candidate| candidate.is_file())
    }

    /// 定位 `Versions/Base<build>` 目录。
    pub fn version_dir(&self) -> Option<PathBuf> {
        let entries = std::fs::read_dir(&self.versions_root).ok()?;
        let mut newest: Option<(u32, PathBuf)> = None;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(number) = name
                .strip_prefix("Base")
                .and_then(|rest| rest.parse::<u32>().ok())
            else {
                continue;
            };

            // 构建号一致则直接命中
            if self.build == Some(number) {
                return Some(path);
            }
            if newest.as_ref().is_none_or(|(best, _)| number > *best) {
                newest = Some((number, path));
            }
        }

        newest.map(|(_, path)| path)
    }

    /// 推荐给用户的启动入口：优先版本切换器，退回根启动器。
    pub fn preferred_launcher(&self) -> &Path {
        match &self.switcher {
            Some(switcher) => switcher.as_path(),
            None => self.executable.as_path(),
        }
    }

    /// 允许写入的根目录白名单（见 `AGENTS.md` 10.5 节）。
    pub fn allowed_roots(&self) -> Vec<&Path> {
        let mut roots = vec![
            self.root.as_path(),
            self.maps_root.as_path(),
            self.mods_root.as_path(),
            self.interfaces_root.as_path(),
        ];
        if let Some(documents) = &self.documents_root {
            roots.push(documents.as_path());
        }
        roots
    }

    /// 按需创建目录，并校验其位于白名单内。
    pub fn ensure_dir(&self, dir: &Path) -> Result<PathBuf> {
        let resolved = crate::safety::ensure_within_any(self.allowed_roots(), dir)?;
        std::fs::create_dir_all(&resolved)?;
        Ok(resolved)
    }

    /// 按需创建自制战役根目录 `Maps/CustomCampaigns`。
    pub fn ensure_custom_campaigns_dir(&self) -> Result<PathBuf> {
        self.ensure_dir(&self.custom_campaigns_root)
    }
}

/// 仅当路径存在时返回 `Some`。
fn existing(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

/// 从注册表读取安装位置。
///
/// 同时尝试 `WOW6432Node`（32 位视图）与 64 位视图：不同渠道安装的游戏
/// 可能只写到其中之一。
#[cfg(windows)]
fn registry_install_location() -> Result<Option<PathBuf>> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    const KEYS: [&str; 2] = [
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\StarCraft II",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\StarCraft II",
    ];

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    for key in KEYS {
        let Ok(subkey) = hklm.open_subkey(key) else {
            continue;
        };
        for value_name in ["InstallLocation", "InstallPath"] {
            if let Ok(location) = subkey.get_value::<String, _>(value_name) {
                let trimmed = location.trim();
                if !trimmed.is_empty() {
                    return Ok(Some(PathBuf::from(trimmed)));
                }
            }
        }
    }

    Ok(None)
}

/// 非 Windows 平台暂不支持自动发现。
#[cfg(not(windows))]
fn registry_install_location() -> Result<Option<PathBuf>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Version!STRING:0\n\
us|0|1.0.0.1\n\
cn|1|5.0.16.97579\n";

    /// 造一个「长得像」的安装目录。
    fn fake_install() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::write(root.join(MARKER), b"stub").expect("marker");
        std::fs::write(root.join(".build.info"), SAMPLE_BUILD_INFO).expect("build info");
        std::fs::create_dir_all(root.join("Support64")).expect("support64");
        std::fs::write(root.join("Support64").join("SC2Switcher_x64.exe"), b"stub")
            .expect("switcher");

        let version_dir = root.join("Versions").join("Base97579");
        std::fs::create_dir_all(&version_dir).expect("versions");
        std::fs::write(version_dir.join("SC2_x64.exe"), b"stub").expect("client");

        dir
    }

    #[test]
    fn rejects_directory_without_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Installation::from_root(dir.path(), DiscoverySource::Manual)
            .expect_err("缺少 StarCraft II.exe 必须被拒绝");
        assert!(matches!(err, Error::InvalidInstallation { .. }));
    }

    #[test]
    fn accepts_valid_looking_install_and_derives_paths() {
        let dir = fake_install();
        let installation =
            Installation::from_root(dir.path(), DiscoverySource::Manual).expect("应通过校验");

        // 关键：版本来自 .build.info，而不是硬编码
        assert_eq!(installation.version.as_deref(), Some("5.0.16.97579"));
        assert_eq!(installation.build, Some(97579));
        assert_eq!(installation.branch.as_deref(), Some("cn"));

        assert!(
            installation
                .custom_campaigns_root
                .ends_with("CustomCampaigns")
        );
        assert!(installation.maps_root.ends_with("Maps"));
        assert_eq!(installation.source, DiscoverySource::Manual);
    }

    #[test]
    fn locates_client_executable_in_matching_version_dir() {
        let dir = fake_install();
        let installation =
            Installation::from_root(dir.path(), DiscoverySource::Manual).expect("应通过校验");

        let client = installation.client_executable().expect("应找到客户端");
        assert!(client.ends_with("Base97579/SC2_x64.exe"));
        assert_eq!(
            installation.preferred_launcher(),
            installation.switcher.as_deref().expect("switcher")
        );
    }

    #[test]
    fn creates_custom_campaigns_dir_on_demand() {
        let dir = fake_install();
        let installation =
            Installation::from_root(dir.path(), DiscoverySource::Manual).expect("应通过校验");

        // 全新安装的游戏没有 Maps/CustomCampaigns，必须能按需创建
        assert!(!installation.custom_campaigns_root.exists());
        let created = installation
            .ensure_custom_campaigns_dir()
            .expect("应能创建目录");
        assert!(created.is_dir());
    }
}
