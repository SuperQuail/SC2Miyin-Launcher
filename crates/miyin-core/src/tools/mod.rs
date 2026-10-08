//! **可选外部工具**的管理：目前是 SC2Diff。
//!
//! SC2Diff 是另一套东西：对 `.SC2Map` / `.SC2Mod`（本质是 MPQ 归档）做**语义 diff**
//! 与 git 形状的版本控制，是我们「模组只存差异」那条路的后端。它不是启动器的必需件 ——
//! 用户不装也能正常用，只是少了 diff 能力。所以：
//!
//! - **可选**：装不装、装哪个版本都由用户定
//! - **默认静默安装**：不去弹浏览器、不去让用户手动解压，点了就装好
//! - **用 Rust 版的 release 附件**（`sc2diff.exe`，单文件无运行时依赖），
//!   不用源码包 —— 用户机器上不该被迫装工具链
//!
//! 安装位置：`<data>/tools/<id>/`，与战役库、模组库并列。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::safety;
use crate::update::Reporter;
use crate::update::check;
use crate::update::net::NetworkSettings;

/// 一个可选工具的定义。
#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    /// 内部标识，同时是目录名。
    pub id: &'static str,
    /// 显示名。
    pub name: &'static str,
    /// GitHub 仓库（`owner/repo`）。
    pub repo: &'static str,
    /// 从 Release 附件里挑哪个文件。
    pub asset: &'static str,
    /// 一句话说明。
    pub about: &'static str,
    /// 装好之后可执行文件叫什么。
    pub binary: &'static str,
}

/// SC2Diff：地图 / 模组的语义 diff 与版本控制。
pub const SC2DIFF: ToolSpec = ToolSpec {
    id: "sc2diff",
    name: "SC2Diff",
    repo: "SuperQuail/SC2Diff",
    asset: "sc2diff.exe",
    about: "对地图和模组做语义 diff 与版本控制，模组「只存差异」靠它",
    binary: "sc2diff.exe",
};

/// 现在管着哪些工具。
pub const ALL: [ToolSpec; 1] = [SC2DIFF];

/// 一个工具的安装状态。
#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub about: String,
    pub repo: String,
    /// 装没装。
    pub installed: bool,
    /// 装的是哪个版本（对外写法）。
    pub version: Option<String>,
    /// 可执行文件的位置。
    pub path: Option<String>,
    pub size_bytes: u64,
}

/// Release 里的一个可装版本。
#[derive(Debug, Clone, Serialize)]
pub struct ToolRelease {
    pub version: String,
    pub tag: String,
    pub published_at: String,
    pub prerelease: bool,
    /// 附件的下载地址。
    pub download_url: String,
    pub size: u64,
    /// 有没有提供我们要的那个附件。
    pub has_asset: bool,
}

/// 工具的安装目录。
fn tool_dir(data: &Path, spec: &ToolSpec) -> PathBuf {
    data.join("tools").join(spec.id)
}

/// 版本记录下来放哪。
fn version_file(data: &Path, spec: &ToolSpec) -> PathBuf {
    tool_dir(data, spec).join("version.txt")
}

/// 可执行文件的路径 —— **只有真的存在才算装好**。
///
/// 只看 version.txt 是不够的：用户可能手删了 exe，那样界面会显示"已安装"却跑不起来。
pub fn executable(data: &Path, spec: &ToolSpec) -> Option<PathBuf> {
    let path = tool_dir(data, spec).join(spec.binary);
    path.is_file().then_some(path)
}

/// 装了什么。
pub fn status(data: &Path, spec: &ToolSpec) -> ToolStatus {
    let path = executable(data, spec);
    let version = std::fs::read_to_string(version_file(data, spec))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());

    ToolStatus {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        about: spec.about.to_string(),
        repo: spec.repo.to_string(),
        installed: path.is_some(),
        // 文件没了就别再报版本号，免得用户以为还能用
        version: if path.is_some() { version } else { None },
        size_bytes: path
            .as_ref()
            .and_then(|path| std::fs::metadata(path).ok())
            .map(|meta| meta.len())
            .unwrap_or(0),
        path: path.map(|path| path.display().to_string()),
    }
}

/// 查这个工具有哪些版本可装。
///
/// 预发行照收 —— SC2Diff 现在**只有**预发行，滤掉就等于没得装。
pub fn releases(
    spec: &ToolSpec,
    settings: &NetworkSettings,
    reporter: Reporter<'_>,
) -> std::result::Result<Vec<ToolRelease>, String> {
    reporter.say(format!("查询 {} 的发行版本…", spec.name));

    let list = check::releases_for(spec.repo, settings, reporter)?;

    let mut rows: Vec<ToolRelease> = list
        .into_iter()
        .map(|release| {
            let asset = release.assets.iter().find(|asset| asset.name == spec.asset);

            ToolRelease {
                version: release.version,
                tag: release.tag,
                published_at: release.published_at,
                prerelease: release.prerelease,
                download_url: asset
                    .map(|asset| asset.download_url.clone())
                    .unwrap_or_default(),
                size: asset.map(|asset| asset.size).unwrap_or(0),
                has_asset: asset.is_some(),
            }
        })
        .collect();

    // 新的在前 —— 挑版本时第一个就是最新
    rows.sort_by(|left, right| right.published_at.cmp(&left.published_at));
    reporter.say(format!("读到 {} 个版本", rows.len()));
    Ok(rows)
}

/// 下载并安装指定版本。
///
/// 先下到临时文件再整体改名 —— 中途失败不会留下半个 exe，
/// 也不会把已经装好的那个覆盖坏。
pub fn install(
    data: &Path,
    spec: &ToolSpec,
    release: &ToolRelease,
    settings: &NetworkSettings,
    reporter: Reporter<'_>,
) -> Result<ToolStatus> {
    if !release.has_asset {
        return Err(Error::PackageRejected(format!(
            "这个版本（{}）里没有 {}，换一个版本试试",
            release.tag, spec.asset
        )));
    }

    let dir = tool_dir(data, spec);
    std::fs::create_dir_all(&dir)?;

    let asset = check::ReleaseAsset {
        name: spec.asset.to_string(),
        size: release.size,
        download_url: release.download_url.clone(),
        sha256: None,
    };

    let staging = safety::ensure_within(&dir, &dir.join(format!(".{}.download", spec.binary)))?;
    let outcome = check::download(&asset, settings, &staging, reporter)?;

    // 下下来的必须真是个 Windows 可执行文件 —— 有的镜像会把错误页当附件返回
    if !looks_like_pe(&staging) {
        let _ = std::fs::remove_file(&staging);
        return Err(Error::PackageRejected(
            "下载到的不是可执行文件，可能被镜像或代理拦截了".to_string(),
        ));
    }

    reporter.say(format!("下载完成（{}）", check::human_size(outcome.bytes)));

    let target = safety::ensure_within(&dir, &dir.join(spec.binary))?;
    if target.exists() {
        let _ = std::fs::remove_file(&target);
    }
    std::fs::rename(&staging, &target)?;
    std::fs::write(version_file(data, spec), &release.version)?;

    reporter.say(format!("{} {} 已装好", spec.name, release.version));
    Ok(status(data, spec))
}

/// 是不是一个 PE 文件（`MZ` 开头）。
fn looks_like_pe(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    bytes.len() > 2 && &bytes[..2] == b"MZ"
}

/// 卸载：把整个工具目录删掉。
pub fn uninstall(data: &Path, spec: &ToolSpec) -> Result<()> {
    let dir = tool_dir(data, spec);
    if dir.exists() {
        safety::ensure_within(&data.join("tools"), &dir)?;
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}
