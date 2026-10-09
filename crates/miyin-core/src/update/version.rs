//! 版本号比较。
//!
//! 只认 `major.minor.patch[-pre]` 这一种形态（GitHub tag 去掉 `v` 前缀之后的样子），
//! 认不出来就返回 `None` —— 由调用方决定怎么办，**不要瞎猜**。
//!
//! 预发行的排序遵循 semver：`0.2.0-alpha.1 < 0.2.0-alpha.2 < 0.2.0`。

use std::cmp::Ordering;

/// 解析出来的版本号。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// 预发行标识（`alpha.1` / `beta.2` / `rc.1`）；正式版是 `None`。
    pub pre: Option<String>,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| compare_pre(self.pre.as_deref(), other.pre.as_deref()))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 预发行段的比较：**没有预发行段的更大**（正式版优先）。
fn compare_pre(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => compare_pre_identifiers(a, b),
    }
}

/// 按 semver 的规则逐段比：数字段按数值比、数字段小于字母段、前缀相同则短的更小。
fn compare_pre_identifiers(left: &str, right: &str) -> Ordering {
    let mut a = left.split('.');
    let mut b = right.split('.');

    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ordering = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    // 数字标识符优先级低于字母标识符
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

/// 解析一个版本号；认不出来返回 `None`。
///
/// 两种写法都认，而且**等价**：
///
/// - semver：`0.1.0-alpha.2`（Cargo 只收这种）
/// - 紧凑：`0.1.0a2` / `0.1.0b1` / `0.1.0rc1`（我们对外的写法、git tag 用的就是它）
///
/// 也接受 `v` 前缀与构建元数据（`0.1.0+build.5`，按 semver 忽略）。
pub fn parse(raw: &str) -> Option<Version> {
    let trimmed = raw.trim();
    let without_v = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);

    // 构建元数据不参与比较
    let without_build = without_v.split('+').next().unwrap_or(without_v);

    let (core, hyphen_pre) = match without_build.split_once('-') {
        Some((core, pre)) => {
            let pre = pre.trim();
            if pre.is_empty() {
                return None;
            }
            (core, Some(pre.to_string()))
        }
        None => (without_build, None),
    };

    let mut parts = core.split('.');
    let major = parts.next()?.trim().parse().ok()?;
    let minor = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
    let patch_raw = parts.next().unwrap_or("0").trim();

    // 紧凑写法把预发行粘在 patch 后面（`0a2`）：拆出数字前缀与字母尾巴
    let (digits, tail) = split_numeric_prefix(patch_raw);
    let patch: u64 = digits.parse().unwrap_or(0);

    // 两种来源的预发行**归一成同一种写法**，否则 `a2` 与 `alpha.2` 会被当成两个版本。
    let pre = match (hyphen_pre, tail.is_empty()) {
        (Some(pre), _) => Some(normalize_pre(&pre)),
        (None, false) => Some(normalize_compact_pre(tail)?),
        (None, true) => None,
    };

    Some(Version {
        major,
        minor,
        patch,
        pre,
    })
}

/// 把 `0a2` 拆成 (`0`, `a2`)。
fn split_numeric_prefix(raw: &str) -> (&str, &str) {
    let end = raw
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(raw.len());
    raw.split_at(end)
}

/// `a2` -> `alpha.2`、`b1` -> `beta.1`、`rc1` -> `rc.1`。
///
/// 字母后面**必须**跟数字，否则不认（避免把 `alpha` 这种整词拆坏）。
fn normalize_compact_pre(tail: &str) -> Option<String> {
    let lower = tail.to_ascii_lowercase();
    for (prefix, label) in [("rc", "rc"), ("a", "alpha"), ("b", "beta")] {
        if let Some(rest) = lower.strip_prefix(prefix)
            && !rest.is_empty()
            && rest.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Some(format!("{label}.{rest}"));
        }
    }
    // 认不出来但确实有尾巴时，原样当作预发行段
    (!lower.is_empty()).then_some(lower)
}

/// 把 `alpha.2` 这类归一（`a.2` -> `alpha.2`）。
fn normalize_pre(pre: &str) -> String {
    match pre.split_once('.') {
        Some((label, rest)) => {
            let normalized = match label.to_ascii_lowercase().as_str() {
                "a" | "alpha" => "alpha",
                "b" | "beta" => "beta",
                "rc" => "rc",
                _ => label,
            };
            format!("{normalized}.{rest}")
        }
        None => pre.to_string(),
    }
}

/// 把 semver 形式压成我们对外用的紧凑写法：`0.1.0-alpha.2` -> `0.1.0a2`。
///
/// 正式版（没有 `-`）原样返回。
pub fn compact(version: &str) -> String {
    let stripped = version.strip_prefix('v').unwrap_or(version);
    let Some((core, pre)) = stripped.split_once('-') else {
        return stripped.to_string();
    };

    let mut segments = pre.split('.');
    let label = match segments.next().unwrap_or("").to_ascii_lowercase().as_str() {
        "alpha" | "a" => "a".to_string(),
        "beta" | "b" => "b".to_string(),
        "rc" => "rc".to_string(),
        other => other.to_string(),
    };
    let rest: String = segments.collect::<Vec<_>>().join("");

    format!("{core}{label}{rest}")
}

/// 比较两个版本字符串；任一边认不出来就返回 `None`。
pub fn compare(left: &str, right: &str) -> Option<Ordering> {
    Some(parse(left)?.cmp(&parse(right)?))
}

/// `candidate` 是不是比 `current` 新。认不出来一律返回 `false`。
///
/// **认不出来就当没有更新** —— 宁可让用户手动去看，也不要提示一个错误的"有新版本"。
pub fn is_newer(candidate: &str, current: &str) -> bool {
    matches!(compare(candidate, current), Some(Ordering::Greater))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_shapes() {
        assert_eq!(
            parse("v0.2.0"),
            Some(Version {
                major: 0,
                minor: 2,
                patch: 0,
                pre: None
            })
        );
        assert_eq!(
            parse("1.2.3-alpha.1"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3,
                pre: Some("alpha.1".to_string())
            })
        );
        // 构建元数据忽略
        assert_eq!(parse("1.2.3+build.5").unwrap().patch, 3);
        // 缺段补 0
        assert_eq!(parse("1.2").unwrap().patch, 0);
        assert_eq!(parse("1").unwrap().minor, 0);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse("v"), None);
        assert_eq!(parse("1.2.3-"), None);
        assert_eq!(compare("abc", "1.0.0"), None);
        // 认不出来就不认为是更新
        assert!(!is_newer("abc", "1.0.0"));
        assert!(!is_newer("2.0.0", "abc"));
    }

    #[test]
    fn compact_form_equals_the_semver_form() {
        // 对外用 0.1.0a2，Cargo 里写 0.1.0-alpha.2 —— 必须等价
        assert_eq!(parse("0.1.0a2"), parse("0.1.0-alpha.2"));
        assert_eq!(parse("0.1.0b1"), parse("0.1.0-beta.1"));
        assert_eq!(parse("0.1.0rc1"), parse("0.1.0-rc.1"));

        assert_eq!(
            parse("0.1.0a2"),
            Some(Version {
                major: 0,
                minor: 1,
                patch: 0,
                pre: Some("alpha.2".to_string())
            })
        );
    }

    #[test]
    fn compact_prereleases_compare_numerically() {
        // 这条如果不成立，a1 和 a2 会被当成同一版，永远检测不到更新
        assert!(is_newer("0.1.0a2", "0.1.0a1"));
        assert!(is_newer("0.1.0a10", "0.1.0a9"));
        assert!(is_newer("0.1.0b1", "0.1.0a9"));
        assert!(is_newer("0.1.0", "0.1.0a2"));
        assert!(!is_newer("0.1.0a1", "0.1.0a2"));
    }

    #[test]
    fn renders_the_compact_display_form() {
        assert_eq!(compact("0.1.0-alpha.2"), "0.1.0a2");
        assert_eq!(compact("0.1.0-beta.1"), "0.1.0b1");
        assert_eq!(compact("0.1.0-rc.3"), "0.1.0rc3");
        assert_eq!(compact("0.1.0"), "0.1.0");
        assert_eq!(compact("v0.1.0a2"), "0.1.0a2");
        // 已经是紧凑写法就原样
        assert_eq!(compact("0.1.0a2"), "0.1.0a2");
    }

    #[test]
    fn prerequisite_orders_before_release() {
        assert!(is_newer("0.2.0", "0.2.0-alpha.1"));
        assert!(is_newer("0.2.0-alpha.2", "0.2.0-alpha.1"));
        assert!(is_newer("0.2.0-beta.1", "0.2.0-alpha.9"));
        assert!(is_newer("0.3.0-alpha.1", "0.2.9"));
        assert!(!is_newer("0.2.0-alpha.1", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0-alpha.1"));
    }

    #[test]
    fn compares_numerically_not_lexically() {
        // 字符串比较会把 10 排在 9 前面
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert_eq!(compare("v1.2.3", "1.2.3"), Some(Ordering::Equal));
    }

    #[test]
    fn prerelease_identifiers_follow_semver() {
        // 前缀相同，短的更小
        assert!(is_newer("1.0.0-alpha.1", "1.0.0-alpha"));
        // 数字段小于字母段
        assert!(is_newer("1.0.0-alpha.beta", "1.0.0-alpha.1"));
    }
}
