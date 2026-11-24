//! 数据根：固定在星际争霸安装目录的同级，附旧 `data/` 迁移。
//!
//! 数据根**只有一处可去**（见 [`data_root`]）。写不进去算权限问题，由调用方提权
//! （见 `docs/design/storage-and-elevation.md`），不再"不可写就退回 exe 旁边" ——
//! 那条路会让同一台机器的几个启动器实例各持一份库。

use std::path::{Path, PathBuf};

/// 数据目录名 —— 放在**星际争霸安装目录的同级**。
///
/// 为什么不在 exe 同级：绿色版可以拷好几份、也可以装在不同地方，各自一份 data 就互不相通。
/// 挂在游戏安装目录旁边之后，同一台机器上的所有启动器实例共用同一份战役库。
pub const DATA_DIR_NAME: &str = "miyin_launcher_data";

/// 数据根目录：`<游戏安装目录的父目录>/miyin_launcher_data`。
///
/// 只算路径，不碰磁盘。不再接受 `Option`，也没有 exe 同级 `data/` 的回退。
pub fn data_root(installation_root: &Path) -> PathBuf {
    installation_root
        .parent()
        .unwrap_or(installation_root)
        .join(DATA_DIR_NAME)
}

/// 旧绿色版的数据目录：exe 同级的 `data/`。**只当迁移源**，不再是任何情况下的兜底。
fn legacy_root(executable: &Path) -> PathBuf {
    executable.parent().unwrap_or(Path::new(".")).join("data")
}

/// 数据根之下的固定布局。
///
/// 约定：**临时文件只进 [`DataPaths::temp`]**（更新包、工具安装包、WebView2 安装器、
/// 解包中转都在它下面）；工具与日志是长期内容，放数据根；启动器配置放 `%APPDATA%`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPaths {
    /// 数据根：`<游戏安装目录的父目录>/miyin_launcher_data`。
    pub root: PathBuf,
    /// 唯一临时目录：`<root>/temp`。
    pub temp: PathBuf,
    /// 更新包与解包：`<root>/temp/updates`。
    pub updates: PathBuf,
    /// 工具安装包中转：`<root>/temp/tools`。
    pub tool_downloads: PathBuf,
    /// WebView2 安装包：`<root>/temp/webview2`。
    pub webview2_download: PathBuf,
    /// 已安装的工具：`<root>/tools`。
    pub tools: PathBuf,
    /// 日志：`<root>/logs`。
    pub logs: PathBuf,
}

impl DataPaths {
    /// 只算路径，不碰磁盘。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let temp = root.join("temp");
        Self {
            updates: temp.join("updates"),
            tool_downloads: temp.join("tools"),
            webview2_download: temp.join("webview2"),
            tools: root.join("tools"),
            logs: root.join("logs"),
            temp,
            root,
        }
    }

    /// 建好整套目录。
    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [
            &self.root,
            &self.temp,
            &self.updates,
            &self.tool_downloads,
            &self.webview2_download,
            &self.tools,
            &self.logs,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// 清空临时目录（启动时调用）；返回删掉的条目数。
    ///
    /// 三个固定子目录保留、只清内容，解包用的 `unpack-*` 整项删掉。
    /// 这些内容要么已经装完，要么只在本次进程内有效，重启后都不再需要。
    pub fn sweep_temp(&self) -> std::io::Result<usize> {
        let mut removed = 0;
        for dir in [&self.updates, &self.tool_downloads, &self.webview2_download] {
            removed += clear_contents(dir)?;
        }

        let Ok(entries) = std::fs::read_dir(&self.temp) else {
            return Ok(removed);
        };
        let reserved = [&self.updates, &self.tool_downloads, &self.webview2_download];
        for entry in entries.flatten() {
            let path = entry.path();
            if reserved.contains(&&path) {
                continue;
            }
            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            if result.is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// 清掉旧版本留在数据根里的目录（`updates/`、`webview2/`）；返回删掉的个数。
    pub fn remove_legacy_dirs(&self) -> usize {
        ["updates", "webview2"]
            .iter()
            .filter(|name| {
                let path = self.root.join(name);
                path.is_dir() && std::fs::remove_dir_all(&path).is_ok()
            })
            .count()
    }
}

/// 数据根下的布局（只算路径）。
pub fn data_paths(installation_root: &Path) -> DataPaths {
    DataPaths::new(data_root(installation_root))
}

/// 数据根的结论：最后用哪个、有没有搬过东西、给人看的一句话。
#[derive(Debug, Clone)]
pub struct DataRootPlan {
    /// 实际使用的数据根 —— 永远是 [`data_root`] 算出来的那个。
    pub root: PathBuf,
    /// 旧的 exe 同级 `data/`（迁移前存在才给）。
    pub legacy: Option<PathBuf>,
    /// 这次是不是真的把旧数据搬过来了。
    pub migrated: bool,
    /// 给日志 / 界面的一句话说明。
    pub note: String,
}

/// 一个数据根算不算「完整」—— 有 `library.json` 才算。
///
/// 为什么不能只看目录在不在：跨盘搬迁搬到一半断电，新根会留下**半份**目录。
/// 把它当完整库用，界面上就是「库空了」，而完整的旧数据就在旁边（verifier 实测过）。
fn is_complete(root: &Path) -> bool {
    root.is_dir() && root.join("library.json").is_file()
}

/// 搬迁中转目录：先整个搬到这里，成了再改名过去 —— 改名是原子的。
pub(super) fn partial_root(root: &Path) -> PathBuf {
    let mut name = root.as_os_str().to_os_string();
    name.push(".partial");
    PathBuf::from(name)
}

/// 给「半途而废」的那份找个不重名的落脚处（`<root>.incomplete`、`-2`…）。
fn parked_root(root: &Path) -> PathBuf {
    for n in 0..1000 {
        let mut name = root.as_os_str().to_os_string();
        name.push(if n == 0 {
            ".incomplete".to_string()
        } else {
            format!(".incomplete-{n}")
        });
        let candidate = PathBuf::from(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    let mut name = root.as_os_str().to_os_string();
    name.push(".incomplete-old");
    PathBuf::from(name)
}

/// 清掉一个目录里的所有条目，目录本身保留；目录不存在就当作已经干净。
fn clear_contents(dir: &Path) -> std::io::Result<usize> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(0);
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if result.is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// 目录里有没有东西（空目录不算"半份数据"）。
fn dir_has_entries(root: &Path) -> bool {
    std::fs::read_dir(root)
        .map(|mut it| it.next().is_some())
        .unwrap_or(false)
}

/// 决定数据根，并在需要时把 exe 旁的旧 `data/` 搬过来。
///
/// 数据根**永远是** [`data_root`] 算出来的那个；旧 `data/` 只是迁移源。
/// 新根是空目录 / 半份目录时先把它挪到 `<root>.incomplete` 留证再重搬 ——
/// 上次跨盘搬迁断在半路留下的半份不能当完整库用，否则界面上整个库会消失。
///
/// 搬迁先落到 `<root>.partial`，完成后再**改名**进来 —— 改名是原子的，
/// 所以中途断电只会留下一个 `.partial`，下一次启动会被认出来并接着办完。
pub fn resolve_data_root(installation_root: &Path, executable: &Path) -> DataRootPlan {
    let root = data_root(installation_root);
    let legacy = legacy_root(executable);
    let legacy = legacy.is_dir().then_some(legacy);

    // 新根已经是完整库 —— 用它（旧数据一个字节都不动）。
    // **这一步必须排在碰 .partial 之前**：多实例同时在跑时，别人正在写的中转目录不该被我们搬走。
    if is_complete(&root) {
        return DataRootPlan {
            note: if legacy.is_some() {
                format!(
                    "当前使用数据目录 {}（程序旁仍有旧 data/，未改动）",
                    root.display()
                )
            } else {
                format!("数据目录 {}", root.display())
            },
            root,
            legacy,
            migrated: false,
        };
    }

    // 上次搬到一半留下的中转目录、或者一份半成品目录，先挪开（**保留**）再重来。
    // 只把「有内容的目录」和「挡路的文件」算半份 —— **空目录不算**（那只是还没导入过东西），
    // 否则每次启动都会攒一个 .incomplete。
    let partial = partial_root(&root);
    let mut leftover: Option<PathBuf> = None;

    // 上次搬到一半留下的中转目录：**先接着把它改名叫上来** —— 里面装的可能是真数据
    // （legacy -> .partial 成功了、.partial -> root 那一步失败时，旧目录已经不在，
    // 数据全在中转目录里；当「半份」停掉就等于把用户的库晾在那儿）。
    if partial.is_dir() && crate::platform::move_path(&partial, &root).is_err() {
        let parked = parked_root(&root);
        if crate::platform::move_path(&partial, &parked).is_ok() {
            leftover = Some(parked);
        }
    }

    // 半成品目录（有内容但没有 library.json）也挪开 —— 空目录不算（那只是还没导入过东西）。
    if root.exists() && (!root.is_dir() || (dir_has_entries(&root) && !is_complete(&root))) {
        let parked = parked_root(&root);
        if crate::platform::move_path(&root, &parked).is_ok() {
            leftover = Some(parked);
        }
    }

    let note_prefix = leftover
        .as_ref()
        .map(|parked| format!("上次的搬迁没完成，半份留在 {}；", parked.display()))
        .unwrap_or_default();

    let Some(legacy_path) = legacy else {
        return DataRootPlan {
            note: format!("{note_prefix}数据目录 {}", root.display()),
            root,
            legacy: None,
            migrated: false,
        };
    };

    // 先搬到 .partial，成了再改名进来
    let partial = partial_root(&root);
    let moved = crate::platform::move_path(&legacy_path, &partial)
        .and_then(|()| crate::platform::move_path(&partial, &root));
    match moved {
        Ok(()) => DataRootPlan {
            note: format!(
                "{note_prefix}已把 {} 迁到 {}",
                legacy_path.display(),
                root.display()
            ),
            root,
            legacy: Some(legacy_path),
            migrated: true,
        },
        Err(error) => DataRootPlan {
            note: format!(
                "{note_prefix}迁移数据目录失败（{error}），稍后再试；数据仍在 {}",
                legacy_path.display()
            ),
            root,
            legacy: Some(legacy_path),
            migrated: false,
        },
    }
}
