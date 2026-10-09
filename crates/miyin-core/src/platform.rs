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

/// 游戏是不是正跑着；跑着就返回进程名。
///
/// **切文件前必须问一句** —— Windows 上运行中的 exe / 被占用的地图删不掉、覆盖会失败，
/// 半途失败会留下「清单记了一半」的中间状态。宁可让用户先退游戏。
///
/// 用 `tasklist` 而不是 sysinfo：只为一个进程名拉一个依赖不值当，
/// 而 tasklist 是 Windows 自带的（`/FO CSV /NH` 的解析见下）。
#[cfg(windows)]
pub fn game_running() -> Option<String> {
    let mut command = std::process::Command::new("tasklist");
    command.args(["/FO", "CSV", "/NH"]);
    hide_console(&mut command);

    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    // CSV 每行形如 "SC2_x64.exe","1234","Console","1","1,234 K"
    // 只认游戏本体；StarCraft II.exe 是根启动器，开着不算（它不占 Maps/Mods）
    ["SC2_x64.exe", "SC2.exe"]
        .into_iter()
        .find(|name| text.contains(&format!("\"{name}\"")))
        .map(str::to_string)
}

/// 非 Windows 上没有这套判定 —— 返回 None，调用方当作「没在跑」。
#[cfg(not(windows))]
pub fn game_running() -> Option<String> {
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// 只验它能跑通、不 panic。**不断言结果** —— 测试机上游戏开着还是关着都可能。
    #[test]
    fn game_running_does_not_panic() {
        let _ = game_running();
    }
}
