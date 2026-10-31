//! 设置与维护：安装目录 / 网络 / 工具 / 更新检查与安装 / 数据合并。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use miyin_core::sc2::{DiscoverySource, Installation};
use miyin_core::tools::{self, ToolRelease, ToolStatus};
use miyin_core::update::Reporter;
use miyin_core::update::apply::Staged;
use miyin_core::update::check::UpdateCheck;
use miyin_core::update::net::NetworkSettings;
use tauri::{Emitter, State};

use crate::platform::open_in_browser;
use crate::{AppState, lock_error};

/// 下载进度事件（发给前端画进度条）。
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ProgressPayload {
    done: u64,
    total: Option<u64>,
    /// 百分比；总长度未知时是 `None`。
    percent: Option<f64>,
}

/// 检测本机的星际争霸 II 安装。
/// 命令层的两条约定：
///
/// 1. **一律 `#[tauri::command(async)]`**。Tauri 2 里同步命令跑在**主线程**上，
///    而这里的活动辄几秒到几分钟（读整个压缩包、拷几个 GB 的战役、等用户选文件）——
///    同步写法会把主线程占死，窗口收不到消息泵，用户看到的就是**「未响应」**。
///    加 `(async)` 后 Tauri 会把它丢到线程池，界面照样能点。
///    （踩过一次：下载更新时整个窗口卡住。）
/// 2. 特别长的活（下载、检查更新）另外用 `async fn` + `spawn_blocking`，
///    免得占着异步 worker 几十秒。
///
#[tauri::command(async)]
pub(crate) fn detect_installation(
    state: State<'_, AppState>,
) -> Result<Option<Installation>, String> {
    let existing = state.installation.lock().map_err(lock_error)?.clone();
    if existing.is_some() {
        return Ok(existing);
    }

    match Installation::discover() {
        Ok(installation) => {
            *state.installation.lock().map_err(lock_error)? = Some(installation.clone());
            Ok(Some(installation))
        }
        Err(miyin_core::Error::InstallationNotFound) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// 手动指定游戏目录，校验通过后记住它。
#[tauri::command(async)]
pub(crate) fn set_installation(
    path: String,
    state: State<'_, AppState>,
) -> Result<Installation, String> {
    let installation = Installation::from_root(&path, DiscoverySource::Manual)
        .map_err(|error| error.to_string())?;

    state.remember(&installation);
    *state.installation.lock().map_err(lock_error)? = Some(installation.clone());
    Ok(installation)
}

/// 可选工具的清单与安装状态。
#[tauri::command(async)]
pub(crate) fn list_tools(state: State<'_, AppState>) -> Result<Vec<ToolStatus>, String> {
    let data = state.library.root();
    Ok(tools::ALL
        .iter()
        .map(|spec| tools::status(data, spec))
        .collect())
}

/// 找一个工具定义。
fn find_tool(id: &str) -> Result<tools::ToolSpec, String> {
    tools::ALL
        .iter()
        .copied()
        .find(|spec| spec.id == id)
        .ok_or_else(|| format!("不认识这个工具：{id}"))
}

/// 启动时调：**没装就静默装上**。
///
/// 失败不等于出错 —— 包成 `ToolEnsure` 返回，界面按 `message` 提示一句就行，
/// 而且之后不再自动重试（用户可以到设置里手动装）。
#[tauri::command(async)]
pub(crate) fn ensure_tool(
    id: String,
    state: State<'_, AppState>,
) -> Result<tools::ToolEnsure, String> {
    let spec = find_tool(&id)?;
    let settings = state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)?;

    Ok(tools::ensure(state.library.root(), &spec, &settings))
}

/// 某个工具有哪些版本可装。
#[tauri::command(async)]
pub(crate) fn tool_releases(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ToolRelease>, String> {
    let spec = find_tool(&id)?;
    let settings = state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)?;

    tools::releases(&spec, &settings, miyin_core::update::Reporter::silent())
}

/// 装一个工具。version 传 null 表示装最新的。
///
/// **静默安装**：不弹浏览器、不用用户解压，点了就下、下完就位。
#[tauri::command(async)]
pub(crate) fn install_tool(
    id: String,
    version: Option<String>,
    state: State<'_, AppState>,
) -> Result<ToolStatus, String> {
    let spec = find_tool(&id)?;
    let settings = state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)?;

    let reporter = miyin_core::update::Reporter::silent();
    let list = tools::releases(&spec, &settings, reporter)?;

    let picked = match &version {
        Some(wanted) => list
            .iter()
            .find(|release| &release.version == wanted || &release.tag == wanted)
            .ok_or_else(|| format!("没找到 {wanted} 这个版本"))?,
        None => list
            .iter()
            .find(|release| release.has_asset)
            .ok_or_else(|| format!("{} 还没有可下载的版本", spec.name))?,
    };

    tools::install(state.library.root(), &spec, picked, &settings, reporter)
        .map_err(|error| error.to_string())
}

/// 卸载一个工具。
#[tauri::command(async)]
pub(crate) fn uninstall_tool(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let spec = find_tool(&id)?;
    tools::uninstall(state.library.root(), &spec).map_err(|error| error.to_string())
}

/// 在系统浏览器里打开工具的仓库。
#[tauri::command(async)]
pub(crate) fn open_tool_repo(id: String) -> Result<(), String> {
    let spec = find_tool(&id)?;
    let url = format!("https://github.com/{}", spec.repo);
    open_in_browser(&url).map_err(|error| format!("打不开浏览器：{error}"))
}

/// 当前程序版本。
#[tauri::command(async)]
pub(crate) fn app_version() -> String {
    miyin_core::update::current_version().to_string()
}

/// 当前生效的网络设置。
#[tauri::command(async)]
pub(crate) fn network_settings(state: State<'_, AppState>) -> Result<NetworkSettings, String> {
    state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)
}

/// 改网络设置并落盘。
#[tauri::command(async)]
pub(crate) fn set_network_settings(
    settings: NetworkSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut guard) = state.network.lock() {
        *guard = settings.clone();
    }
    if let Some(parent) = state.network_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    std::fs::write(&state.network_path, text).map_err(|error| error.to_string())
}

/// 当前自动探测到的代理（界面要如实告诉用户走的是哪条路）。
#[tauri::command(async)]
pub(crate) fn detected_proxy(
    state: State<'_, AppState>,
) -> Option<miyin_core::update::net::DetectedProxy> {
    state
        .network
        .lock()
        .ok()
        .and_then(|guard| miyin_core::update::net::detect_proxy(&guard))
}

/// 检查更新。**网络失败不算错误**：包在返回值里，界面照常显示"检查失败"。
///
/// 过程会通过 `update://log` 事件实时发给界面，渲染成那个内嵌终端。
///
/// ⚠️ **必须是 `async fn`**：Tauri 2 里同步命令跑在**主线程**上，
/// 而这里是好几秒的网络等待 —— 同步写法会让整个窗口卡住（踩过，就是下载时"未响应"）。
/// 写成 async 后 Tauri 会把它丢到运行时线程；再用 `spawn_blocking`
/// 把这坨阻塞 IO 挪出异步线程池，免得占着 worker。
#[tauri::command(async)]
pub(crate) async fn check_update(
    window: tauri::Window,
    state: State<'_, AppState>,
) -> Result<UpdateCheck, String> {
    let settings = state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)?;

    tauri::async_runtime::spawn_blocking(move || {
        let emit = |message: &str| {
            let _ = window.emit("update://log", message.to_string());
        };
        miyin_core::update::check::check(
            &miyin_core::update::current_version(),
            &settings,
            Reporter::with_log(&emit),
        )
    })
    .await
    .map_err(|error| format!("检查任务异常：{error}"))
}

/// 下载更新包（会持续发 ``update://progress`` 事件）。
///
/// 下载是几十秒级别的事，**必须 async** —— 同步命令跑在主线程上，
/// 下载期间整个窗口得不到消息泵，用户看到的就是「未响应」。
#[tauri::command(async)]
pub(crate) async fn download_update(
    version: String,
    window: tauri::Window,
    state: State<'_, AppState>,
) -> Result<Staged, String> {
    let settings = state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)?;
    let data_dir = state.library.root().to_path_buf();

    let staged = tauri::async_runtime::spawn_blocking(move || -> Result<Staged, String> {
        // 用最新一次检查的结果拿资产；这里再查一次，避免界面把过期的 URL 传回来
        let quiet = |_: &str| {};
        let found = miyin_core::update::check::check(
            &miyin_core::update::current_version(),
            &settings,
            Reporter::with_log(&quiet),
        );
        let release = found
            .latest
            .filter(|release| release.version == version)
            .ok_or_else(|| format!("没找到 {version} 这个版本，请重新检查更新"))?;
        let asset = release
            .platform_asset()
            .ok_or_else(|| "这个发行版没有适合 Windows 的包".to_string())?
            .clone();

        // 日志与进度都实时发给界面：日志渲染成内嵌终端，进度画进度条
        let emit = |message: &str| {
            let _ = window.emit("update://log", message.to_string());
        };
        let progress = |done: u64, total: Option<u64>| {
            let _ = window.emit(
                "update://progress",
                ProgressPayload {
                    done,
                    total,
                    percent: total
                        .filter(|total| *total > 0)
                        .map(|total| ((done as f64 / total as f64) * 100.0).min(100.0)),
                },
            );
        };

        miyin_core::update::stage(
            &asset,
            &settings,
            &data_dir,
            &version,
            Reporter {
                log: Some(&emit),
                progress: Some(&progress),
            },
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("下载任务异常：{error}"))??;

    if let Ok(mut guard) = state.staged.lock() {
        *guard = Some(staged.clone());
    }

    Ok(staged)
}

/// 换上新版本并退出程序（替换脚本会等我们让出 exe 的锁）。
#[tauri::command(async)]
pub(crate) fn apply_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let staged = state
        .staged
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .ok_or_else(|| "还没有下载好的更新".to_string())?;

    let current = std::env::current_exe().map_err(|error| error.to_string())?;
    let data_dir = state.library.root().to_path_buf();

    let emit = |message: &str| {
        let _ = app.emit("update://log", message.to_string());
    };

    miyin_core::update::apply(&staged, &current, &data_dir, Reporter::with_log(&emit))
        .map_err(|error| error.to_string())?;

    // 给脚本一点时间起来，然后让出 exe
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        app.exit(0);
    });

    Ok(())
}

/// 弹出目录选择框，返回用户选中的游戏目录。
#[tauri::command(async)]
pub(crate) fn pick_game_directory() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择星际争霸 II 安装目录")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 启动时发现的数据合并案（没有冲突就已并完，这时返回 null）。
#[tauri::command(async)]
pub(crate) fn merge_status(
    state: State<'_, AppState>,
) -> Option<miyin_core::library::merge::MergePlan> {
    state.merge_plan.lock().ok().and_then(|plan| plan.clone())
}

/// 应用合并。`choices` 的 key 是 `"<slot>/<variant>"`，值是 `target` / `local` / `both`。
#[tauri::command(async)]
pub(crate) fn merge_apply(
    choices: std::collections::HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<miyin_core::library::merge::MergeReport, String> {
    use miyin_core::library::merge::MergeChoice;

    let plan = state
        .merge_plan
        .lock()
        .map_err(lock_error)?
        .clone()
        .ok_or("没有待处理的合并")?;

    let parsed: std::collections::BTreeMap<String, MergeChoice> = choices
        .into_iter()
        .map(|(key, value)| {
            let choice = match value.as_str() {
                "local" => MergeChoice::Local,
                "both" => MergeChoice::Both,
                _ => MergeChoice::Target,
            };
            (key, choice)
        })
        .collect();

    let report = miyin_core::library::merge::apply(&plan.target, &plan, &parsed)
        .map_err(|error| error.to_string())?;
    if let Ok(mut held) = state.merge_plan.lock() {
        *held = None;
    }
    Ok(report)
}

#[tauri::command(async)]
pub(crate) fn resolve_site_url(https: String, http: String) -> String {
    if miyin_core::net::https_usable(&https) {
        https
    } else {
        http
    }
}
