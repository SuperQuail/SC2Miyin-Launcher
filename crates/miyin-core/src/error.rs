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

    /// 压缩包加了密，需要密码才能读。
    ///
    /// 里面那句话是给用户看的（不同打包格式能给的提示不一样，
    /// 例如第三方格式会提示「先用 7-Zip 解开再导入」）。
    #[error("{0}")]
    PasswordRequired(String),

    /// 密码不对。
    ///
    /// 也可能意味着包已经损坏 —— 少数加密方式（ZipCrypto）允许错误的密码
    /// "打开"条目，到解压 / 校验那一步才炸；两种都该让用户换个密码或重新下载。
    #[error("压缩包密码不对（也可能是这个包已经损坏），请重新输入")]
    WrongPassword,

    /// 找不到指定的战役。
    #[error("找不到战役：{0}")]
    CampaignNotFound(String),

    /// 目标已存在且调用方要求不覆盖。
    #[error("目标已存在：{0}")]
    AlreadyExists(PathBuf),

    /// 激活 / 铺盘中途失败，已经把我们这次铺的一半回滚掉。
    ///
    /// 用户看到的必须是「失败 + 已回到原版」，而不是半套状态。
    #[error("安装失败，已回滚到原版状态：{0}")]
    InstallRolledBack(String),

    /// 清单和磁盘对不上：清单里记着某个位置，反查时却落不到。
    #[error("清单和磁盘对不上：{0}")]
    ManifestMismatch(String),

    /// 该写的东西被系统拒绝了 —— 需要管理员权限。
    ///
    /// Display 固定以 `[PERMISSION] ` 开头：界面据此直接走提权 / 只读横幅，
    /// 而不是去猜「os error 5」这类字符串（见 docs/design/storage-and-elevation.md §4）。
    #[error("[PERMISSION] {0}")]
    PermissionDenied(String),

    /// 底层 IO 错误。
    ///
    /// Windows 的「拒绝访问」（os error 5 / PermissionDenied）归入权限错误：
    /// 展示文本同样以 `[PERMISSION] ` 开头，后面接一句排查提示。
    #[error("{}", io_message(.0))]
    Io(#[from] std::io::Error),

    /// JSON 序列化 / 反序列化失败（索引、配置这些自家文件）。
    #[error("JSON 处理失败：{0}")]
    Json(#[from] serde_json::Error),

    /// 压缩包读写失败。
    #[error("压缩包处理失败：{0}")]
    Zip(#[from] zip::result::ZipError),
}

impl Error {
    /// 是不是「需要管理员权限」这一类错误 —— 界面据此提权，而不是当成普通失败。
    ///
    /// 显式构造的 [`Error::PermissionDenied`] 与 IO 层真正的「拒绝访问」都算。
    pub fn is_permission_denied(&self) -> bool {
        match self {
            Self::PermissionDenied(_) => true,
            Self::Io(error) => is_denied(error),
            _ => false,
        }
    }
}

/// 核心库统一的结果类型。
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Windows「拒绝访问」时追加在原错误后面的排查提示。
const ACCESS_DENIED_HINT: &str = "。请确认是否将星际争霸游戏目录安装在了 C 盘，如果是，请尝试使用管理员权限运行启动器；如果不是，并且使用管理员权限运行依旧失败，请尝试关闭杀毒软件或设置白名单";

/// 是不是「拒绝访问」：标准库的 kind，或 Windows 上裸的 os error 5。
fn is_denied(error: &std::io::Error) -> bool {
    // Linux 上 os error 5 是 EIO 不是拒绝访问，裸错误码只在 Windows 认
    error.kind() == std::io::ErrorKind::PermissionDenied
        || (cfg!(windows) && error.raw_os_error() == Some(5))
}

/// IO 错误的展示文本。
///
/// 普通错误加「IO 错误：」前缀；拒绝访问换成 `[PERMISSION] ` 前缀，再接一句排查提示。
/// 只认 `ERROR_ACCESS_DENIED`（5）与 `PermissionDenied` —— 找不到文件不是权限问题，
/// 套这句只会误导，也不该让界面弹提权。
fn io_message(error: &std::io::Error) -> String {
    if is_denied(error) {
        format!("[PERMISSION] {error}{ACCESS_DENIED_HINT}")
    } else {
        format!("IO 错误：{error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 显式构造的权限错误固定带 `[PERMISSION] ` 前缀 —— 界面靠它认，不靠 os error 字符串。
    #[test]
    fn permission_denied_has_the_prefix() {
        let error = Error::PermissionDenied("数据根写不进去".to_string());

        assert!(error.is_permission_denied());
        assert!(
            error.to_string().starts_with("[PERMISSION] "),
            "前缀要在最前面：{error}"
        );
        assert!(
            error.to_string().contains("数据根写不进去"),
            "原因要保留：{error}"
        );
    }

    /// 普通 IO 错误不算权限问题，也不带前缀。
    #[test]
    fn plain_io_errors_are_not_permission_denied() {
        let error = Error::from(std::io::Error::from_raw_os_error(2));

        assert!(!error.is_permission_denied());
        let text = error.to_string();
        assert!(!text.starts_with("[PERMISSION] "), "别乱加前缀：{text}");
        assert!(!text.contains("C 盘"), "别的 IO 错误不许套排查提示：{text}");
        assert!(text.contains("IO 错误："), "{text}");
    }

    /// 别的错误类型同样不是权限问题。
    #[test]
    fn other_errors_are_not_permission_denied() {
        assert!(!Error::Parse("坏文件".to_string()).is_permission_denied());
        assert!(!Error::WrongPassword.is_permission_denied());
    }

    /// IO 层的「拒绝访问」按权限错误走：前缀 + 排查提示在后面。
    #[test]
    fn io_permission_denied_gets_the_prefix_and_hint() {
        let error = Error::from(std::io::Error::from(std::io::ErrorKind::PermissionDenied));

        assert!(error.is_permission_denied());
        let text = error.to_string();
        assert!(text.starts_with("[PERMISSION] "), "前缀要在最前面：{text}");
        let hint = text.find("C 盘").expect("要带 C 盘那句排查提示");
        assert!(
            hint > "[PERMISSION] ".len(),
            "提示要追加在原错误之后：{text}"
        );
        assert!(text.contains("管理员权限"), "提示要完整：{text}");
        assert!(text.contains("白名单"), "提示要完整：{text}");
    }

    /// Windows 上真的 `ERROR_ACCESS_DENIED` 走同一条路。
    #[cfg(windows)]
    #[test]
    fn windows_access_denied_is_permission_denied() {
        let error = Error::from(std::io::Error::from_raw_os_error(5));

        assert!(error.is_permission_denied());
        let text = error.to_string();
        assert!(text.starts_with("[PERMISSION] "), "{text}");
        assert!(text.contains("C 盘"), "os error 5 要带排查提示：{text}");
    }
}
