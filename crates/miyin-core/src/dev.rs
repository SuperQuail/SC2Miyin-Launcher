//! 开发者页（IDE）要用的**只读**接口：扫游戏目录、给文件做个打开预览。
//!
//! 这一层刻意很薄：它不写盘、不改库、不碰清单。写盘的活还是走
//! [`crate::library::install`] 那套统一引擎 —— 开发者页现在只是**看**。
//!
//! 界面侧是独立的 React 入口（见 AGENTS.md §18），靠下面这些结构体说话。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::sc2::Installation;

/// 一次最多列这么多条 —— 游戏目录能塞十万个地图，界面撑不住。
pub const MAX_ENTRIES: usize = 4000;

/// 文本预览最多读这么多字节，超了就只给前面一段。
pub const MAX_TEXT_BYTES: usize = 512 * 1024;

/// 扫出来的一条。
#[derive(Debug, Clone, Serialize)]
pub struct DevEntry {
    /// 相对游戏目录的路径（`/` 分隔）；自定义目录用它自己的绝对路径。
    pub path: String,
    /// 绝对路径 —— 编辑器拿它去读文件。
    pub abs: String,
    /// 最后一段，界面直接显示。
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
    /// 不在游戏目录里 —— 用户自己加的目录。界面要标出来。
    pub external: bool,
}

/// 扫描结果。
#[derive(Debug, Clone, Serialize)]
pub struct DevScan {
    pub entries: Vec<DevEntry>,
    /// 被 [`MAX_ENTRIES`] 截断了吗。
    pub truncated: bool,
    /// 列了几个自定义目录。
    pub external_roots: Vec<String>,
}

/// 扫内容区（`Maps` / `Mods` / `Interfaces`）+ 用户自己加的目录。
///
/// `extra` 是自定义目录的绝对路径。**这里不做白名单校验** —— 开发者页
/// 就是要看游戏目录外面的东西；越界由界面标红提示，写盘时才由安装引擎拦。
pub fn scan(installation: &Installation, extra: &[String]) -> Result<DevScan> {
    let mut entries = Vec::new();
    let mut truncated = false;

    for root in [
        &installation.maps_root,
        &installation.mods_root,
        &installation.interfaces_root,
    ] {
        if walk(root, installation, false, &mut entries, &mut truncated) {
            break;
        }
    }

    let mut external_roots = Vec::new();
    if !truncated {
        for raw in extra {
            let dir = PathBuf::from(raw);
            if !dir.is_dir() {
                continue;
            }
            external_roots.push(raw.clone());
            if walk(&dir, installation, true, &mut entries, &mut truncated) {
                break;
            }
        }
    }

    Ok(DevScan {
        entries,
        truncated,
        external_roots,
    })
}

/// 走一棵树。返回 true 表示撞到上限了，调用方该停。
fn walk(
    root: &Path,
    installation: &Installation,
    external: bool,
    out: &mut Vec<DevEntry>,
    truncated: &mut bool,
) -> bool {
    if !root.is_dir() {
        return false;
    }

    // 只走两层：再深列出来也没人看，界面也不展开到那么细
    for entry in walkdir::WalkDir::new(root)
        .max_depth(2)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.path() == root {
            continue;
        }
        let is_dir = entry.file_type().is_dir();
        let bytes = if is_dir {
            0
        } else {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        };
        let path = if external {
            entry.path().to_string_lossy().replace('\\', "/")
        } else {
            entry
                .path()
                .strip_prefix(&installation.root)
                .map(|rest| rest.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| entry.path().to_string_lossy().replace('\\', "/"))
        };

        out.push(DevEntry {
            abs: entry.path().to_string_lossy().into_owned(),
            name: entry.file_name().to_string_lossy().into_owned(),
            path,
            is_dir,
            bytes,
            external,
        });

        if out.len() >= MAX_ENTRIES {
            *truncated = true;
            return true;
        }
    }

    false
}

/// 文件预览：编辑器按它决定怎么开。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilePreview {
    /// 能按文本打开。
    Text {
        text: String,
        lines: usize,
        /// 只给了前面一段（超过 [`MAX_TEXT_BYTES`]）。
        truncated: bool,
        bytes: u64,
    },
    /// 图片：界面自己显示，我们只报大小。
    Image { bytes: u64 },
    /// 二进制：界面给「不支持的打开方式」那张落版。
    Binary { bytes: u64 },
}

/// 图片扩展名。**不解析内容** —— 界面只是显示，认扩展名够了。
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// 只读预览一个文件。
///
/// 判定顺序：目录 → 图片 → 有 NUL 或不是 UTF-8 → 二进制 → 文本。
/// 这是**故意简单**的：编辑器不需要知道地图文件里面是什么，
/// 它只需要知道「能不能按行显示」。
pub fn read_preview(path: &Path) -> Result<FilePreview> {
    let meta = std::fs::metadata(path)?;
    let bytes = meta.len();

    if !meta.is_file() {
        return Ok(FilePreview::Binary { bytes });
    }

    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(FilePreview::Image { bytes });
    }

    let head = read_head(path, MAX_TEXT_BYTES.min(bytes as usize))?;
    if !looks_textual(&head) {
        return Ok(FilePreview::Binary { bytes });
    }

    let truncated = bytes > MAX_TEXT_BYTES as u64;
    Ok(FilePreview::Text {
        lines: head.iter().filter(|byte| **byte == b'\n').count() + 1,
        text: String::from_utf8_lossy(&head).into_owned(),
        truncated,
        bytes,
    })
}

fn read_head(path: &Path, limit: usize) -> Result<Vec<u8>> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; limit];
    let read = file.read(&mut buffer)?;
    buffer.truncate(read);
    Ok(buffer)
}

/// 像不像文本：合法 UTF-8 且没有 NUL。只看开头一段 —— 整文件扫在白读。
fn looks_textual(data: &[u8]) -> bool {
    let head = &data[..data.len().min(4096)];
    !head.contains(&0) && std::str::from_utf8(head).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_binary_are_told_apart() {
        let dir = tempfile::tempdir().expect("tempdir");

        let text = dir.path().join("说明.txt");
        std::fs::write(&text, "第一行\n第二行\n").expect("写");
        match read_preview(&text).expect("预览") {
            FilePreview::Text {
                lines, truncated, ..
            } => {
                assert_eq!(lines, 3, "两行文字 + 结尾那行");
                assert!(!truncated);
            }
            other => panic!("文本被判成了 {other:?}"),
        }

        let binary = dir.path().join("a.SC2Map");
        std::fs::write(&binary, [0x4d, 0x50, 0x51, 0x1a, 0x00, 0x01]).expect("写");
        assert!(matches!(
            read_preview(&binary).expect("预览"),
            FilePreview::Binary { .. }
        ));

        let image = dir.path().join("封面.png");
        std::fs::write(&image, [0x89, 0x50, 0x4e, 0x47]).expect("写");
        assert!(matches!(
            read_preview(&image).expect("预览"),
            FilePreview::Image { .. }
        ));
    }

    #[test]
    fn scan_lists_content_roots_and_marks_outsiders() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("StarCraft II.exe"), b"stub").expect("marker");
        let installation =
            Installation::from_root(root, crate::sc2::DiscoverySource::Manual).expect("安装");
        std::fs::create_dir_all(installation.maps_root.join("CustomCampaigns")).expect("mkdir");
        std::fs::write(
            installation
                .maps_root
                .join("CustomCampaigns")
                .join("a.SC2Map"),
            b"x",
        )
        .expect("写");

        let outside = dir.path().join("外部工作区");
        std::fs::create_dir_all(&outside).expect("mkdir");
        std::fs::write(outside.join("b.txt"), b"hi").expect("写");

        let scan = scan(&installation, &[outside.to_string_lossy().into_owned()]).expect("扫描");
        assert!(
            scan.entries
                .iter()
                .any(|entry| entry.path.starts_with("Maps/") && !entry.external),
            "游戏目录里的要按相对路径列"
        );
        assert!(
            scan.entries.iter().any(|entry| entry.external),
            "自定义目录要标成 external"
        );
        assert_eq!(scan.external_roots.len(), 1);
    }
}
