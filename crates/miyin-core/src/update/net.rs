//! 网络层：代理自动选择 + 镜像竞速下载。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::update::Reporter;

/// 连接超时。**不设总超时** —— 国内下几十 MB 的包，总超时会把正常下载掐断。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// 单次读取的块大小。
const CHUNK: usize = 64 * 1024;

/// 代理模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// **自动**：环境变量 -> Windows 系统代理 -> 直连。
    #[default]
    Auto,
    /// 强制直连，不读任何系统设置。
    Off,
    /// 手动指定（用 [`NetworkSettings::proxy_url`]）。
    Manual,
}

/// 网络相关设置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    #[serde(default)]
    pub proxy_mode: ProxyMode,
    /// `Manual` 模式下用的代理地址，如 `http://127.0.0.1:7897`。
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// 是否允许走 GitHub 镜像加速。
    #[serde(default = "default_true")]
    pub use_mirrors: bool,
    /// 是否包含预发行版本。
    #[serde(default = "default_true")]
    pub include_prerelease: bool,
}

fn default_true() -> bool {
    true
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            proxy_mode: ProxyMode::Auto,
            proxy_url: None,
            use_mirrors: true,
            include_prerelease: true,
        }
    }
}

/// 自动探测到的代理地址。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectedProxy {
    pub url: String,
    /// 从哪来的，界面要如实告诉用户。
    pub source: &'static str,
}

/// 按设置挑一个代理；返回 `None` 表示直连。
///
/// 自动模式的顺序：**环境变量 -> Windows 系统代理 -> 直连**。
/// 环境变量优先是因为它更明确（用户或 CI 特意设的）。
pub fn detect_proxy(settings: &NetworkSettings) -> Option<DetectedProxy> {
    match settings.proxy_mode {
        ProxyMode::Off => None,
        ProxyMode::Manual => settings
            .proxy_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(|url| DetectedProxy {
                url: normalize_proxy(url),
                source: "手动设置",
            }),
        ProxyMode::Auto => from_environment().or_else(system_proxy),
    }
}

/// 环境变量里的代理。
fn from_environment() -> Option<DetectedProxy> {
    // 大小写都认：Windows 上两种写法都常见
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(DetectedProxy {
                    url: normalize_proxy(trimmed),
                    source: "环境变量",
                });
            }
        }
    }
    None
}

/// 把 `host:port` 补成 `http://host:port`。
fn normalize_proxy(raw: &str) -> String {
    if raw.contains("://") {
        raw.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", raw.trim_end_matches('/'))
    }
}

/// 读 Windows 系统代理（`Internet Settings`）。
#[cfg(windows)]
fn system_proxy() -> Option<DetectedProxy> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .ok()?;

    let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 {
        return None;
    }

    let server: String = key.get_value("ProxyServer").ok()?;
    let trimmed = server.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 可能是 "http=host:port;https=host:port" 这种分协议写法，取 https 那一段
    let picked = trimmed
        .split(';')
        .find_map(|part| part.trim().strip_prefix("https="))
        .or_else(|| {
            trimmed
                .split(';')
                .find_map(|part| part.trim().strip_prefix("http="))
        })
        .unwrap_or(trimmed);

    Some(DetectedProxy {
        url: normalize_proxy(picked),
        source: "Windows 系统代理",
    })
}

#[cfg(not(windows))]
fn system_proxy() -> Option<DetectedProxy> {
    None
}

/// 按代理设置建一个 HTTP 客户端。
pub fn build_client(
    proxy: Option<&DetectedProxy>,
    user_agent: &str,
) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .user_agent(user_agent)
        // 跟随 GitHub 的资产跳转（会跳到 objects.githubusercontent.com）
        .redirect(reqwest::redirect::Policy::limited(10))
        // **必须显式关掉** reqwest 的环境变量代理探测：代理只由我们统一决定。
        // 否则 `proxy = None`（"直连"/"关掉代理"/"代理被限流后改直连"）会被
        // `HTTPS_PROXY` 悄悄拉回代理上，那几条路径就全是假的。
        .no_proxy();

    if let Some(found) = proxy {
        builder = builder.proxy(
            reqwest::Proxy::all(&found.url)
                .map_err(|error| Error::Parse(format!("代理地址无法解析：{error}")))?,
        );
    }

    builder
        .build()
        .map_err(|error| Error::Parse(format!("HTTP 客户端创建失败：{error}")))
}

/// 竞速下载的结果。
#[derive(Debug, Clone, Serialize)]
pub struct DownloadOutcome {
    /// 最终用的那个 URL（镜像还是直连）。
    pub url: String,
    pub bytes: u64,
}

/// **并发竞速**下载：所有候选同时开跑，第一个成功的胜出，其余立刻放弃。
///
/// 失败信息会一并返回，方便界面告诉用户"直连超时、镜像 403"之类。
pub fn race_download(
    client: &reqwest::blocking::Client,
    urls: &[String],
    destination: &Path,
    reporter: Reporter<'_>,
) -> Result<DownloadOutcome> {
    if urls.is_empty() {
        return Err(Error::PackageRejected("没有可用的下载地址".to_string()));
    }

    // 第一个拿到完整内容的线程把它置为 true，其余在读取循环里看到就放弃
    let finished = AtomicBool::new(false);
    let mut failures: Vec<String> = Vec::new();

    let results: Vec<std::result::Result<DownloadOutcome, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = urls
            .iter()
            .enumerate()
            .map(|(index, url)| {
                let finished = &finished;
                scope.spawn(move || {
                    download_one(client, url, destination, index, finished, reporter)
                })
            })
            .collect();

        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .collect()
    });

    for result in results {
        match result {
            Ok(outcome) => return Ok(outcome),
            Err(message) => failures.push(message),
        }
    }

    Err(Error::PackageRejected(format!(
        "所有下载地址都失败了：{}",
        failures.join("；")
    )))
}

/// 单个候选的下载。
fn download_one(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &Path,
    index: usize,
    finished: &AtomicBool,
    reporter: Reporter<'_>,
) -> std::result::Result<DownloadOutcome, String> {
    let part = part_path(destination, index);

    let result = (|| -> std::result::Result<DownloadOutcome, String> {
        let mut response = client
            .get(url)
            .send()
            .map_err(|error| format!("{url} -> 请求失败: {error}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("{url} -> HTTP {status}"));
        }
        let total = response.content_length();

        let mut file = std::fs::File::create(&part)
            .map_err(|error| format!("{url} -> 建文件失败: {error}"))?;
        let mut buffer = vec![0u8; CHUNK];
        let mut written: u64 = 0;

        loop {
            // 别人已经下完了，别浪费带宽
            if finished.load(Ordering::Relaxed) {
                return Err(format!("{url} -> 慢了一步"));
            }

            let read = response
                .read(&mut buffer)
                .map_err(|error| format!("{url} -> 读取失败: {error}"))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])
                .map_err(|error| format!("{url} -> 写入失败: {error}"))?;
            written += read as u64;
            reporter.tell_progress(written, total);
        }

        file.flush()
            .map_err(|error| format!("{url} -> 落盘失败: {error}"))?;
        drop(file);

        if written == 0 {
            return Err(format!("{url} -> 下载到 0 字节"));
        }
        // 声明自己赢了；如果别人抢先，我们就退场
        if finished.swap(true, Ordering::SeqCst) {
            return Err(format!("{url} -> 慢了一步"));
        }

        std::fs::rename(&part, destination)
            .map_err(|error| format!("{url} -> 改名失败: {error}"))?;

        Ok(DownloadOutcome {
            url: url.to_string(),
            bytes: written,
        })
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// 每个候选写自己的临时文件，避免互相踩。
fn part_path(destination: &Path, index: usize) -> PathBuf {
    let mut name = destination.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".part{index}"));
    destination.with_file_name(name)
}

/// HTTP 失败的原因。
#[derive(Debug, Clone)]
pub struct HttpFailure {
    pub message: String,
    /// 403 / 429：多半是**额度或限流**，换条线路（比如直连）往往就好了。
    pub rate_limited: bool,
}

impl std::fmt::Display for HttpFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

/// 简单 GET，用于查询 API（不竞速：镜像基本不代理 `api.github.com`）。
pub fn get_text(
    client: &reqwest::blocking::Client,
    url: &str,
) -> std::result::Result<String, HttpFailure> {
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|error| HttpFailure {
            message: format!("请求失败：{error}"),
            rate_limited: false,
        })?;

    let status = response.status();
    if !status.is_success() {
        // 把 GitHub 给的原话带出来 —— 它会明说是不是限流、限的是哪个 IP，
        // 笼统来一句"失败了"根本没法排查。
        let remaining = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("-")
            .to_string();
        let body = response.text().unwrap_or_default();
        let reason = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("message")
                    .and_then(|message| message.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| body.chars().take(200).collect());

        let rate_limited = status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS;

        return Err(HttpFailure {
            message: if rate_limited {
                format!("HTTP {status} {reason}（剩余额度 {remaining}）")
            } else {
                format!("HTTP {status} {reason}")
            },
            rate_limited,
        });
    }

    response.text().map_err(|error| HttpFailure {
        message: format!("读取响应失败：{error}"),
        rate_limited: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_proxy_addresses() {
        assert_eq!(normalize_proxy("127.0.0.1:7897"), "http://127.0.0.1:7897");
        assert_eq!(
            normalize_proxy("http://127.0.0.1:7897/"),
            "http://127.0.0.1:7897"
        );
        assert_eq!(
            normalize_proxy("socks5://127.0.0.1:1080"),
            "socks5://127.0.0.1:1080"
        );
    }

    #[test]
    fn manual_mode_uses_the_given_url() {
        let settings = NetworkSettings {
            proxy_mode: ProxyMode::Manual,
            proxy_url: Some("127.0.0.1:7897".to_string()),
            ..NetworkSettings::default()
        };
        let found = detect_proxy(&settings).expect("应当探测到");
        assert_eq!(found.url, "http://127.0.0.1:7897");
        assert_eq!(found.source, "手动设置");
    }

    #[test]
    fn off_mode_never_returns_a_proxy() {
        let settings = NetworkSettings {
            proxy_mode: ProxyMode::Off,
            proxy_url: Some("http://127.0.0.1:7897".to_string()),
            ..NetworkSettings::default()
        };
        assert!(detect_proxy(&settings).is_none());
    }

    #[test]
    fn empty_manual_url_falls_back_to_direct() {
        let settings = NetworkSettings {
            proxy_mode: ProxyMode::Manual,
            proxy_url: Some("   ".to_string()),
            ..NetworkSettings::default()
        };
        assert!(detect_proxy(&settings).is_none());
    }

    #[test]
    fn part_paths_do_not_collide() {
        let dest = Path::new("C:/tmp/update.zip");
        assert_ne!(part_path(dest, 0), part_path(dest, 1));
        assert!(
            part_path(dest, 0)
                .to_string_lossy()
                .ends_with("update.zip.part0")
        );
    }
}
