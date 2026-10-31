//! 无边框窗口的标题栏按钮（最小化 / 最大化 / 关闭）。
//!
//! 命令名是 IPC 契约（前端 `ui/src/api/`），不要改。

/// 最小化窗口。
#[tauri::command(async)]
pub(crate) fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

/// 最大化 / 还原。
#[tauri::command(async)]
pub(crate) fn window_toggle_maximize(window: tauri::Window) -> Result<bool, String> {
    let maximized = window.is_maximized().map_err(|error| error.to_string())?;
    if maximized {
        window.unmaximize().map_err(|error| error.to_string())?;
    } else {
        window.maximize().map_err(|error| error.to_string())?;
    }
    Ok(!maximized)
}

/// 关窗口。
#[tauri::command(async)]
pub(crate) fn window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|error| error.to_string())
}

/// 现在是不是最大化 —— 标题栏的按钮图标要跟着变。
#[tauri::command(async)]
pub(crate) fn window_is_maximized(window: tauri::Window) -> Result<bool, String> {
    window.is_maximized().map_err(|error| error.to_string())
}
