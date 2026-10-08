//! 弥音启动器 Tauri 后端。
//!
//! 这一层**只做三件事**：持有配置与状态、把命令转发给 `miyin-core`、
//! 把领域错误转成可以展示的字符串。所有业务规则都在 `miyin-core` 里
//! （见 `AGENTS.md` 的依赖方向约定）。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use tauri::Emitter;

use miyin_core::campaign::metadata::PackageKind;
use miyin_core::campaign::package::{self, PackageInspection};
use miyin_core::campaign::scanner;
use miyin_core::library::{
    self, Binding, Conflict, DocInfo, ImportMode, Library, LibraryMod, MainMapChoice, MapEntry,
    ModChanges, ModEntry, Patch, SlotView, StandaloneMod, Variant, VariantChanges, mods,
};
use miyin_core::sc2::{DiscoverySource, GameModEntry, Installation};
use miyin_core::tools::{self, ToolRelease, ToolStatus};
use miyin_core::update::Reporter;
use miyin_core::update::apply::Staged;
use miyin_core::update::check::UpdateCheck;
use miyin_core::update::net::NetworkSettings;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

/// 持久化配置。
#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    /// 用户手动指定的游戏根目录。
    game_root: Option<PathBuf>,
}

/// 应用运行期状态。
/// 网络设置的落盘文件名（与既有配置放在同一目录）。
const NETWORK_FILE: &str = "network.json";

struct AppState {
    installation: Mutex<Option<Installation>>,
    config_path: Mutex<Option<PathBuf>>,
    /// 战役库：存放玩家导入的各个版本（与游戏目录解耦）。
    library: Library,
    /// 网络设置（代理 / 镜像 / 更新通道），落盘在 `network.json`。
    network: Mutex<NetworkSettings>,
    /// 网络设置文件的位置。
    network_path: PathBuf,
    /// 已经下载好、等着换上去的更新。
    staged: Mutex<Option<Staged>>,
}

/// 下载进度事件（发给前端画进度条）。
#[derive(Debug, Clone, serde::Serialize)]
struct ProgressPayload {
    done: u64,
    total: Option<u64>,
    /// 百分比；总长度未知时是 `None`。
    percent: Option<f64>,
}

impl AppState {
    /// 读取配置并尝试恢复上次使用的安装。
    fn bootstrap(config_path: PathBuf, library_root: PathBuf) -> Self {
        let config = load_config(&config_path);

        let installation = config
            .game_root
            .and_then(|root| Installation::from_root(root, DiscoverySource::Manual).ok())
            .or_else(|| Installation::discover().ok());

        // 网络设置与既有配置放同一目录
        let network_path = config_path
            .parent()
            .map(|dir| dir.join(NETWORK_FILE))
            .unwrap_or_else(|| PathBuf::from(NETWORK_FILE));
        let network = std::fs::read_to_string(&network_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        Self {
            installation: Mutex::new(installation),
            config_path: Mutex::new(Some(config_path)),
            library: Library::new(library_root),
            network: Mutex::new(network),
            network_path,
            staged: Mutex::new(None),
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

/// 用系统默认浏览器打开链接。
#[cfg(windows)]
fn open_in_browser(url: &str) -> std::io::Result<()> {
    // 不能直接 `start url`：cmd 会把 & 当分隔符。用 rundll32 更稳
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
}

#[cfg(not(windows))]
fn open_in_browser(url: &str) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .map(|_| ())
}

/// 把锁中毒转成可展示的错误。
fn lock_error<T>(_: PoisonError<T>) -> String {
    "内部状态异常，请重启启动器".to_string()
}

/// 取当前安装，未设置时报错。
fn require_installation(state: &State<'_, AppState>) -> Result<Installation, String> {
    state
        .installation
        .lock()
        .map_err(lock_error)?
        .clone()
        .ok_or_else(|| "尚未设置星际争霸 II 安装目录，请到「设置」中选择".to_string())
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
fn detect_installation(state: State<'_, AppState>) -> Result<Option<Installation>, String> {
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
fn set_installation(path: String, state: State<'_, AppState>) -> Result<Installation, String> {
    let installation = Installation::from_root(&path, DiscoverySource::Manual)
        .map_err(|error| error.to_string())?;

    state.remember(&installation);
    *state.installation.lock().map_err(lock_error)? = Some(installation.clone());
    Ok(installation)
}

/// 战役库根目录（软件同级的 data 目录）。
#[tauri::command(async)]
fn library_root(state: State<'_, AppState>) -> String {
    state.library.root().to_string_lossy().into_owned()
}

/// 读取全部槽位：每个官方资料片下已导入的版本，以及当前启用的是哪个。
#[tauri::command(async)]
fn list_slots(state: State<'_, AppState>) -> Result<Vec<SlotView>, String> {
    let installation = state.installation.lock().map_err(lock_error)?.clone();
    Ok(state.library.slots(installation.as_ref()))
}

/// 安装前预检一个战役包。
#[tauri::command(async)]
fn inspect_package(path: String) -> Result<PackageInspection, String> {
    package::inspect(Path::new(&path)).map_err(|error| error.to_string())
}

/// 这条导入信息是从哪来的。界面据此决定怎么措辞。
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ImportSource {
    /// 读到了 CCM 或弥音约定的元数据 —— **以数据为准**。
    Metadata,
    /// 没有元数据，靠证据链自动识别 —— **尽力而为**。
    Inferred,
    /// 什么线索都没有，需要用户手动指定。
    Manual,
}

/// 导入预览：预检结果 + 目标战役 + 与已有版本的冲突。
#[derive(Debug, serde::Serialize)]
struct ImportPreview {
    path: String,
    inspection: PackageInspection,
    /// 这条信息是从哪来的。
    source: ImportSource,
    /// 自动判断出的目标战役；None 表示需要用户指定。
    ///
    /// **界面必须始终允许用户改成别的战役** —— 自动识别只是尽力而为，
    /// 用户说了算（import_package 传了 slot 就按传的来）。
    slot: Option<String>,
    /// 与库里已有版本的冲突；None 表示没有冲突。
    conflict: Option<Conflict>,
}

/// 选完文件后的第一步：预检、判断归属、查冲突。**不写任何文件。**
#[tauri::command(async)]
fn prepare_import(path: String, state: State<'_, AppState>) -> Result<ImportPreview, String> {
    let inspection = package::inspect(Path::new(&path)).map_err(|error| error.to_string())?;

    // 补丁不自动挑战役 —— 它是覆盖层，要挂到哪个战役上由用户定
    let slot = match inspection.kind {
        PackageKind::Patch => None,
        PackageKind::Campaign => library::slot_for(&inspection.campaign_type).map(str::to_string),
    };

    let conflict = slot.as_deref().and_then(|slug| {
        library::conflict_for(
            &state.library,
            slug,
            inspection.id.as_deref(),
            inspection.name.as_deref().unwrap_or_default(),
            inspection.version.as_deref(),
        )
    });

    // 有元数据且元数据说得清归属 -> 以数据为准；否则看证据链；都没有就得问用户
    let source = if inspection.identification.is_none()
        && matches!(
            inspection.format,
            miyin_core::campaign::CampaignFormat::Ccm
                | miyin_core::campaign::CampaignFormat::Standard
        )
        && inspection.suggested_slot.as_deref().is_some()
    {
        ImportSource::Metadata
    } else if slot.is_some() {
        ImportSource::Inferred
    } else {
        ImportSource::Manual
    };

    Ok(ImportPreview {
        path,
        inspection,
        source,
        slot,
        conflict,
    })
}

/// 把一个战役包导入到某个槽位。
///
/// - 槽位可以不传：按包内声明自动判断（进化包会归到「虫群之心」）
/// - mode 决定遇到已有同名 / 同 ID 版本时，是**覆盖更新**还是**重命名后导入**
#[tauri::command(async)]
fn import_package(
    path: String,
    slot: Option<String>,
    mode: Option<ImportMode>,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    let inspection = package::inspect(Path::new(&path)).map_err(|error| error.to_string())?;

    let chosen = slot
        .filter(|value| !value.trim().is_empty())
        .or_else(|| library::slot_for(&inspection.campaign_type).map(str::to_string))
        .ok_or_else(|| "无法从包内识别它属于哪个战役，请手动指定".to_string())?;

    library::import(
        &state.library,
        Path::new(&path),
        &chosen,
        mode.unwrap_or_default(),
    )
    .map_err(|error| error.to_string())
}

/// 启用某个版本；variantId 传 null 表示切回**原版战役**。
#[tauri::command(async)]
fn activate_variant(
    slot: String,
    variant_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let installation = require_installation(&state)?;
    library::activate(&state.library, &installation, &slot, variant_id.as_deref())
        .map_err(|error| error.to_string())
}

/// 列出某个版本里的地图（自制战役主要用这个）。
///
/// 地图**不进游戏目录**，就躺在库里 —— 界面把它们列出来，
/// 用户挑一张交给编辑器打开。
#[tauri::command(async)]
fn variant_maps(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MapEntry>, String> {
    Ok(state.library.variant_maps(&slot, &variant_id))
}

/// 列出某个版本里的模组，并标出各自挂没挂载。
#[tauri::command(async)]
fn variant_mods(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModEntry>, String> {
    Ok(state.library.variant_mods(&slot, &variant_id))
}

/// 改某个版本的挂载模组清单。
#[tauri::command(async)]
fn set_mounted_mods(
    slot: String,
    variant_id: String,
    mods: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    state
        .library
        .set_mounted_mods(&slot, &variant_id, &mods)
        .map_err(|error| error.to_string())
}

/// 改某个版本的主地图；传 `null` 表示清空。
#[tauri::command(async)]
fn set_main_map(
    slot: String,
    variant_id: String,
    map: Option<String>,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    state
        .library
        .set_main_map(&slot, &variant_id, map.as_deref())
        .map_err(|error| error.to_string())
}

/// 这个版本该用哪张地图作为游玩入口。
#[tauri::command(async)]
fn main_map_choice(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<MainMapChoice, String> {
    let variant = state
        .library
        .variant(&slot, &variant_id)
        .ok_or_else(|| format!("库里找不到版本 {variant_id}"))?;
    let maps = state.library.variant_maps(&slot, &variant_id);

    Ok(library::resolve_main_map(
        &maps,
        variant.main_map.as_deref(),
    ))
}

/// 启动编辑器打开某张地图的结果。
#[derive(Debug, Clone, serde::Serialize)]
struct EditorLaunch {
    /// 用的是哪个编辑器。
    editor: String,
    /// 打开了哪张地图（相对路径）。
    map: String,
    /// 用户接下来要自己做什么 —— 界面照着念。
    guidance: String,
}

/// 用编辑器打开自制战役里的某张地图。
///
/// 两步，顺序不能反：
///
/// 1. **先把挂载的模组铺进游戏目录** —— 地图里写死了 `Mods\xxx.SC2Mod` 依赖，
///    模组不在位，打开就是一堆丢失的资源
/// 2. 再用编辑器打开地图
///
/// 之后**不代劳进游戏**：编辑器起来后要用户自己按 Ctrl+F9（测试文档），
/// 所以我们把这句话原样返回给界面去念。
#[tauri::command(async)]
fn open_map_in_editor(
    slot: String,
    variant_id: String,
    map: String,
    state: State<'_, AppState>,
) -> Result<EditorLaunch, String> {
    let installation = require_installation(&state)?;
    let editor = installation
        .editor
        .clone()
        .ok_or_else(|| "没找到游戏编辑器 —— 到「设置」里重新指定游戏目录试试".to_string())?;

    // 带了模组却一个都没挂：先拦住，别让用户白白等编辑器起来再报错
    let mods = state.library.variant_mods(&slot, &variant_id);
    if !mods.is_empty() && !mods.iter().any(|item| item.mounted) {
        return Err(
            "这个战役带了模组，但你一个都没挂载 —— 这样打开地图会报错。先在「挂载模组」里勾上再试"
                .to_string(),
        );
    }

    // 先把挂载的模组铺进 <游戏>/Mods/
    library::activate(&state.library, &installation, &slot, Some(&variant_id))
        .map_err(|error| error.to_string())?;

    let path = state
        .library
        .map_path(&slot, &variant_id, &map)
        .map_err(|error| error.to_string())?;

    std::process::Command::new(&editor)
        .arg(&path)
        .spawn()
        .map_err(|error| format!("启动编辑器失败：{error}"))?;

    Ok(EditorLaunch {
        editor: editor.display().to_string(),
        map,
        guidance: "编辑器已打开这张地图，按 Ctrl+F9（菜单「测试文档」）就能进入游戏。".to_string(),
    })
}

/// 可选工具的清单与安装状态。
#[tauri::command(async)]
fn list_tools(state: State<'_, AppState>) -> Result<Vec<ToolStatus>, String> {
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
fn ensure_tool(id: String, state: State<'_, AppState>) -> Result<tools::ToolEnsure, String> {
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
fn tool_releases(id: String, state: State<'_, AppState>) -> Result<Vec<ToolRelease>, String> {
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
fn install_tool(
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
fn uninstall_tool(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let spec = find_tool(&id)?;
    tools::uninstall(state.library.root(), &spec).map_err(|error| error.to_string())
}

/// 在系统浏览器里打开工具的仓库。
#[tauri::command(async)]
fn open_tool_repo(id: String) -> Result<(), String> {
    let spec = find_tool(&id)?;
    let url = format!("https://github.com/{}", spec.repo);
    open_in_browser(&url).map_err(|error| format!("打不开浏览器：{error}"))
}

/// 独立模组库：列表。
#[tauri::command(async)]
fn list_standalone_mods(state: State<'_, AppState>) -> Result<Vec<StandaloneMod>, String> {
    Ok(mods::list(state.library.root()))
}

/// 选一个模组包：文件（压缩包 / .SC2Mod）或文件夹。
#[tauri::command(async)]
fn pick_mod_source(kind: String) -> Option<String> {
    let dialog = rfd::FileDialog::new().set_title("选择模组包");

    if kind == "folder" {
        return dialog
            .pick_folder()
            .map(|path| path.to_string_lossy().into_owned());
    }

    dialog
        .add_filter(
            "模组包",
            &[
                "zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "xz", "sc2mod",
            ],
        )
        .add_filter("所有文件", &["*"])
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 导入一个独立模组包（目录 / .SC2Mod / 压缩包都行）。
#[tauri::command(async)]
fn import_mod(path: String, state: State<'_, AppState>) -> Result<StandaloneMod, String> {
    mods::import(state.library.root(), Path::new(&path)).map_err(|error| error.to_string())
}

/// 改模组信息（只改启动器记录的，不动文件）。
#[tauri::command(async)]
fn update_mod(
    id: String,
    changes: ModChanges,
    state: State<'_, AppState>,
) -> Result<StandaloneMod, String> {
    mods::update(state.library.root(), &id, changes).map_err(|error| error.to_string())
}

/// 删掉一个独立模组。
#[tauri::command(async)]
fn remove_mod(id: String, state: State<'_, AppState>) -> Result<(), String> {
    mods::remove(state.library.root(), &id).map_err(|error| error.to_string())
}

/// 把独立模组打包导出成一个 zip，返回写到了哪。
#[tauri::command(async)]
fn export_mod(id: String, state: State<'_, AppState>) -> Result<String, String> {
    let record =
        mods::get(state.library.root(), &id).ok_or_else(|| "这个模组不在库里".to_string())?;

    let target = rfd::FileDialog::new()
        .set_title("导出模组包")
        .set_file_name(format!("{}.zip", record.name))
        .add_filter("压缩包", &["zip"])
        .save_file()
        .ok_or_else(|| "已取消".to_string())?;

    mods::export(state.library.root(), &id, &target).map_err(|error| error.to_string())?;
    Ok(target.to_string_lossy().into_owned())
}

/// 启用 / 停用独立模组。启用会立刻把它铺进 `<游戏>/Mods/`。
#[tauri::command(async)]
fn set_mod_enabled(
    id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<StandaloneMod, String> {
    // 先落状态，再同步 —— 顺序不能反，sync 读的就是这份记录
    mods::set_enabled(state.library.root(), &id, enabled).map_err(|error| error.to_string())?;

    // 有游戏目录就顺手铺进去；没设游戏目录的话只记状态，等设置好再说
    if let Ok(installation) = require_installation(&state) {
        mods::sync(state.library.root(), &installation).map_err(|error| error.to_string())?;
    }

    mods::get(state.library.root(), &id).ok_or_else(|| "这个模组不在库里".to_string())
}

/// **全库模组汇总**：模组管理菜单用。
#[tauri::command(async)]
fn list_library_mods(state: State<'_, AppState>) -> Result<Vec<LibraryMod>, String> {
    let mut rows = state.library.all_mods();

    // 把独立模组并进来 —— 用户要的是**一张表**，不该分两个地方看
    for record in mods::list(state.library.root()) {
        rows.push(LibraryMod {
            slot: String::new(),
            slot_name: "独立模组".to_string(),
            variant_id: String::new(),
            variant_name: String::new(),
            path: record.id.clone(),
            name: record.name.clone(),
            mounted: record.enabled,
            parts: record.parts,
            origin: miyin_core::library::ModOrigin::Standalone,
            required: false,
            standalone_id: Some(record.id.clone()),
        });
    }

    rows.sort_by(|left, right| {
        left.slot_name
            .cmp(&right.slot_name)
            .then_with(|| left.variant_name.cmp(&right.variant_name))
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(rows)
}

/// 游戏目录 Mods/ 里实际放着的模组。
#[tauri::command(async)]
fn list_game_mods(state: State<'_, AppState>) -> Result<Vec<GameModEntry>, String> {
    let installation = require_installation(&state)?;
    Ok(installation.game_mods())
}

/// 某个版本自带的说明文档（PDF）；没有就是 `None`。
#[tauri::command(async)]
fn variant_doc(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<Option<DocInfo>, String> {
    Ok(state.library.variant_doc(&slot, &variant_id))
}

/// 读出说明文档的字节，交给界面渲染。
///
/// 走 IPC 传原始字节（`tauri::ipc::Response`）而不是让 WebView 去读文件：
/// 既不用放开文件系统访问，也省掉 asset 协议的配置。
#[tauri::command(async)]
fn read_doc(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<tauri::ipc::Response, String> {
    let bytes = state
        .library
        .doc_bytes(&slot, &variant_id)
        .map_err(|error| error.to_string())?;

    Ok(tauri::ipc::Response::new(bytes))
}

/// 修改一个已导入版本的元数据（名称 / 作者 / 注册 ID / 描述）。
///
/// 只改启动器自己记录的元数据，不动包内容；改名不会改版本目录，
/// 因此挂在这个版本上的补丁绑定不受影响。
#[tauri::command(async)]
fn update_variant(
    slot: String,
    variant_id: String,
    changes: VariantChanges,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    library::update_variant(&state.library, &slot, &variant_id, changes)
        .map_err(|error| error.to_string())
}

/// 当前程序版本。
#[tauri::command(async)]
fn app_version() -> String {
    miyin_core::update::current_version().to_string()
}

/// 当前生效的网络设置。
#[tauri::command(async)]
fn network_settings(state: State<'_, AppState>) -> Result<NetworkSettings, String> {
    state
        .network
        .lock()
        .map(|guard| guard.clone())
        .map_err(lock_error)
}

/// 改网络设置并落盘。
#[tauri::command(async)]
fn set_network_settings(
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
fn detected_proxy(state: State<'_, AppState>) -> Option<miyin_core::update::net::DetectedProxy> {
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
async fn check_update(
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
async fn download_update(
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
fn apply_update(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
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

/// 用系统默认浏览器打开一个链接。
#[tauri::command(async)]
fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("只允许打开 http(s) 链接".to_string());
    }
    open_in_browser(&url).map_err(|error| error.to_string())
}

/// 把某个版本导出成 CCM 也能读的战役包。
#[tauri::command(async)]
fn export_variant(
    slot: String,
    variant_id: String,
    destination: String,
    merge_patches: Option<bool>,
    state: State<'_, AppState>,
) -> Result<library::export::ExportReport, String> {
    let index = state.library.index();
    let variant = index
        .slots
        .get(&slot)
        .and_then(|entry| entry.variants.iter().find(|v| v.id == variant_id).cloned())
        .ok_or_else(|| "找不到这个版本".to_string())?;

    library::export::export(
        &state.library,
        &slot,
        &variant,
        &library::export::ExportOptions {
            destination: std::path::PathBuf::from(destination),
            merge_patches: merge_patches.unwrap_or(false),
        },
    )
    .map_err(|error| error.to_string())
}

/// 弹出"另存为"对话框选导出位置。
#[tauri::command(async)]
fn pick_export_path(default_name: String) -> Option<String> {
    rfd::FileDialog::new()
        .set_file_name(format!("{default_name}.zip"))
        .add_filter("战役包", &["zip"])
        .save_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 库里全部补丁。
#[tauri::command(async)]
fn list_patches(state: State<'_, AppState>) -> Vec<Patch> {
    state.library.index().patches.into_values().collect()
}

/// 某个战役挂着的补丁（含未启用的），带上它此刻能不能自动匹配。
#[derive(Debug, serde::Serialize)]
struct BoundPatch {
    #[serde(flatten)]
    patch: Patch,
    /// 生效优先级（可能被挂载时改过）。
    priority: i64,
    /// 是否启用。
    enabled: bool,
    /// 按 requires 是否匹配得上这个战役。
    matched: bool,
}

/// 列出某个战役挂着的补丁。
#[tauri::command(async)]
fn list_bindings(slot: String, state: State<'_, AppState>) -> Vec<BoundPatch> {
    library::compose::bindings_of(&state.library, &slot)
        .into_iter()
        .map(|(binding, patch)| BoundPatch {
            matched: library::patch::matches_slot(&state.library, &slot, &patch),
            patch,
            priority: binding.priority,
            enabled: binding.enabled,
        })
        .collect()
}

/// 库里还没挂到某个战役上的补丁（供界面挑选手动挂载）。
#[tauri::command(async)]
fn list_available_patches(slot: String, state: State<'_, AppState>) -> Vec<BoundPatch> {
    let bound: Vec<String> = library::compose::bindings_of(&state.library, &slot)
        .into_iter()
        .map(|(binding, _)| binding.patch_id)
        .collect();

    state
        .library
        .index()
        .patches
        .into_values()
        .filter(|patch| !bound.contains(&patch.id))
        .map(|patch| BoundPatch {
            matched: library::patch::matches_slot(&state.library, &slot, &patch),
            priority: patch.priority,
            enabled: false,
            patch,
        })
        .collect()
}

/// 导入一个补丁包（补丁不归任何战役，导入后需要挂到战役上）。
#[tauri::command(async)]
fn import_patch(path: String, state: State<'_, AppState>) -> Result<Patch, String> {
    library::patch::import_patch(&state.library, Path::new(&path)).map_err(|e| e.to_string())
}

/// 把库里所有**声明了依赖且匹配得上**的补丁自动挂到对应战役上。
#[tauri::command(async)]
fn auto_bind_patches(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    library::patch::auto_bind(&state.library)
        .map(|bound| {
            bound
                .into_iter()
                .map(|(slot, name)| format!("{slot}::{name}"))
                .collect()
        })
        .map_err(|error| error.to_string())
}

/// 把补丁挂到某个战役上。
#[tauri::command(async)]
fn bind_patch(slot: String, patch_id: String, state: State<'_, AppState>) -> Result<(), String> {
    library::patch::bind(&state.library, &slot, &patch_id).map_err(|e| e.to_string())
}

/// 解绑（补丁本身还在库里）。
#[tauri::command(async)]
fn unbind_patch(slot: String, patch_id: String, state: State<'_, AppState>) -> Result<(), String> {
    library::patch::unbind(&state.library, &slot, &patch_id).map_err(|e| e.to_string())
}

/// 改挂载设置：启用状态 / 优先级。**改完需要重新启用一次战役才会生效。**
#[tauri::command(async)]
fn configure_patch(
    slot: String,
    patch_id: String,
    enabled: Option<bool>,
    priority: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Binding, String> {
    library::patch::configure_binding(&state.library, &slot, &patch_id, enabled, priority)
        .map_err(|error| error.to_string())
}

/// 改一个已导入补丁的元数据。
#[tauri::command(async)]
fn update_patch(
    patch_id: String,
    changes: library::patch::PatchChanges,
    state: State<'_, AppState>,
) -> Result<Patch, String> {
    library::patch::update_patch(&state.library, &patch_id, changes).map_err(|e| e.to_string())
}

/// 单独导出一个补丁包。
#[tauri::command(async)]
fn export_patch(
    patch_id: String,
    destination: String,
    state: State<'_, AppState>,
) -> Result<library::export::ExportReport, String> {
    let patch = state
        .library
        .index()
        .patches
        .get(&patch_id)
        .cloned()
        .ok_or_else(|| "找不到这个补丁".to_string())?;

    library::export::export_patch(&state.library, &patch, std::path::Path::new(&destination))
        .map_err(|error| error.to_string())
}

/// 从库里彻底删掉一个补丁。
#[tauri::command(async)]
fn delete_patch(patch_id: String, state: State<'_, AppState>) -> Result<(), String> {
    library::patch::remove_patch(&state.library, &patch_id).map_err(|e| e.to_string())
}

/// 预览某个版本打上当前补丁后的合成清单。
#[tauri::command(async)]
fn preview_composition(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<library::compose::Composition, String> {
    let index = state.library.index();
    let variant = index
        .slots
        .get(&slot)
        .and_then(|entry| entry.variants.iter().find(|v| v.id == variant_id).cloned())
        .ok_or_else(|| "找不到这个版本".to_string())?;

    let kind = library::require_slot(&slot).map_err(|error| error.to_string())?;
    let sub = variant
        .target_sub
        .clone()
        .or_else(|| kind.sub_directory().map(str::to_string));

    Ok(library::compose::compose(
        &state.library,
        &slot,
        &variant,
        sub.as_deref(),
    ))
}

/// 删除库里的某个版本（若正在启用会先切回原版战役）。
#[tauri::command(async)]
fn delete_variant(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let installation = require_installation(&state)?;
    library::remove_variant(&state.library, &installation, &slot, &variant_id)
        .map_err(|error| error.to_string())
}

/// 启动游戏。
#[tauri::command(async)]
fn launch_game(state: State<'_, AppState>) -> Result<(), String> {
    let installation = require_installation(&state)?;
    let launcher = installation.preferred_launcher().to_path_buf();

    std::process::Command::new(&launcher)
        .current_dir(&installation.root)
        .spawn()
        .map_err(|error| format!("启动失败：{error}"))?;

    Ok(())
}

/// 在资源管理器中定位文件或目录。
#[tauri::command(async)]
fn reveal_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(format!("路径不存在：{path}"));
    }

    reveal_in_file_manager(&target)
}

/// Windows：用资源管理器选中目标。
#[cfg(windows)]
fn reveal_in_file_manager(target: &Path) -> Result<(), String> {
    // explorer 对 /select 的路径比较挑剔，统一用反斜杠
    let argument = format!("/select,{}", target.display());
    std::process::Command::new("explorer")
        .arg(argument)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开资源管理器：{error}"))
}

/// 其它平台暂不支持。
#[cfg(not(windows))]
fn reveal_in_file_manager(_target: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开资源管理器".to_string())
}

/// 弹出文件选择框，返回用户选中的战役包路径。
#[tauri::command(async)]
fn pick_package() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择战役包 / 补丁包")
        // 后端借助系统自带的 tar（bsdtar/libarchive）能读一大片格式，
        // 所以对话框里就该把它们都列出来，而不是只给个 zip 让人以为不支持。
        .add_filter(
            "压缩包",
            &["zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "xz", "zst", "cab", "iso"],
        )
        .add_filter("所有文件", &["*"])
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 弹出目录选择框，返回用户选中的游戏目录。
#[tauri::command(async)]
fn pick_game_directory() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择星际争霸 II 安装目录")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 读取战役目录内的封面图，返回 data URL 供界面直接显示。
#[tauri::command(async)]
fn campaign_cover(dir: String) -> Option<String> {
    let cover = scanner::find_cover_for(&PathBuf::from(dir))?;
    file_to_data_url(&cover)
}

/// 读取库里某个版本自带的封面图。
///
/// 返回 `None` 表示这个包没配封面，界面会退回该战役的官方美术。
#[tauri::command(async)]
fn variant_cover(slot: String, variant_id: String, state: State<'_, AppState>) -> Option<String> {
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

/// 启动应用。
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_path = app
                .path()
                .app_config_dir()
                .map(|dir| dir.join("config.json"))
                .unwrap_or_else(|_| PathBuf::from("miyin-config.json"));

            // 战役库放在**软件同级目录**下（绿色版，随软件走）
            let data_root = std::env::current_exe()
                .ok()
                .map(|executable| miyin_core::library::default_root(&executable))
                .unwrap_or_else(|| PathBuf::from("data"));

            app.manage(AppState::bootstrap(config_path, data_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_installation,
            set_installation,
            library_root,
            list_slots,
            export_variant,
            pick_export_path,
            app_version,
            network_settings,
            set_network_settings,
            detected_proxy,
            check_update,
            download_update,
            apply_update,
            open_url,
            list_patches,
            list_bindings,
            list_available_patches,
            import_patch,
            auto_bind_patches,
            bind_patch,
            unbind_patch,
            configure_patch,
            update_patch,
            export_patch,
            delete_patch,
            preview_composition,
            inspect_package,
            prepare_import,
            import_package,
            activate_variant,
            variant_maps,
            variant_mods,
            set_mounted_mods,
            set_main_map,
            main_map_choice,
            open_map_in_editor,
            variant_doc,
            list_library_mods,
            list_standalone_mods,
            list_tools,
            ensure_tool,
            tool_releases,
            install_tool,
            uninstall_tool,
            open_tool_repo,
            pick_mod_source,
            import_mod,
            update_mod,
            remove_mod,
            export_mod,
            set_mod_enabled,
            list_game_mods,
            read_doc,
            update_variant,
            delete_variant,
            launch_game,
            reveal_path,
            pick_package,
            pick_game_directory,
            campaign_cover,
            variant_cover,
        ])
        .run(tauri::generate_context!())
        .expect("弥音启动器启动失败");
}
