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

/// 应用用到的全部落盘位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// 数据根：`<游戏安装目录的父目录>/miyin_launcher_data`。
    pub root: PathBuf,
    /// 启动器配置（`<root>/config.json`）。
    pub config: PathBuf,
    /// 网络设置（`<root>/network.json`）。
    pub network: PathBuf,
    /// WebView2 profile 目录（`<root>/webview2`）。
    pub webview2: PathBuf,
    /// 解压 / 下载的临时目录（`<root>/temp`）。
    pub temp: PathBuf,
}

impl AppPaths {
    /// 数据根之下的固定布局。
    fn under(root: PathBuf) -> Self {
        Self {
            config: root.join("config.json"),
            network: root.join("network.json"),
            webview2: root.join("webview2"),
            temp: root.join("temp"),
            root,
        }
    }
}

/// 应用落盘位置：**只算路径**，不建目录、不迁移。
pub fn app_paths(installation_root: &Path) -> AppPaths {
    AppPaths::under(data_root(installation_root))
}

/// 准备好数据根：建目录、把 exe 旁的旧 `data/` 整体搬进来，然后给出落盘位置。
///
/// 搬完旧路径就不存在了。数据根已经有完整的库时不碰旧数据。
/// **不做"写不进去就换个目录"的回退** —— 写不了是权限问题，由调用方提权处理。
///
/// 搬迁失败不拦启动：旧数据原样留在原处（一个字节不丢），原因写进
/// [`DataRootPlan::note`]，下一次启动再试。提权后重跑通常就能搬成。
pub fn prepare_app_paths(installation_root: &Path, executable: &Path) -> std::io::Result<AppPaths> {
    let plan = resolve_data_root(installation_root, executable);
    let paths = AppPaths::under(plan.root);

    // config / network 是文件，它们的父目录就是 root，跟着一起建出来。
    for dir in [&paths.root, &paths.webview2, &paths.temp] {
        std::fs::create_dir_all(dir)?;
    }
    Ok(paths)
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
                    "数据目录用 {}（程序旁边还有一份旧的 data/，没动它）",
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
