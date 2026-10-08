//! 平台相关的小工具。
//!
//! 目前只有一件：让子进程**不弹控制台窗口**。

use std::process::Command;

/// 让子进程不弹控制台窗口。
///
/// **这是踩出来的**：从 GUI 程序拉一个控制台程序（`tar`、`sc2diff`、`cmd`…）
/// 时，Windows 会顺手给它在屏幕上开一个黑框 —— 用户看到的就是「解压的时候
/// 闪了一下黑窗口」，观感很差，而且会抢焦点。
///
/// 加 `CREATE_NO_WINDOW` 就不开那个窗口了。非 Windows 平台是空操作。
///
/// ⚠️ **只对控制台程序用**。拉游戏本体、编辑器那种 GUI 程序时别加 ——
/// 那是人家自己的窗口，藏不得。
pub fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// 不创建控制台窗口。
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    {
        // 其它平台没有这个概念
        let _ = command;
    }
}
