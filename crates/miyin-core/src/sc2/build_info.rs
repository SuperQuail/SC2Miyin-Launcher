//! `.build.info` 解析。
//!
//! 星际争霸 II 安装根目录下的 `.build.info` 记录了当前安装的分支、版本号与 CDN
//! 信息，格式是「两行制管道分隔表」：首行为列定义，其后每行一条记录。
//!
//! ```text
//! Branch!STRING:0|Active!DEC:1|...|Version!STRING:0|Product!STRING:0
//! cn|1|...|5.0.16.97579|S2
//! ```
//!
//! 列定义形如 `名称!类型:长度`；值中若含 `|` 会用双引号包裹。

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};

/// `.build.info` 中一条记录的内容。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildInfo {
    fields: BTreeMap<String, String>,
}

impl BuildInfo {
    /// 解析 `.build.info` 文本，返回其中**激活**（`Active=1`）的那条记录。
    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text
            .lines()
            .map(|line| line.trim_start_matches('\u{feff}').trim())
            .filter(|line| !line.is_empty());

        let header = lines
            .next()
            .ok_or_else(|| Error::Parse(".build.info 为空".to_string()))?;

        let names: Vec<String> = split_row(header)
            .into_iter()
            .map(|cell| {
                cell.split('!')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
            .collect();

        let mut fallback: Option<BuildInfo> = None;

        for line in lines {
            let values = split_row(line);
            let fields: BTreeMap<String, String> = names
                .iter()
                .cloned()
                .zip(values.into_iter().map(|value| value.trim().to_string()))
                .collect();

            let record = BuildInfo { fields };
            if record.is_active() {
                return Ok(record);
            }
            fallback.get_or_insert(record);
        }

        fallback.ok_or_else(|| Error::Parse(".build.info 没有任何数据行".to_string()))
    }

    /// 从文件读取并解析。
    pub fn read(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::parse(&String::from_utf8_lossy(&bytes))
    }

    /// 按列名取值；空字符串视为「无」。
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .get(name)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    /// 安装分支，如 `cn` / `us` / `eu`。
    pub fn branch(&self) -> Option<&str> {
        self.get("Branch")
    }

    /// 完整版本号，如 `5.0.16.97579`。
    pub fn version(&self) -> Option<&str> {
        self.get("Version")
    }

    /// 构建号（版本号最后一段），如 `97579`。
    pub fn build_number(&self) -> Option<u32> {
        self.version()?.rsplit('.').next()?.parse().ok()
    }

    /// 产品代号。
    pub fn product(&self) -> Option<&str> {
        self.get("Product")
    }

    /// CDN 主机列表（空格分隔）。
    pub fn cdn_hosts(&self) -> Option<&str> {
        self.get("CDN Hosts")
    }

    /// 标记位，例如 `Windows code CN? acct-CHN?`。
    pub fn tags(&self) -> Option<&str> {
        self.get("Tags")
    }

    /// 是否为当前激活的记录。
    pub fn is_active(&self) -> bool {
        matches!(self.get("Active"), Some("1"))
    }
}

/// 按 `|` 切分一行，尊重双引号包裹的字段。
fn split_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '|' if !in_quotes => cells.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    cells.push(current);
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 取自开发机（国服 / 网易代理客户端）的真实样本。
    const SAMPLE: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|CDN Key!HEX:16|Install Key!HEX:16|IM Size!DEC:4|CDN Path!STRING:0|CDN Hosts!STRING:0|CDN Servers!STRING:0|Tags!STRING:0|Armadillo!STRING:0|Last Activated!STRING:0|Version!STRING:0|KeyRing!HEX:16|Product!STRING:0\n\
cn|1|0601979c3d36b9ea4d66a250191d8df5|85dc7fd5646db6c1c0425dd814d7409e|||tpr/sc2|blzdist-s2.necdn.leihuo.netease.com|http://blzdist-s2.necdn.leihuo.netease.com/?maxhosts=5 https://blzdist-s2.necdn.leihuo.netease.com/?maxhosts=5&fallback=1|Windows code CN? acct-CHN? geoip-CN? zhCN speech?:Windows code CN? acct-CHN? geoip-CN? zhCN text?|||5.0.16.97579||\n";

    #[test]
    fn parses_real_world_sample() {
        let info = BuildInfo::parse(SAMPLE).expect("应解析成功");
        assert_eq!(info.branch(), Some("cn"));
        assert_eq!(info.version(), Some("5.0.16.97579"));
        assert_eq!(info.build_number(), Some(97579));
        assert!(info.is_active());
        assert_eq!(
            info.cdn_hosts(),
            Some("blzdist-s2.necdn.leihuo.netease.com")
        );
        assert!(info.tags().is_some_and(|tags| tags.contains("zhCN")));
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(BuildInfo::parse("").is_err());
    }

    #[test]
    fn header_only_is_an_error() {
        assert!(BuildInfo::parse("Branch!STRING:0|Active!DEC:1\n").is_err());
    }

    #[test]
    fn picks_active_record_among_many() {
        let text = "Branch!STRING:0|Active!DEC:1|Version!STRING:0\n\
us|0|1.0.0.1\n\
cn|1|5.0.16.97579\n";
        let info = BuildInfo::parse(text).expect("应解析成功");
        assert_eq!(info.branch(), Some("cn"));
        assert_eq!(info.build_number(), Some(97579));
    }

    #[test]
    fn quoted_cells_keep_pipe() {
        let cells = split_row(r#"a|"b|c"|d"#);
        assert_eq!(cells, vec!["a", "b|c", "d"]);
    }
}
