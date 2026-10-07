//! 战役元数据解析。
//!
//! 支持两种格式：
//!
//! | 格式 | 元数据文件 | 位置 | 语法 |
//! | --- | --- | --- | --- |
//! | CCM | `metadata.txt` | 包内**任意层级** | `键=值` 文本 |
//! | 弥音标准 | `metadata.json` | **包根** | JSON |
//!
//! # 相比参考实现修正了什么
//!
//! 1. **键名大小写不敏感**。上游 CCM 用 `ToLower()` 比较，而枢纽用字面量比较，
//!    于是 `Title=` 的包解析出空名字 —— 进而触发目录穿越删除（见 `sanitize` 模块注释）。
//! 2. **没有 `=` 的行被跳过**，而不是取 `undefined` 再抛 `TypeError`。
//! 3. **值按第一个 `=` 切分**，值里的 `=` 不会被截断。
//! 4. **编码探测**：优先 UTF-8，失败退回 GBK，中文包的标题不再乱码。
//! 5. **`campaign` 字段容错匹配**：`wings` / `liberty` / `swarm` 等写法都能识别。

use serde::{Deserialize, Serialize};

/// 把元数据字节解码为文本。
///
/// 先按 UTF-8 解码；失败则按 GBK 解码（国服中文包的常见编码）。
pub fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.trim_start_matches('\u{feff}').to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
            decoded.trim_start_matches('\u{feff}').to_string()
        }
    }
}

/// CCM 包元数据（来自 `metadata.txt`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CcmMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub campaign: Option<String>,
    pub version: Option<String>,
}

impl CcmMetadata {
    /// 解析 `metadata.txt` 文本。
    ///
    /// 该函数**不会失败**：畸形行一律跳过，最坏情况返回全空的元数据。
    pub fn parse(text: &str) -> Self {
        // 直接调用本函数时文本可能带 BOM。U+FEFF 并非空白字符，
        // trim() 不会去掉它，必须显式剥离，否则第一行的键名永远匹配不上。
        let text = text.trim_start_matches('\u{feff}');
        let mut meta = Self::default();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // 没有等号的行直接跳过（参考实现在这里会抛 TypeError）
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            match key.trim().to_ascii_lowercase().as_str() {
                "title" | "name" => meta.title = Some(value.to_string()),
                "desc" | "description" => meta.description = Some(value.to_string()),
                "author" => meta.author = Some(value.to_string()),
                "campaign" => meta.campaign = Some(value.to_string()),
                "version" => meta.version = Some(value.to_string()),
                _ => {}
            }
        }

        meta
    }

    /// 是否一个字段都没解析到。
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.author.is_none()
            && self.campaign.is_none()
            && self.version.is_none()
    }
}

/// 弥音/枢纽标准包元数据（来自包根的 `metadata.json`）。
///
/// 所有字段都是可选的：标准里标了「必填」的字段在现实中经常缺失，
/// 解析层不该因为缺字段就整体失败（参考实现把可选字段当必填，导致激活崩溃）。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StandardMetadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// 包类型，标准包中 Campaign 才是战役。
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub campaign: Option<String>,
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub maps_directory: Option<String>,
    #[serde(default)]
    pub mods_directory: Option<String>,
}

impl StandardMetadata {
    /// 解析 `metadata.json` 文本。
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// 战役归属的资料片。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CampaignType {
    Wol,
    Hots,
    HotsEvolution,
    Lotv,
    LotvPrologue,
    Nova,
    /// 未识别（或缺失）的取值，原样保留便于排查。
    Other(String),
}

impl CampaignType {
    /// 容错解析 `campaign` 字段。
    ///
    /// 参考实现要求严格等于 `WOL` / `HOTS` / `LOTV` / `NCO` 之一，
    /// 于是 `campaign=wings` 的包「装上了却扫不到」。
    pub fn parse(raw: &str) -> Self {
        let value = raw.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Self::Other(String::new());
        }

        // 顺序要紧：先匹配更具体的取值
        if value.contains("evolut") || value.contains("hotsevo") {
            Self::HotsEvolution
        } else if value.contains("prolog") {
            Self::LotvPrologue
        } else if value.contains("wol") || value.contains("wings") || value.contains("liberty") {
            Self::Wol
        } else if value.contains("hots") || value.contains("swarm") || value.contains("kerrigan") {
            Self::Hots
        } else if value.contains("lotv") || value.contains("void") || value.contains("legacy") {
            Self::Lotv
        } else if value.contains("nco") || value.contains("nova") {
            Self::Nova
        } else {
            Self::Other(raw.trim().to_string())
        }
    }

    /// 启用时地图应复制到的 `Maps/Campaign` 子目录。
    ///
    /// 返回 `None` 表示直接放在 `Maps/Campaign` 根下（自由之翼）。
    pub fn sub_directory(&self) -> Option<&'static str> {
        match self {
            Self::Wol => None,
            Self::Hots => Some("swarm"),
            Self::HotsEvolution => Some("swarm/evolution"),
            Self::Lotv => Some("void"),
            Self::LotvPrologue => Some("voidprologue"),
            Self::Nova => Some("nova"),
            Self::Other(_) => None,
        }
    }

    /// 是否可以启用（需要能确定归属资料片）。
    pub fn is_actionable(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// 面向用户的资料片名称。
    pub fn display_name(&self) -> String {
        match self {
            Self::Wol => "自由之翼".to_string(),
            Self::Hots => "虫群之心".to_string(),
            Self::HotsEvolution => "虫群之心 · 进化".to_string(),
            Self::Lotv => "虚空之遗".to_string(),
            Self::LotvPrologue => "虚空之遗 · 序章".to_string(),
            Self::Nova => "诺娃隐秘行动".to_string(),
            Self::Other(raw) if raw.is_empty() => "未标注资料片".to_string(),
            Self::Other(raw) => format!("未知资料片（{raw}）"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ccm_keys_are_case_insensitive() {
        // 参考实现在这里会得到空名字，进而触发目录穿越
        let meta = CcmMetadata::parse("Title=Reborn\nAUTHOR=SomeCreator\nCampaign=WOL\n");
        assert_eq!(meta.title.as_deref(), Some("Reborn"));
        assert_eq!(meta.author.as_deref(), Some("SomeCreator"));
        assert_eq!(meta.campaign.as_deref(), Some("WOL"));
    }

    #[test]
    fn ccm_skips_lines_without_equals_sign() {
        // 参考实现取 lineData[1] 得到 undefined，随后 .replace 抛 TypeError
        let meta = CcmMetadata::parse("title\njust some noise\nversion=1.0\n");
        assert_eq!(meta.title, None);
        assert_eq!(meta.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn ccm_keeps_equals_sign_inside_value() {
        let meta = CcmMetadata::parse("desc=Score = 10, Lives = 3\n");
        assert_eq!(meta.description.as_deref(), Some("Score = 10, Lives = 3"));
    }

    #[test]
    fn ccm_handles_crlf_and_bom() {
        let meta = CcmMetadata::parse("\u{feff}title=重制版自由之翼\r\ndesc=含 32 张关卡\r\n");
        assert_eq!(meta.title.as_deref(), Some("重制版自由之翼"));
        assert_eq!(meta.description.as_deref(), Some("含 32 张关卡"));
    }

    #[test]
    fn decode_text_falls_back_to_gbk() {
        let (bytes, _, _) = encoding_rs::GBK.encode("title=星际争霸：重制战役");
        let decoded = decode_text(&bytes);
        assert_eq!(decoded, "title=星际争霸：重制战役");

        let meta = CcmMetadata::parse(&decoded);
        assert_eq!(meta.title.as_deref(), Some("星际争霸：重制战役"));
    }

    #[test]
    fn campaign_type_tolerates_real_world_spellings() {
        assert_eq!(CampaignType::parse("WOL"), CampaignType::Wol);
        assert_eq!(CampaignType::parse("wings"), CampaignType::Wol);
        assert_eq!(CampaignType::parse("swarm"), CampaignType::Hots);
        assert_eq!(
            CampaignType::parse("swarm/evolution"),
            CampaignType::HotsEvolution
        );
        assert_eq!(
            CampaignType::parse("lotvprologue"),
            CampaignType::LotvPrologue
        );
        assert_eq!(CampaignType::parse("nova"), CampaignType::Nova);
        assert!(matches!(
            CampaignType::parse("something-else"),
            CampaignType::Other(_)
        ));
        assert!(matches!(CampaignType::parse(""), CampaignType::Other(_)));
    }

    #[test]
    fn campaign_type_maps_to_expected_sub_directory() {
        assert_eq!(CampaignType::Wol.sub_directory(), None);
        assert_eq!(CampaignType::Hots.sub_directory(), Some("swarm"));
        assert_eq!(CampaignType::Lotv.sub_directory(), Some("void"));
        assert_eq!(CampaignType::Nova.sub_directory(), Some("nova"));
        assert!(!CampaignType::Other("x".into()).is_actionable());
    }

    #[test]
    fn standard_metadata_tolerates_missing_fields() {
        let meta = StandardMetadata::parse(r#"{"name":"A","author":"B"}"#).expect("应解析成功");
        assert_eq!(meta.name.as_deref(), Some("A"));
        assert_eq!(meta.version, None);
        assert_eq!(meta.maps_directory, None);
    }
}
