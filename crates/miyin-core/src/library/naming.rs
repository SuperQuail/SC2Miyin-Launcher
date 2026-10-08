//! 元数据的**命名规则**：注册 ID 怎么拼、版本号怎么算合法。
//!
//! 规则**只在这里实现一份**。界面要实时提示（边打字边看），就通过命令回来问 ——
//! 本地 IPC 往返不到一毫秒，比在两边各写一套、然后慢慢漂移要好。

/// 把一段文字规范成可以当标识符用的形式。
///
/// - 转小写
/// - 非字母数字（含中文）的字符一律换成 `-`
/// - 首尾的 `-` 去掉，连续的合并成一个
///
/// 中文**保留** —— 玩家的战役名很多是中文，硬转拼音只会得到一串谁也认不出的东西。
pub fn slug(value: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;

    for ch in value.trim().chars() {
        if ch.is_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.extend(ch.to_lowercase());
        } else {
            pending_dash = true;
        }
    }

    out
}

/// 从**作者名**和**战役名**拼出注册 ID：`作者名.战役名`。
///
/// 用户不该被要求手写一长串注册名 —— 他们要填的本来就只有这两样东西，
/// 拼起来是启动器的事。
///
/// `@text
/// 作者 HTXL  +  战役 Golden LotV   ->  htxl.golden-lotv
/// 作者（空）  +  战役 群友之战        ->  qun-you-zhi-zhan?（见下）
/// `@
///
/// 缺作者就只用战役名（很多个人作品没写作者）。
pub fn registration_id(author: &str, name: &str) -> String {
    let left = slug(author);
    let right = slug(name);

    match (left.is_empty(), right.is_empty()) {
        (true, true) => String::new(),
        (true, false) => right,
        (false, true) => left,
        (false, false) => format!("{left}.{right}"),
    }
}

/// 版本号的问题在哪；`None` 表示没问题。
///
/// 宽松但有底线：必须有数字，分段之间只能有 `.` / `-`，不能有空格和乱七八糟的符号。
/// **不要求非得是三段** —— `1.2` 和 `1.2.3` 都是常见的写法。
pub fn version_error(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some("版本号不能为空".to_string());
    }
    if trimmed.chars().count() > 32 {
        return Some("版本号太长了（最多 32 个字符）".to_string());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Some("版本号里不能有空格".to_string());
    }

    // 允许一个 v 前缀，之后必须是数字开头
    let body = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);

    if !body.starts_with(|ch: char| ch.is_ascii_digit()) {
        return Some("版本号要以数字开头（例如 1.0 或 1.2.3）".to_string());
    }

    // 每段要么是纯数字，要么是 -xxx 这种后缀
    for part in body.split(['.', '-']) {
        if part.is_empty() {
            return Some("版本号里不能有连续的分隔符".to_string());
        }
        if !part.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return Some("版本号只能用字母、数字、. 和 -".to_string());
        }
    }

    None
}

/// 规范一下版本号：去掉首尾空格，去掉开头的 `v`。
pub fn normalize_version(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed)
        .to_string()
}

/// 界面实时校验的一次往返结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetadataCheck {
    /// 按当前作者名 / 战役名拼出来的注册 ID。
    pub registration_id: String,
    /// 版本号有没有问题。
    pub version_ok: bool,
    /// 有问题的话，问题是什么（给用户看的一句话）。
    pub version_message: Option<String>,
}

/// 算一次校验结果 —— 界面边打字边问。
pub fn check(author: &str, name: &str, version: &str) -> MetadataCheck {
    let error = version_error(version);
    MetadataCheck {
        registration_id: registration_id(author, name),
        version_ok: error.is_none(),
        version_message: error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_id_is_built_from_author_and_name() {
        assert_eq!(registration_id("HTXL", "Golden LotV"), "htxl.golden-lotv");
        assert_eq!(registration_id("htxl", "golden"), "htxl.golden");
        // 中文保留 —— 玩家的战役名大量是中文
        assert_eq!(registration_id("某人", "群友之战"), "某人.群友之战");
        // 没作者就只用战役名
        assert_eq!(registration_id("", "孤军奋战"), "孤军奋战");
        assert_eq!(registration_id("   ", "Solo"), "solo");
        // 两样都没有 -> 空
        assert_eq!(registration_id("", ""), "");
    }

    #[test]
    fn version_validation_catches_obvious_junk() {
        // 合法的常见写法
        for good in ["1.0", "1.2.3", "v1.2", "2", "1.0-beta", "0.9.1-rc2"] {
            assert!(version_error(good).is_none(), "{good} 应当合法");
        }

        // 明显有问题的
        for bad in ["", "   ", "abc", "1.0 final", "v", "1..2", "1.0!"] {
            assert!(version_error(bad).is_some(), "{bad} 应当被判为有问题");
        }
    }

    #[test]
    fn version_is_normalized_for_storage() {
        assert_eq!(normalize_version("  v1.2.3 "), "1.2.3");
        assert_eq!(normalize_version("1.0"), "1.0");
    }
}
