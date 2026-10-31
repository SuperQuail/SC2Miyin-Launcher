//! 界面杂项：打开外链 / 在资源管理器里定位 / 封面图。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use std::path::{Path, PathBuf};

use tauri::State;

use crate::AppState;
use crate::platform::{open_in_browser, reveal_in_file_manager};

/// 用系统默认浏览器打开一个链接。
#[tauri::command(async)]
pub(crate) fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("只允许打开 http(s) 链接".to_string());
    }
    open_in_browser(&url).map_err(|error| error.to_string())
}

/// 在资源管理器中定位文件或目录。
#[tauri::command(async)]
pub(crate) fn reveal_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(format!("路径不存在：{path}"));
    }

    reveal_in_file_manager(&target)
}

/// 读取战役目录内的封面图，返回 data URL 供界面直接显示。
#[tauri::command(async)]
pub(crate) fn campaign_cover(dir: String) -> Option<String> {
    let dir = PathBuf::from(dir);
    let cover = miyin_core::library::store::find_cover(&dir).map(|found| dir.join(found))?;
    file_to_data_url(&cover)
}

/// 读取库里某个版本自带的封面图。
///
/// 返回 `None` 表示这个包没配封面，界面会退回该战役的官方美术。
#[tauri::command(async)]
pub(crate) fn variant_cover(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Option<String> {
    let relative = {
        let index = state.library.index();
        index
            .slots
            .get(&slot)?
            .variants
            .iter()
            .find(|variant| variant.id == variant_id)?
            .cover
            .clone()?
    };

    // 相对路径来自索引文件，落盘前仍然过一遍白名单校验
    let root = state.library.slot_dir(&slot).join(&variant_id);
    let target = miyin_core::safety::ensure_within(&root, &root.join(&relative)).ok()?;
    file_to_data_url(&target)
}

/// 把图片文件读成 data URL。
fn file_to_data_url(path: &Path) -> Option<String> {
    const MAX_BYTES: u64 = 6 * 1024 * 1024;

    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_BYTES {
        return None;
    }

    let bytes = std::fs::read(path).ok()?;
    let mime = match path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/jpeg",
    };

    Some(format!("data:{mime};base64,{}", base64_encode(&bytes)))
}

/// 极简 base64 编码（避免为此引入依赖）。
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let byte0 = chunk[0] as u32;
        let byte1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let byte2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (byte0 << 16) | (byte1 << 8) | byte2;

        out.push(TABLE[(triple >> 18) as usize & 0x3f] as char);
        out.push(TABLE[(triple >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }

    out
}
