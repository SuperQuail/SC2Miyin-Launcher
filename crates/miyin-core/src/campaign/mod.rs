//! 战役（Custom Campaign）领域模型。
//!
//! 本模块只负责**模型与规则**，具体动作分散在：
//!
//! - [`metadata`]：CCM `metadata.txt` 与枢纽标准 `metadata.json` 的解析
//! - [`sanitize`]：目录名安全化（防目录穿越与非法名）
//! - [`package`]：zip 包预检（格式识别、zip-slip 校验、体积上限）
//! - [`contents`]：任意打包形式的读取（zip / 7z / rar / tar）
//! - [`identify`]：没有元数据时按证据链判定归属
//!
//! 注意：**战役库**（多版本共存与切换）在 [`crate::library`] 里，
//! 不在这里 —— 库属于启动器自身的数据，与游戏目录解耦。
//! 曾经这里还有「扫描游戏目录里已装的战役」的一套模型，库做出来之后就没用了。

pub mod collect;
pub mod contents;
pub mod identify;
pub mod metadata;
pub mod package;
pub mod sanitize;

use serde::{Deserialize, Serialize};

pub use metadata::CampaignType;

/// 战役包的来源格式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CampaignFormat {
    /// CCM 自制战役包：包内任意层级有 `metadata.txt`。
    Ccm,
    /// 弥音/枢纽标准包：包根有 `metadata.json`。
    #[serde(rename = "miyin")]
    Standard,
    /// 没有元数据文件，仅靠目录内容识别。
    #[default]
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
