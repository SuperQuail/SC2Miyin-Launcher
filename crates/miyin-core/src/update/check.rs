//! 查询 GitHub Releases，挑出该装哪个版本。

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::update::Reporter;
use crate::update::mirror;
use crate::update::net::{self, NetworkSettings};
use crate::update::version;

/// 本项目的仓库坐标。
pub const REPO: &str = "SuperQuail/SC2Miyin-Launcher";

/// Releases API（列全部，因为要按版本号自己挑最大，不能信 `latest`）。
fn releases_api() -> String {
    format!("https://api.github.com/repos/{REPO}/releases?per_page=30")
}

/// 项目主页。
pub fn homepage() -> String {
    format!("https://github.com/{REPO}")
}

/// 一个可下载的资产。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    pub download_url: String,
    /// GitHub 给的 `sha256:` 摘要；没有就是 `None`。
    #[serde(default)]
    pub sha256: Option<String>,
}

/// 一次发行。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// 去掉 `v` 前缀的版本号。
    pub version: String,
    pub tag: String,
    pub published_at: String,
    pub html_url: String,
    pub notes: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseInfo {
    /// 挑出适合当前平台的包：优先 `*-win64.zip`。
    pub fn platform_asset(&self) -> Option<&ReleaseAsset> {
        self.assets
            .iter()
            .find(|asset| asset.name.ends_with("-win64.zip"))
            .or_else(|| {
                self.assets
                    .iter()
                    .find(|asset| asset.name.ends_with(".zip"))
            })
    }
}

/// 检查结果。
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheck {
    /// 当前版本。
    pub current: String,
    /// 找到的最新版本；一个都没找到就是 `None`。
    pub latest: Option<ReleaseInfo>,
    /// 有没有更新。
    pub available: bool,
    /// 走的是什么网络路径（直连 / 某个代理），界面要如实显示。
    pub via: Option<String>,
    /// 查不到时的原因，供界面提示。
    pub error: Option<String>,
}

/// GitHub 的 release 载荷（只取我们要的字段）。
#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

/// 只保留 `sha256:<64 hex>`。
fn parse_digest(digest: Option<&str>) -> Option<String> {
    let hex = digest?.trim().strip_prefix("sha256:")?;
    (hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| hex.to_ascii_lowercase())
}

/// 把 GitHub 的载荷转成我们的模型。
fn to_release(dto: GhRelease) -> ReleaseInfo {
    ReleaseInfo {
        version: dto
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&dto.tag_name)
            .to_string(),
        tag: dto.tag_name,
        published_at: dto.published_at.unwrap_or_default(),
        html_url: dto.html_url,
        notes: dto.body.unwrap_or_default(),
        prerelease: dto.prerelease,
        assets: dto
            .assets
            .into_iter()
            .filter(|asset| !asset.name.is_empty())
            .map(|asset| ReleaseAsset {
                sha256: parse_digest(asset.digest.as_deref()),
                name: asset.name,
                size: asset.size,
                download_url: asset.browser_download_url,
            })
            .collect(),
    }
}

/// 从列表里挑出**版本号最大**的那个。
///
/// 不能直接用 `/releases/latest`，也不能取列表第一个 —— GitHub 是按**创建时间**
/// 排序的，补发的旧版本会排在前面（参考实现踩过这个坑）。
/// 认不出预发行段的 tag 会被跳过（`compare` 返回 None 的不参与排序）。
pub fn pick_latest(releases: Vec<ReleaseInfo>, include_prerelease: bool) -> Option<ReleaseInfo> {
    releases
        .into_iter()
        .filter(|release| include_prerelease || !release.prerelease)
        .filter(|release| version::parse(&release.version).is_some())
        .reduce(
            |best, candidate| match version::compare(&candidate.version, &best.version) {
                Some(std::cmp::Ordering::Greater) => candidate,
                _ => best,
            },
        )
}

/// 一次检查失败时的返回（网络问题不算错误，包起来给界面显示就行）。
fn failed(current: &str, via: Option<String>, reason: String) -> UpdateCheck {
    UpdateCheck {
        current: current.to_string(),
        latest: None,
        available: false,
        via,
        error: Some(reason),
    }
}

/// 拉**任意仓库**的发行列表 —— 给可选工具用（比如 SC2Diff）。
///
/// 与 `check()` 一样的两条底线：网络失败不算错误（返回 `Err(说明)` 让界面显示
/// "检查失败"）；代理被限流时自动改直连重试一次。
pub fn releases_for(
    repo: &str,
    settings: &NetworkSettings,
    reporter: Reporter<'_>,
) -> std::result::Result<Vec<ReleaseInfo>, String> {
    let api = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
    let proxy = net::detect_proxy(settings);

    let read =
        |proxy: &Option<net::DetectedProxy>| -> std::result::Result<String, net::HttpFailure> {
            let client = net::build_client(proxy.as_ref(), &user_agent()).map_err(|error| {
                net::HttpFailure {
                    message: error.to_string(),
                    rate_limited: false,
                }
            })?;
            net::get_text(&client, &api)
        };

    let body = match read(&proxy) {
        Ok(body) => body,
        Err(failure) if failure.rate_limited && proxy.is_some() => {
            reporter.say("代理被限流，改用直连重试…");
            read(&None).map_err(|error| error.message)?
        }
        Err(failure) => return Err(failure.message),
    };

    let parsed: Vec<GhRelease> =
        serde_json::from_str(&body).map_err(|error| format!("发行列表解析失败：{error}"))?;

    Ok(parsed
        .into_iter()
        .filter(|release| !release.draft)
        .map(to_release)
        .collect())
}

/// 拉一次发行列表。
fn fetch(
    proxy: &Option<net::DetectedProxy>,
    user_agent: &str,
) -> std::result::Result<String, net::HttpFailure> {
    let client =
        net::build_client(proxy.as_ref(), user_agent).map_err(|error| net::HttpFailure {
            message: error.to_string(),
            rate_limited: false,
        })?;
    net::get_text(&client, &releases_api())
}

/// 检查更新。
///
/// 两处刻意的设计：
///
/// 1. **网络失败不算错误** —— 包成 [`UpdateCheck::error`] 返回，界面显示"检查失败"就行，
///    不该因为一次网络抖动弹一堆红字。
/// 2. **代理被限流时自动改直连重试** —— GitHub 的 API 额度是按出口 IP 算的，
///    国内用的代理往往是共享 IP，额度早被别人用光（实测就差 403）。
///    直连用的是另一个 IP，多半还有额度，所以值得再试一次。
pub fn check(current: &str, settings: &NetworkSettings, reporter: Reporter<'_>) -> UpdateCheck {
    reporter.say(format!("开始检查更新（当前版本 {current}）"));

    let proxy = net::detect_proxy(settings);
    let via = proxy
        .as_ref()
        .map(|found| format!("{}（{}）", found.url, found.source));
    match &via {
        Some(via) => reporter.say(format!("网络：{via}")),
        None => reporter.say("网络：直连（没有检测到代理）"),
    }

    reporter.say("查询 GitHub Releases…");
    let body = match fetch(&proxy, &user_agent()) {
        Ok(body) => body,
        Err(failure) if failure.rate_limited && proxy.is_some() => {
            reporter.say(format!("代理被限流：{}", failure.message));
            reporter.say("改用直连重试…");
            match fetch(&None, &user_agent()) {
                Ok(body) => {
                    reporter.say("直连成功");
                    return parse_and_pick(
                        current,
                        &body,
                        Some("直连（代理被限流）".to_string()),
                        settings,
                        reporter,
                    );
                }
                Err(_) => {
                    reporter.say(format!("直连也失败了：{}", failure.message));
                    return failed(current, via, failure.message);
                }
            }
        }
        Err(failure) => {
            reporter.say(format!("查询失败：{}", failure.message));
            return failed(current, via, failure.message);
        }
    };

    parse_and_pick(current, &body, via, settings, reporter)
}

/// 解析响应并挑出版本。
fn parse_and_pick(
    current: &str,
    body: &str,
    via: Option<String>,
    settings: &NetworkSettings,
    reporter: Reporter<'_>,
) -> UpdateCheck {
    let parsed: Vec<GhRelease> = match serde_json::from_str(body) {
        Ok(parsed) => parsed,
        Err(error) => {
            reporter.say(format!("返回内容解析失败：{error}"));
            return failed(current, via, format!("GitHub 返回的内容解析失败：{error}"));
        }
    };

    let total = parsed.len();
    let latest = pick_latest(
        parsed
            .into_iter()
            .filter(|release| !release.draft)
            .map(to_release)
            .collect(),
        settings.include_prerelease,
    );

    reporter.say(format!(
        "读到 {total} 个发行版本（{}）",
        if settings.include_prerelease {
            "含预发行"
        } else {
            "不含预发行"
        }
    ));

    let available = latest
        .as_ref()
        .is_some_and(|release| version::is_newer(&release.version, current));

    match &latest {
        Some(release) if available => {
            reporter.say(format!(
                "发现新版本 {}（{}{}）",
                release.version,
                if release.prerelease {
                    "预发行，"
                } else {
                    ""
                },
                release.published_at.get(0..10).unwrap_or("")
            ));
            match release.platform_asset() {
                Some(asset) => reporter.say(format!(
                    "可下载：{}（{}）",
                    asset.name,
                    human_size(asset.size)
                )),
                None => reporter.say("这个发行版没有适合 Windows 的包"),
            }
        }
        Some(release) => reporter.say(format!(
            "已经是最新版本（仓库里最新是 {}）",
            release.version
        )),
        None => reporter.say("没有找到任何可用的发行版本"),
    }

    UpdateCheck {
        current: current.to_string(),
        latest,
        available,
        via,
        error: None,
    }
}

/// 把字节数说成人话。
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

/// 请求用的 User-Agent（GitHub API 要求带）。
pub fn user_agent() -> String {
    format!("SC2Miyin-Launcher/{}", env!("CARGO_PKG_VERSION"))
}

/// 下载更新包的候选 URL：GitHub 直连 + 镜像（如果开了加速）。
pub fn asset_urls(asset: &ReleaseAsset, settings: &NetworkSettings) -> Vec<String> {
    if !settings.use_mirrors || !mirror::is_github_url(&asset.download_url) {
        return vec![asset.download_url.clone()];
    }
    crate::update::mirror::build_mirror_urls(&asset.download_url, None)
}

/// 下载更新包到指定路径。
pub fn download(
    asset: &ReleaseAsset,
    settings: &NetworkSettings,
    destination: &std::path::Path,
    reporter: Reporter<'_>,
) -> Result<net::DownloadOutcome> {
    reporter.say(format!(
        "开始下载 {}（{}）",
        asset.name,
        human_size(asset.size)
    ));

    let proxy = net::detect_proxy(settings);
    let client = net::build_client(proxy.as_ref(), &user_agent())?;
    let urls = asset_urls(asset, settings);
    reporter.say(format!(
        "候选地址 {} 个（直连 + 镜像），并发竞速，谁先完成用谁",
        urls.len()
    ));

    let outcome = net::race_download(&client, &urls, destination, reporter)?;
    reporter.say(format!(
        "下载完成：{}，来自 {}",
        human_size(outcome.bytes),
        short_source(&outcome.url)
    ));

    // 有摘要就校验：镜像毕竟是第三方，下歪了要能发现
    if let Some(expected) = &asset.sha256 {
        reporter.say("校验 SHA-256…");
        let actual = sha256_file(destination)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(destination);
            reporter.say("校验失败，文件已删除");
            return Err(Error::PackageRejected(format!(
                "校验失败：期望 {expected}，实际 {actual}。文件已删除，请重试"
            )));
        }
        reporter.say("校验通过 ✓");
    } else {
        reporter.say("这个发行版没提供摘要，跳过校验");
    }

    Ok(outcome)
}

/// 把下载地址说短一点（镜像前缀很长，刷屏）。
fn short_source(url: &str) -> String {
    match url.split_once("https://") {
        Some((prefix, _)) if !prefix.is_empty() => {
            let host = prefix.trim_end_matches('/');
            format!("{host}（镜像）")
        }
        _ => "GitHub 直连".to_string(),
    }
}

/// 算文件的 SHA-256。
fn sha256_file(path: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 128 * 1024];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(version: &str, prerelease: bool) -> ReleaseInfo {
        ReleaseInfo {
            version: version.to_string(),
            tag: format!("v{version}"),
            published_at: String::new(),
            html_url: String::new(),
            notes: String::new(),
            prerelease,
            assets: Vec::new(),
        }
    }

    #[test]
    fn picks_the_highest_version_not_the_first() {
        // GitHub 按创建时间排序，补发的旧版本会排在前面
        let list = vec![
            release("0.1.0", false),
            release("0.3.0", false),
            release("0.2.0", false),
        ];
        assert_eq!(pick_latest(list, true).unwrap().version, "0.3.0");
    }

    #[test]
    fn prereleases_can_be_filtered_out() {
        let list = vec![release("0.2.0-alpha.1", true), release("0.1.0", false)];
        assert_eq!(pick_latest(list.clone(), false).unwrap().version, "0.1.0");
        assert_eq!(pick_latest(list, true).unwrap().version, "0.2.0-alpha.1");
    }

    #[test]
    fn skips_tags_that_cannot_be_parsed() {
        let list = vec![release("nightly", false), release("0.1.0", false)];
        assert_eq!(pick_latest(list, true).unwrap().version, "0.1.0");
    }

    #[test]
    fn none_when_nothing_usable() {
        assert!(pick_latest(vec![release("nightly", false)], true).is_none());
        assert!(pick_latest(Vec::new(), true).is_none());
    }

    #[test]
    fn prefers_the_win64_asset() {
        let mut info = release("0.2.0", false);
        info.assets = vec![
            ReleaseAsset {
                name: "source.zip".to_string(),
                size: 1,
                download_url: String::new(),
                sha256: None,
            },
            ReleaseAsset {
                name: "SC2Miyin-Launcher-0.2.0-win64.zip".to_string(),
                size: 2,
                download_url: String::new(),
                sha256: None,
            },
        ];
        assert_eq!(
            info.platform_asset().unwrap().name,
            "SC2Miyin-Launcher-0.2.0-win64.zip"
        );
    }

    #[test]
    fn digest_needs_the_sha256_prefix_and_64_hex() {
        let hex = "a".repeat(64);
        assert_eq!(parse_digest(Some(&format!("sha256:{hex}"))), Some(hex));
        assert_eq!(parse_digest(Some("md5:abc")), None);
        assert_eq!(parse_digest(Some("sha256:short")), None);
        assert_eq!(parse_digest(None), None);
    }

    #[test]
    fn mirrors_are_only_used_when_enabled() {
        let asset = ReleaseAsset {
            name: "a.zip".to_string(),
            size: 1,
            download_url: "https://github.com/o/r/releases/download/v1/a.zip".to_string(),
            sha256: None,
        };

        let with = NetworkSettings {
            use_mirrors: true,
            ..NetworkSettings::default()
        };
        assert!(asset_urls(&asset, &with).len() > 1);

        let without = NetworkSettings {
            use_mirrors: false,
            ..NetworkSettings::default()
        };
        assert_eq!(asset_urls(&asset, &without).len(), 1);
    }
}
