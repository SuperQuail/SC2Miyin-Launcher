//! 目录名安全化。
//!
//! **这是本项目的安全命门。**
//!
//! 参考实现（星际枢纽）直接拿包里的 `title` 当落地目录名，只过滤了
//! `~!@#$%^*"|:<>/\\` 而**没有过滤点**，于是 `title=..` 会让
//! `join(CustomCampaigns, "..")` 得到整个 `Maps` 目录，紧接着被递归删除。
//! 证据：`reference/scnexus/packages/app-main/src/modules/campaign/ccm-process.ts:43-48`。
//!
//! 本模块的目标：**任何**来自外部的字符串，经过 [`sanitize_dir_name`] 之后
//! 一定是一个普通的、单层的、合法的 Windows 目录名，或者被明确拒绝。

/// Windows 保留设备名（作为文件名时不可用，不区分大小写）。
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Windows 文件名非法字符。
const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// 目录名长度上限。
///
/// 取 100 是为了给 `Maps\\CustomCampaigns\\<名字>\\Maps\\<地图名>` 这类深层
/// 路径留出余量，避免 Windows 260 字符上限带来的 `ENOENT`。
pub const MAX_DIR_NAME_LEN: usize = 100;

/// 把任意外部输入转换为安全的单层目录名。
///
/// 返回 `None` 表示输入完全不可用（空、纯空白、纯点、Windows 保留名，
/// 或过滤后什么都不剩），调用方**必须**换一个名字而不是硬着头皮用。
pub fn sanitize_dir_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();

    // 最关键的拒绝：任何形式的父目录引用
    if trimmed.is_empty() || is_dot_like(trimmed) {
        return None;
    }

    let mut cleaned: String = trimmed
        .chars()
        .map(|ch| {
            if ch.is_control() || ILLEGAL_CHARS.contains(&ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();

    // Windows 会静默丢弃结尾的点和空格。这里显式处理，否则 "foo." 与 "foo"
    // 会指向同一个目录，造成难以排查的互相覆盖。
    while cleaned.ends_with('.') || cleaned.ends_with(' ') {
        cleaned.pop();
    }

    let cleaned = cleaned.trim_start().to_string();

    if cleaned.is_empty() || is_dot_like(&cleaned) || is_reserved(&cleaned) {
        return None;
    }

    Some(truncate_chars(&cleaned, MAX_DIR_NAME_LEN))
}

/// 在原始名不可用或重名时，生成一个带后缀的备选目录名。
pub fn with_suffix(base: &str, suffix: &str) -> Option<String> {
    let stem = sanitize_dir_name(base)?;
    let suffix = sanitize_dir_name(suffix).unwrap_or_else(|| "pkg".to_string());
    let keep = MAX_DIR_NAME_LEN
        .saturating_sub(suffix.chars().count() + 2)
        .max(1);
    Some(format!("{} #{}", truncate_chars(&stem, keep), suffix))
}

/// 是否为纯点串（`.` / `..` / `...`）。
fn is_dot_like(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch == '.')
}

/// 是否为 Windows 保留设备名（`CON`、`con.txt` 都算）。
fn is_reserved(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or(value).trim();
    RESERVED_NAMES
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// 按字符（而非字节）截断，避免把中文切坏。
fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value
        .chars()
        .take(max)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_directory_references() {
        // 这一条正是参考实现会拿来当目录名的输入，后果是删除整个 Maps
        assert_eq!(sanitize_dir_name(".."), None);
        assert_eq!(sanitize_dir_name("."), None);
        assert_eq!(sanitize_dir_name("..."), None);
        assert_eq!(sanitize_dir_name("  ..  "), None);
    }

    #[test]
    fn rejects_empty_and_blank() {
        assert_eq!(sanitize_dir_name(""), None);
        assert_eq!(sanitize_dir_name("   "), None);
    }

    #[test]
    fn rejects_windows_reserved_names() {
        assert_eq!(sanitize_dir_name("CON"), None);
        assert_eq!(sanitize_dir_name("nul"), None);
        assert_eq!(sanitize_dir_name("Com1.txt"), None);
        assert_eq!(sanitize_dir_name("Console"), Some("Console".to_string()));
    }

    #[test]
    fn replaces_illegal_characters() {
        assert_eq!(
            sanitize_dir_name("Wings of Liberty: Reborn"),
            Some("Wings of Liberty_ Reborn".to_string())
        );
        assert_eq!(sanitize_dir_name("a/b\\c"), Some("a_b_c".to_string()));
    }

    #[test]
    fn strips_trailing_dots_and_spaces() {
        assert_eq!(sanitize_dir_name("Campaign."), Some("Campaign".to_string()));
        assert_eq!(
            sanitize_dir_name("Campaign . "),
            Some("Campaign".to_string())
        );
    }

    #[test]
    fn keeps_chinese_names() {
        assert_eq!(
            sanitize_dir_name("星际争霸：重制战役"),
            Some("星际争霸：重制战役".to_string())
        );
    }

    #[test]
    fn truncates_long_names_by_chars() {
        let long = "收".repeat(300);
        let result = sanitize_dir_name(&long).expect("应可用");
        assert_eq!(result.chars().count(), MAX_DIR_NAME_LEN);
    }

    #[test]
    fn sanitized_name_never_escapes_its_parent() {
        for raw in [
            "..",
            "../..",
            "a/../../b",
            "C:\\Windows",
            "....",
            " x ",
            "//",
        ] {
            if let Some(name) = sanitize_dir_name(raw) {
                assert!(!name.contains('/'));
                assert!(!name.contains('\\'));
                assert!(!is_dot_like(&name));
                assert_ne!(name, "..");
            }
        }
    }
}
