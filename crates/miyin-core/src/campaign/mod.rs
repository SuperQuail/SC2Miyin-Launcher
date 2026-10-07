//! 战役（Custom Campaign）领域模型。
//!
//! 本模块只负责**模型与规则**，具体动作分散在：
//!
//! - [`metadata`]：CCM `metadata.txt` 与枢纽标准 `metadata.json` 的解析
//! - [`sanitize`]：目录名安全化（防目录穿越与非法名）
//! - [`package`]：zip 包预检（格式识别、zip-slip 校验、体积上限）
//! - [`installer`]：直接安装到游戏目录（旧路径，保留给"就地安装"场景）
//! - [`scanner`]：目录扫描与核对
//!
//! 注意：**战役库**（多版本共存与切换）在 [`crate::library`] 里，
//! 不在这里 —— 库属于启动器自身的数据，与游戏目录解耦。

pub mod contents;
pub mod identify;
pub mod installer;
pub mod metadata;
pub mod package;
pub mod sanitize;
pub mod scanner;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use metadata::CampaignType;

/// 战役包的来源格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CampaignFormat {
    /// CCM 自制战役包：包内任意层级有 `metadata.txt`。
    Ccm,
    /// 弥音/枢纽标准包：包根有 `metadata.json`。
    #[serde(rename = "miyin")]
    Standard,
    /// 没有元数据文件，仅靠目录内容识别。
    Plain,
    /// 无法识别。
    Unknown,
}

impl CampaignFormat {
    /// 面向用户的中文名称。
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ccm => "CCM 战役包",
            Self::Standard => "弥音标准包",
            Self::Plain => "无元数据",
            Self::Unknown => "未知格式",
        }
    }
}

/// 健康度等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthLevel {
    /// 一切正常。
    Ok,
    /// 可以运行，但有需要注意的地方。
    Warning,
    /// 无法正常运行。
    Broken,
}

/// 一条核对结论。
///
/// `code` 是稳定标识，便于界面做图标与文案映射；`message` 直接展示给用户。
#[derive(Debug, Clone, Serialize)]
pub struct HealthIssue {
    pub level: HealthLevel,
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
}

impl HealthIssue {
    /// 构造一条「需要注意」的结论。
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: HealthLevel::Warning,
            code: code.into(),
            message: message.into(),
            hint: None,
        }
    }

    /// 构造一条「无法运行」的结论。
    pub fn broken(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: HealthLevel::Broken,
            code: code.into(),
            message: message.into(),
            hint: None,
        }
    }

    /// 追加修复建议。
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// 一个已安装的战役。
#[derive(Debug, Clone, Serialize)]
pub struct Campaign {
    /// 稳定标识：安装目录名。
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// 封面图的本地绝对路径（若包内或同目录提供了图片）。
    pub cover: Option<String>,
    /// 安装目录。
    pub path: PathBuf,
    pub format: CampaignFormat,
    /// 归属资料片（来自元数据的 `campaign` 字段）。
    pub campaign_type: CampaignType,
    /// 是否已启用（地图已被复制进官方战役目录）。
    pub enabled: bool,
    pub health: HealthLevel,
    /// 核对出的问题清单；为空即为完全正常。
    pub issues: Vec<HealthIssue>,
    pub map_count: Option<usize>,
    pub size_bytes: Option<u64>,
}

impl Campaign {
    /// 依据问题清单重算健康度。
    pub fn recompute_health(&mut self) {
        self.health = derive_health(&self.issues);
    }
}

/// 由问题清单推导总体健康度。
pub fn derive_health(issues: &[HealthIssue]) -> HealthLevel {
    if issues
        .iter()
        .any(|issue| issue.level == HealthLevel::Broken)
    {
        HealthLevel::Broken
    } else if issues.is_empty() {
        HealthLevel::Ok
    } else {
        HealthLevel::Warning
    }
}
