//! 写盘时遇到「文件被占用」的处置方式。
//!
//! 引擎只负责发现，判断留给调用方：桌面端弹框询问用户，无人值守的调用直接停下。

use std::path::Path;

use crate::error::Error;

/// 删除文件被占用时的处置方式。
///
/// 写盘引擎只负责「发现」，判断留给调用方：桌面端弹框询问用户，无人值守的调用
/// （启动时自动挂载、启动游戏前的同步）直接停下。
pub trait BusyDecision {
    /// 文件无法删除或替换时调用。返回 `true` = 跳过该项并继续；`false` = 停下。
    fn on_busy(&mut self, path: &Path, reason: &str) -> bool;
}

/// 停下并报错（默认）。
pub struct AbortOnBusy;

impl BusyDecision for AbortOnBusy {
    fn on_busy(&mut self, _path: &Path, _reason: &str) -> bool {
        false
    }
}

/// 跳过并继续（测试，以及「没有人可问但允许尽量做完」的场景）。
pub struct SkipOnBusy;

impl BusyDecision for SkipOnBusy {
    fn on_busy(&mut self, _path: &Path, _reason: &str) -> bool {
        true
    }
}

/// 这个错误是不是「文件被其他程序占用」。
pub(super) fn is_busy(error: &Error) -> bool {
    match error {
        Error::FileBusy { .. } => true,
        Error::Io(io) => crate::error::is_busy_io(io),
        _ => false,
    }
}

/// 把「被占用」的 IO 错误翻成 [`Error::FileBusy`]，其余原样返回。
pub(super) fn as_busy(error: Error, path: &Path) -> Error {
    match error {
        Error::Io(io) if crate::error::is_busy_io(&io) => Error::FileBusy {
            path: path.to_path_buf(),
            reason: format!("文件正被其他程序占用（{io}）"),
        },
        other => other,
    }
}

/// 从「被占用」的错误里取出给用户看的原因。
pub(super) fn busy_reason(error: &Error) -> String {
    match error {
        Error::FileBusy { reason, .. } => reason.clone(),
        Error::Io(io) => format!("文件正被其他程序占用（{io}）"),
        other => other.to_string(),
    }
}

/// 跳过一项时给用户的提示（书面语：原因 + 下一步）。
pub(super) fn skipped(action: &str, target: &str) -> String {
    format!(
        "无法{action}「{target}」：文件正被其他程序占用，本次操作已跳过该项。请关闭占用该文件的程序后重试。"
    )
}
