//! 弥音启动器 Tauri 后端。
//!
//! 这一层**只做三件事**：持有配置与状态、把命令转发给 `miyin-core`、
//! 把领域错误转成可以展示的字符串。所有业务规则都在 `miyin-core` 里
//! （见 `AGENTS.md` 的依赖方向约定）。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use miyin_core::library::Library;
use miyin_core::sc2::{DiscoverySource, Installation};
use miyin_core::update::apply::Staged;
use miyin_core::update::net::NetworkSettings;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use commands::{campaign, dev, mods, patch, saves, settings, ui, window};
use platform::{install_crash_hook, panic_text};

// 命令层按领域拆到 commands/，平台相关实现拆到 platform.rs —— 这里只留状态与启动。
mod commands;
mod platform;
// 内嵌 7-Zip（导入解压 + 「解压工具」窗口）
mod sevenzip;
// 提权助手（同 exe 的 --privileged-worker 模式）
mod worker;
// 缺 WebView2 运行时时的自动补全（原生进度窗 + 下载安装），只有 Windows 有这回事
#[cfg(windows)]
mod webview2_install;

/// 持久化配置。
#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    /// 用户手动指定的游戏根目录。
    game_root: Option<PathBuf>,
}

/// 应用运行期状态。
/// 网络设置的落盘文件名（与既有配置放在同一目录）。
const NETWORK_FILE: &str = "network.json";

/// panic 现场记录的位置（exe 同级）。release 没有控制台，用户报障只能靠它。
pub(crate) const CRASH_LOG: &str = "miyin-crash.log";

pub(crate) struct AppState {
    installation: Mutex<Option<Installation>>,
    /// 启动时算好的合并案（有冲突时才留给界面问）—— 见 library::merge。
    merge_plan: Mutex<Option<miyin_core::library::merge::MergePlan>>,
    config_path: Mutex<Option<PathBuf>>,
    /// 战役库：存放玩家导入的各个版本（与游戏目录解耦）。
    library: Library,
    /// 网络设置（代理 / 镜像 / 更新通道），落盘在 `network.json`。
    network: Mutex<NetworkSettings>,
    /// 网络设置文件的位置。
    network_path: PathBuf,
    /// 已经下载好、等着换上去的更新。
    staged: Mutex<Option<Staged>>,
    /// 启动时自动挂载留下的提示（跳过 / 失败），等前端启动完取一次。
    ///
    /// release 没有控制台，只写 `eprintln!` 等于没写 —— 用户会以为自动挂载成功了。
    /// 见 [`take_mount_warnings`]。
    mount_warnings: Mutex<Vec<String>>,
    /// 导出想中途停手时置 true（见 dev_cancel_export）。
    dev_cancel: std::sync::atomic::AtomicBool,
}

/// 后台启动的结果。
///
/// 先 `manage` 上，前台的 `boot_ready` 靠它区分「还在准备」和「准备失败了」——
/// 后台线程 panic 时 AppState 永远不会被 manage，没有这个就等于前端永远转圈。
#[derive(Default)]
struct BootStatus {
    /// Some(文本) = 数据准备失败（bootstrap panic 了）。成功时一直是 None。
    error: Mutex<Option<String>>,
}

thread_local! {
    /// 当前线程 panic 时**不要**弹原生框。
    ///
    /// 后台启动线程的 panic 会被 `catch_unwind` 接住、原因走 `boot_ready` 显示在界面上；
    /// 再从 hook 里弹一个框，只会挡在真正的原因前面。
    pub(crate) static PANIC_QUIET: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl AppState {
    /// 读取网络设置、准备战役库。
    ///
    /// 安装位置和数据根在 [`run`] 里就已经定好了（要先把旧配置 / 注册表 / 用户手选跑完，
    /// 还要把旧 `data/` 搬过来）—— 这里不再自己 resolve，免得两条路各说各话。
    fn bootstrap(installation: Installation, data_root: PathBuf, config_path: PathBuf) -> Self {
        // 网络设置与既有配置放同一目录
        let network_path = config_path
            .parent()
            .map(|dir| dir.join(NETWORK_FILE))
            .unwrap_or_else(|| data_root.join(NETWORK_FILE));
        let network = std::fs::read_to_string(&network_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        let library = Library::new(data_root);

        // **启动时自动挂载模组**：用户不该为了「重装过 / Mods 被删过 / 上次没铺完」
        // 再去点一次启用。失败不拦启动 —— 但也不能只写 eprintln：release 没有控制台，
        // 用户会以为挂上了。提示攒进 [`AppState`]，前端启动完用 [`take_mount_warnings`] 取一次。
        let mut mount_warnings: Vec<String> = Vec::new();
        {
            let installation = &installation;
            if let Some(running) = miyin_core::platform::game_running() {
                // 这条不算失败（游戏跑着本来就该跳过），不进用户提示
                eprintln!("[miyin] 游戏正在运行（{running}），跳过启动时的模组自动挂载");
            } else {
                match miyin_core::library::activation::sync_mods(&library, installation) {
                    Ok(warnings) => mount_warnings.extend(warnings),
                    Err(error) => mount_warnings.push(format!("模组自动挂载失败：{error}")),
                }
                if let Err(error) = miyin_core::library::mods::sync(library.root(), installation) {
                    mount_warnings.push(format!("独立模组同步失败：{error}"));
                }
                // 安装清单坏了要让人知道：否则它会以「空清单」的样子出现，
                // 用户看到的是界面说「没装」、游戏目录里却躺着一堆我们放的文件。
                if let Some(message) = miyin_core::library::Manifest::health(library.root()) {
                    mount_warnings.push(message);
                }
                // 已启用的版本文件不全（重装过游戏 / 用户手清过目录）：只提示，不自动铺地图
                mount_warnings.extend(miyin_core::library::activation::incomplete_installation(
                    &library,
                    installation,
                ));
            }
        }

        Self {
            installation: Mutex::new(Some(installation)),
            config_path: Mutex::new(Some(config_path)),
            library,
            // 旧 `data/` 由 core 的 resolve_data_root 整个搬过来，不再走「合并案」那条路
            merge_plan: Mutex::new(None),
            network: Mutex::new(network),
            network_path,
            staged: Mutex::new(None),
            mount_warnings: Mutex::new(mount_warnings),
            dev_cancel: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 记住用户选择的安装位置。
    fn remember(&self, installation: &Installation) {
        let Ok(path) = self.config_path.lock() else {
            return;
        };
        let Some(path) = path.as_ref() else {
            return;
        };
        let config = Config {
            game_root: Some(installation.root.clone()),
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(path, text);
        }
    }
}

/// 读取配置；文件不存在或损坏都回退到默认值。
fn load_config(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// 把锁中毒转成可展示的错误。
pub(crate) fn lock_error<T>(_: PoisonError<T>) -> String {
    "内部状态异常，请重启启动器".to_string()
}

/// 写游戏目录前的拦截：游戏跑着的时候一律不碰游戏目录。
///
/// 运行中的游戏占着 `Maps/` / `Mods/` 里的文件 —— 删不掉、也覆盖不了，半途失败会留下
/// 「清单说撤了、文件还在」的账实分叉。宁可请用户先退游戏。
pub(crate) fn ensure_game_not_running() -> Result<(), String> {
    match miyin_core::platform::game_running() {
        Some(running) => Err(format!(
            "游戏正在运行（{running}）：先把游戏关掉再改战役 —— 它正占着游戏目录里的文件，这时候改会留下删不掉的旧文件"
        )),
        None => Ok(()),
    }
}

/// 库里这个路径的东西，装到游戏目录之后在哪。
///
/// `Ok(None)` = 清单里没有这条记录（没装过）；`Err` = 清单说装了、游戏目录里却没有 ——
/// 调用方**不能**把它当成「没装」静默退回库里那份，得让用户知道为什么打不开。
/// [`Manifest::target_of`] 自己会按当前库根把「记录的相对路径」和这个绝对路径对上
/// （数据根可能搬过，所以清单里不再信绝对路径）。
pub(crate) fn installed_target(
    state: &State<'_, AppState>,
    installation: &Installation,
    library_path: &Path,
) -> Result<Option<PathBuf>, String> {
    miyin_core::library::Manifest::load(state.library.root())
        .target_of(state.library.root(), installation, library_path)
        .map_err(|error| error.to_string())
}

/// 取走启动时自动挂载攒下的提示（取一次就清空，免得每次启动重复念）。
///
/// 前端启动完调一次；非空就告诉用户「上次模组没挂全」，别让他以为已经生效了。
#[tauri::command(async)]
fn take_mount_warnings(state: State<'_, AppState>) -> Vec<String> {
    state
        .mount_warnings
        .lock()
        .map(|mut warnings| std::mem::take(&mut *warnings))
        .unwrap_or_default()
}

/// 取当前安装，未设置时报错。
pub(crate) fn require_installation(state: &State<'_, AppState>) -> Result<Installation, String> {
    state
        .installation
        .lock()
        .map_err(lock_error)?
        .clone()
        .ok_or_else(|| "尚未设置星际争霸 II 安装目录，请到「设置」中选择".to_string())
}

/// 等后端把数据准备好（启动时的数据目录迁移跑在后台线程上，见 [`run`]）。
///
/// 前端在这条命令返回之前只画「正在准备数据」——**在它返回之前，任何碰 [`AppState`]
/// 的命令都会 panic**（状态还没 manage 上），所以它必须排在所有数据命令前面。
#[tauri::command(async)]
fn boot_ready(app: tauri::AppHandle) -> Result<(), String> {
    // 阻塞的是线程池里的线程，不是主线程（同步命令体 + `(async)` = spawn_blocking）
    loop {
        if app.try_state::<AppState>().is_some() {
            return Ok(());
        }
        // 后台线程 panic 了：AppState 永远不会就绪，得把原因交给界面，不能让它转圈到死
        if let Some(error) = boot_error(&app) {
            return Err(error);
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
}

/// 后台启动失败的原因（还在准备 / 成功时都是 `None`）。
fn boot_error(app: &tauri::AppHandle) -> Option<String> {
    let status = app.try_state::<BootStatus>()?;
    status.error.lock().ok()?.clone()
}

/// 记下后台启动失败的原因，`boot_ready` 会把它交给界面。
fn mark_boot_failed(app: &tauri::AppHandle, text: String) {
    let Some(status) = app.try_state::<BootStatus>() else {
        return;
    };
    let Ok(mut slot) = status.error.lock() else {
        return;
    };
    *slot = Some(text);
}

/// 配置目录：`%APPDATA%\net.miyin.launcher`（即 Tauri 的 `app_config_dir()`）。
///
/// 解析安装发生在 Tauri 起来之前，那会儿拿不到 `app.path()`，所以按同一位置自己拼。
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("net.miyin.launcher"))
}

/// 解析安装：`%APPDATA%` 里的配置 → 注册表 → 用户手选。
fn resolve_installation() -> Option<Installation> {
    let mut hints: Vec<(PathBuf, DiscoverySource)> = Vec::new();
    if let Some(dir) = config_dir() {
        if let Some(root) = load_config(&dir.join("config.json")).game_root {
            hints.push((root, DiscoverySource::Manual));
        }
    }
    if let Ok(found) = Installation::discover() {
        hints.push((found.root, DiscoverySource::Registry));
    }

    for (root, source) in hints {
        if let Ok(installation) = Installation::from_root(&root, source) {
            return Some(installation);
        }
    }

    // 注册表里没有、配置也没记：请用户选一次。没有它数据根无处安放。
    loop {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("选择《星际争霸 II》安装目录")
            .pick_folder()
        else {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("弥音启动器")
                .set_description(concat!(
                    "没有指定《星际争霸 II》的安装目录。\n\n",
                    "启动器把战役库放在游戏安装目录旁边，",
                    "不知道游戏在哪就没法继续，这次先退出。",
                ))
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
            return None;
        };
        match Installation::from_root(&folder, DiscoverySource::Manual) {
            Ok(installation) => return Some(installation),
            Err(_) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Warning)
                    .set_title("这个目录里没有游戏")
                    .set_description(format!(
                        concat!(
                            "{} 里没有 StarCraft II.exe。\n\n",
                            "请选择《星际争霸 II》的安装根目录 —— ",
                            "打开它就能直接看到 StarCraft II.exe 的那个文件夹。"
                        ),
                        folder.display()
                    ))
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
            }
        }
    }
}

/// 安装 + 数据根。exe 旁边若还留着旧 `data/`，core 会在这里整个搬过去。
fn resolve_installation_and_root(executable: &Path) -> Option<(Installation, PathBuf)> {
    let installation = resolve_installation()?;
    let plan = miyin_core::library::resolve_data_root(&installation.root, executable);
    if plan.migrated || plan.legacy.is_some() {
        eprintln!("[miyin] {}", plan.note);
    }
    Some((installation, plan.root))
}

/// 启动应用。
///
/// 顺序按 `docs/design/storage-and-elevation.md` §2（**普通权限启动，不弹 UAC**）：
/// crash hook → 提权助手分流 → 单实例锁 → 安装发现 → 数据根 → 普通 Tauri 启动。
///
/// 有一条和「白屏」有关的窗口参数：`additionalBrowserArgs` 显式带上了
/// `CalculateNativeWinOcclusion`（关掉 Chromium 的原生窗口遮挡计算 —— 窗口被别的窗口盖住 /
/// 最小化 / 被系统判成「不可见」时，Chromium 会停掉渲染，再回来可能是一片白）。
///
/// **依据**：wry 的默认参数是 `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`
/// （wry 0.55.1 src/webview2/mod.rs:297），一旦自己传这个参数，那三个默认项就丢了 ——
/// 所以四项必须一起写（见 tauri-utils 2.9.3 config.rs:2081 的字段说明）。
/// 窗口由 `tauri.conf.json` 按普通配置创建（不写 `create: false`、不手动指定 WebView2
/// profile 目录）；数据根写不进去也不拦启动，写操作出错时界面再按需提权。
pub fn run() {
    // 先装现场记录：下面任何一步 panic，用户和我们都拿得到原因（release 没有控制台）
    install_crash_hook();

    // 提权助手：同一个 exe 的 `--privileged-worker` 模式，不加载 Tauri / WebView2。
    // 不是助手模式时立刻返回 false，继续正常启动（助手不碰单实例锁）。
    if worker::run_if_requested() {
        return;
    }

    // 内嵌 7-Zip：解到用户目录并设置 MIYIN_7Z，导入 7z/rar 不再依赖系统装没装。
    sevenzip::prepare_for_startup();

    // 低完整性目录（安全软件保护区、某些沙箱工作区）里的 exe 会被 Windows 以 Low 完整性
    // 启动：写不了 WebView2 profile，窗口必然建不出来。这里先给明确原因，别让用户猜。
    #[cfg(windows)]
    if platform::is_low_integrity() {
        let text = "启动器正在以低完整性（Low integrity）运行。\n\n".to_string()
            + "通常是因为程序被解压在受限制的目录（安全软件保护的项目目录、某些沙箱工作区等），"
            + "这种权限下 WebView2 无法创建缓存。\n\n请把压缩包解压到普通文件夹（桌面、D:\\Games 等）"
            + "再运行；不要直接放在受保护的工作目录里。";
        eprintln!("[miyin] {text}");
        platform::fatal_dialog(&text);
        std::process::exit(1);
    }

    // 单实例：命名内核对象，与数据根是否可写无关。已有实例就把它的窗口拎到前面；
    // 连窗口都找不到才提示后退出。
    if !platform::claim_single_instance() {
        if !platform::focus_existing_window() {
            platform::notify_already_running();
        }
        return;
    }

    let executable =
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("miyin-launcher.exe"));

    // 安装位置 + 数据根（exe 旁边那份旧 data/ 在这里搬过去）
    let Some((installation, data_root)) = resolve_installation_and_root(&executable) else {
        return;
    };

    // 绿色版没有安装器 —— 机器上可能压根没装 WebView2 运行时，缺它时窗口根本建不出来。
    // 这里先自己补环境：边下边显示进度，成功就继续正常启动，取消 / 失败才退出
    // （原生进度窗与失败指引都在 webview2_install 里，见 AGENTS.md 第 14.9 节）。
    // 运行时已经装好时**什么都不做**，启动逻辑与普通 Tauri 应用一致。
    #[cfg(windows)]
    {
        match platform::webview2_version() {
            Some(version) => eprintln!("[miyin] WebView2 {version}"),
            None => {
                if let Err(reason) = webview2_install::ensure_runtime() {
                    eprintln!("[miyin] WebView2 自动补全没完成：{reason}");
                    return;
                }
            }
        }
        // 调试开关：这台机器已经装了运行时，用 MIYIN_WEBVIEW2_DEMO=1 强制再走一遍安装界面
        if webview2_install::demo_mode() {
            if let Err(reason) = webview2_install::ensure_runtime() {
                eprintln!("[miyin] WebView2 演示流程没走完：{reason}");
                return;
            }
        }
    }

    let started = tauri::Builder::default()
        .setup(move |app| {
            // 托盘：进程活着就让用户看得见、能右键退出 —— 窗口万一没起来（WebView2 失败等）
            // 也不会只剩一个没有界面的后台进程，用户能自己关掉。
            if let Err(error) = setup_tray(app.handle()) {
                eprintln!("[miyin] 托盘创建失败：{error}");
            }

            // 窗口是 Tauri 在建 setup 之前按配置建的；到这里一个都没有，说明 WebView2 起不来
            // （典型：启动器放在受限制/低完整性目录）。直接报错退出，别留一个没界面的进程。
            // 注意：`webview_windows()` 里可能有「注册了但窗口没真正建出来」的条目，
            // 必须看 HWND 是否真的存在（受限目录下 WebView2 失败时就是这种状态）。
            let window_visible = app
                .webview_windows()
                .values()
                .any(|window| window.hwnd().is_ok());
            if !window_visible {
                return Err(std::io::Error::other(
                    "窗口没有创建成功：WebView2 无法启动。\n\n"                     .to_string()
                        + "常见原因：启动器被放在受限制的目录（例如被安全软件保护的项目目录），"
                        + "WebView2 建不了缓存。请解压到普通文件夹（桌面、Games 目录等）再试；"
                        + "或确认系统已安装 WebView2 运行时。",
                )
                .into());
            }

            // 配置留在 %APPDATA%（app_config_dir），不进数据根：看界面永远不需要提权
            let config_path = app
                .path()
                .app_config_dir()
                .map(|dir| dir.join("config.json"))
                .unwrap_or_else(|_| PathBuf::from("miyin-config.json"));

            // 让它不像浏览器：关掉 WebView2 自己的浏览器加速键（见 tame_webview）
            #[cfg(windows)]
            for (label, window) in app.webview_windows() {
                if let Err(error) = window.with_webview(|webview| platform::tame_webview(&webview)) {
                    eprintln!("[miyin] 拿不到 webview（{label}）：{error}；浏览器快捷键兜底交给页面里的拦截");
                }
            }

            // **数据准备不能压在主线程上。**
            //
            // setup 跑在主线程的事件循环里（tauri 2.11 src/app.rs:1425）。bootstrap 会做
            // 旧数据迁移（跨盘时是「复制 + 删源」，GB 级）—— 主线程一卡，WebView2
            // 连第一帧都画不出来，用户看到的就是「启动后白屏」，白多久取决于库有多大。
            //
            // 所以这里只起线程，状态等它准备好再 manage；前端用 `boot_ready` 等它。
            //
            // 失败（磁盘满、权限不足、目录解析炸了）时不能就这么算了 —— 那时 AppState
            // 永远不会就绪，前端会一直停在「正在准备数据」。catch_unwind 接住原因，
            // 交给先 manage 上的 BootStatus，boot_ready 把文本还给界面。
            app.manage(BootStatus::default());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // 这个线程上的 panic 不弹原生框：原因会走 boot_ready 显示在界面上
                PANIC_QUIET.with(|quiet| quiet.set(true));

                match std::panic::catch_unwind(move || {
                    AppState::bootstrap(installation, data_root, config_path)
                }) {
                    Ok(state) => {
                        handle.manage(state);
                    }
                    Err(payload) => {
                        let text = panic_text(&*payload);
                        eprintln!("[miyin] 启动失败：{text}");
                        mark_boot_failed(&handle, text);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            boot_ready,
            take_mount_warnings,
            dev::dev_scan,
            settings::merge_status,
            settings::merge_apply,
            dev::dev_export,
            dev::dev_pick_doc,
            dev::dev_pick_export_path,
            dev::list_installed_campaigns,
            dev::dev_history,
            dev::dev_forget_commit,
            dev::dev_commit,
            dev::dev_diff,
            saves::list_saves,
            saves::save_isolation,
            saves::set_save_isolation,
            saves::save_current_saves,
            saves::save_into_profile,
            saves::saves_targets,
            saves::save_group_files,
            saves::move_save_file,
            saves::move_save_to_key,
            saves::scan_saves,
            saves::switch_save_profile,
            saves::assign_save_profile,
            saves::backup_saves,
            saves::restore_saves,
            saves::list_save_backups,
            campaign::sc2_running,
            campaign::preview_activation,
            campaign::activation_conflicts,
            campaign::deactivate_all_campaigns,
            dev::dev_pick_directory,
            dev::dev_read_file,
            settings::detect_installation,
            settings::set_installation,
            campaign::library_root,
            campaign::list_slots,
            campaign::export_variant,
            campaign::pick_export_path,
            settings::app_version,
            settings::network_settings,
            settings::set_network_settings,
            settings::detected_proxy,
            settings::check_update,
            settings::download_update,
            settings::apply_update,
            ui::open_url,
            patch::list_patches,
            patch::list_bindings,
            patch::list_available_patches,
            patch::import_patch,
            patch::auto_bind_patches,
            patch::bind_patch,
            patch::unbind_patch,
            patch::configure_patch,
            patch::update_patch,
            patch::export_patch,
            patch::delete_patch,
            patch::preview_composition,
            campaign::inspect_package,
            campaign::prepare_import,
            campaign::import_package,
            campaign::activate_variant,
            campaign::variant_maps,
            campaign::variant_mods,
            campaign::set_mounted_mods,
            campaign::set_main_map,
            campaign::main_map_choice,
            campaign::open_map_in_editor,
            campaign::variant_doc,
            mods::list_library_mods,
            mods::list_standalone_mods,
            settings::list_tools,
            settings::ensure_tool,
            window::window_minimize,
            window::window_toggle_maximize,
            window::window_close,
            window::window_is_maximized,
            settings::tool_releases,
            settings::install_tool,
            settings::uninstall_tool,
            settings::open_tool_repo,
            mods::pick_mod_source,
            mods::preview_mod,
            mods::import_mod,
            mods::update_mod,
            mods::edit_mod,
            mods::remove_mod,
            mods::export_mod,
            mods::export_mods,
            mods::compare_mods,
            mods::set_mod_enabled,
            mods::list_game_mods,
            campaign::read_doc,
            campaign::update_variant,
            campaign::delete_variant,
            campaign::launch_game,
            dev::dev_cancel_export,
            campaign::launch_game_with_map,
            settings::resolve_site_url,
            campaign::update_variant_from_package,
            ui::reveal_path,
            campaign::pick_package,
            campaign::pick_package_folder,
            settings::pick_game_directory,
            ui::campaign_cover,
            ui::variant_cover,
            worker::request_privileged_worker,
            worker::privileged_op,
            sevenzip::open_sevenzip,
            sevenzip::close_sevenzip,
        ])
        .run(tauri::generate_context!());

    // 起不来时不能「窗口没了、进程还在」：弹原生框说清原因，然后退出。
    if let Err(error) = started {
        let text = format!(
            "弥音启动器启动失败：{error}\n\n常见原因：\n\
             1. 启动器放在受限制的目录（例如被安全软件保护的项目目录），WebView2 建不了缓存；\n\
                请解压到普通文件夹（桌面、Games 目录等）再试。\n\
             2. 缺少 WebView2 运行时 —— 正常情况下启动器会自动安装，装完重开即可。\n\n\
             日志：程序旁边的 {CRASH_LOG}（如果存在）"
        );
        eprintln!("[miyin] {text}");
        platform::fatal_dialog(&text);
        std::process::exit(1);
    }
}

/// 托盘图标与右键菜单（显示主界面 / 退出启动器）。
///
/// 托盘是「进程还活着」的可见标志：窗口被系统/WebView2 弄没了，用户也能右键退出，
/// 不会留下一个只能去任务管理器结束的后台进程。
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出启动器", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::with_id("miyin-tray")
        .tooltip("弥音启动器")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// 把主窗口显示出来并提到前面（托盘左键 / 菜单「显示主界面」）。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    } else {
        eprintln!("[miyin] 主窗口已经不在了");
    }
}
