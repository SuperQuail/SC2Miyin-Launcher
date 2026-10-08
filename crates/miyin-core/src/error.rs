//! 核心库统一错误类型。

use std::path::PathBuf;

/// 弥音启动器核心库的错误。
///
/// 面向用户的错误信息使用中文，便于直接展示在界面上。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 指定的目录存在，但不是有效的星际争霸 II 安装目录。
    #[error("不是一个有效的星际争霸 II 安装目录：{path}（缺少 {marker}）")]
    InvalidInstallation { path: PathBuf, marker: &'static str },

    /// 注册表与常见位置都没有找到游戏。
    #[error("没有找到星际争霸 II 安装，请手动指定游戏目录")]
    InstallationNotFound,

    /// 读取注册表失败。
    #[error("读取注册表失败：{0}")]
    Registry(String),

    /// 写操作目标越出了允许的根目录。
    #[error("路径越界：{target} 不在允许的根目录 {root} 内")]
    PathEscapesRoot { root: PathBuf, target: PathBuf },

    /// 路径无法被安全解析（例如全部祖先都不存在）。
    #[error("无法解析路径：{0}")]
    UnresolvablePath(PathBuf),

    /// 解析文本格式失败。
    #[error("解析失败：{0}")]
    Parse(String),

    /// 战役包未通过预检，拒绝安装。
    #[error("战役包被拒绝：{0}")]
    PackageRejected(String),

    /// 找不到指定的战役。
    #[error("找不到战役：{0}")]
    CampaignNotFound(String),

    /// 目标已存在且调用方要求不覆盖。
    #[error("目标已存在：{0}")]
    AlreadyExists(PathBuf),

    /// 底层 IO 错误。
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化 / 反序列化失败（索引、配置这些自家文件）。
    #[error("JSON 处理失败：{0}")]
    Json(#[from] serde_json::Error),

    /// 压缩包读写失败。
    #[error("压缩包处理失败：{0}")]
    Zip(#[from] zip::result::ZipError),
}

/// 核心库统一的结果类型。
pub type Result<T, E = Error> = std::result::Result<T, E>;
