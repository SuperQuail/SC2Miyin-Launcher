//! GitHub 镜像列表与 URL 改写。
//!
//! 国内直连 `github.com` 下 release 资产经常几十 KB/s 甚至超时，
//! 所以下载时把原始 URL 展开成「直连 + 若干镜像」的候选列表，**并发竞速**，
//! 谁先成功用谁（见 [`crate::update::net::race_download`]）。
//!
//! 镜像前缀是 reverse-proxy 形态：完整原始 URL 直接拼在前缀后面。
//! 不是所有镜像都支持全部域名，竞速阶段失败的候选自然淘汰。

/// 内置候选前缀（不含末尾 `/`）。空串表示 **GitHub 直连**。
///
/// 排序有讲究：直连放第一（海外用户直接命中），国内镜像按实测速度从快到慢排。
pub const DEFAULT_MIRROR_PREFIXES: &[&str] = &[
    "",                        // 直连
    "https://gh.ddlc.top",     // ddlc
    "https://gh-proxy.com",    // gh-proxy
    "https://ghfast.top",      // ghfast
    "https://cors.isteed.cc",  // isteed
    "https://ghproxy.cc",      // ghproxy
    "https://github.akams.cn", // akams
];

/// 把原始 GitHub URL 展开成竞速用的候选列表。
///
/// `prefixes` 传 `None` 用 [`DEFAULT_MIRROR_PREFIXES`]。
pub fn build_mirror_urls(original: &str, prefixes: Option<&[&str]>) -> Vec<String> {
    let prefixes = prefixes.unwrap_or(DEFAULT_MIRROR_PREFIXES);
    prefixes
        .iter()
        .map(|prefix| {
            if prefix.is_empty() {
                original.to_string()
            } else {
                format!("{}/{}", prefix.trim_end_matches('/'), original)
            }
        })
        .collect()
}

/// 这条 URL 是不是 GitHub 的（只有 GitHub 的才值得走镜像）。
pub fn is_github_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("https://github.com/")
        || lower.starts_with("https://objects.githubusercontent.com/")
        || lower.starts_with("https://raw.githubusercontent.com/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_to_direct_plus_mirrors() {
        let urls = build_mirror_urls("https://github.com/o/r/releases/download/v1/a.zip", None);
        assert_eq!(urls.len(), DEFAULT_MIRROR_PREFIXES.len());
        assert_eq!(urls[0], "https://github.com/o/r/releases/download/v1/a.zip");
        assert!(urls[1].starts_with("https://gh.ddlc.top/https://github.com/"));
    }

    #[test]
    fn strips_trailing_slash_on_prefixes() {
        let urls = build_mirror_urls(
            "https://github.com/x.zip",
            Some(&["", "https://m.com/", "https://n.com"]),
        );
        assert_eq!(urls[0], "https://github.com/x.zip");
        assert_eq!(urls[1], "https://m.com/https://github.com/x.zip");
        assert_eq!(urls[2], "https://n.com/https://github.com/x.zip");
    }

    #[test]
    fn recognises_github_urls() {
        assert!(is_github_url("https://github.com/o/r/x.zip"));
        assert!(is_github_url("https://objects.githubusercontent.com/x"));
        assert!(!is_github_url("https://example.com/x"));
    }
}
