//! 弥音启动器 Tauri 后端。
//!
//! 这一层**只做三件事**：持有配置与状态、把命令转发给 `miyin-core`、
//! 把领域错误转成可以展示的字符串。所有业务规则都在 `miyin-core` 里
//! （见 `AGENTS.md` 的依赖方向约定）。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use miyin_core::campaign::metadata::PackageKind;
use miyin_core::campaign::package::{self, PackageInspection};
use miyin_core::campaign::scanner;
use miyin_core::library::{self, Conflict, ImportMode, Library, SlotView, Variant, VariantChanges};
use miyin_core::sc2::{DiscoverySource, Installation};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

/// 持久化配置。
#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    /// 用户手动指定的游戏根目录。
    game_root: Option<PathBuf>,
}

/// 应用运行期状态。
struct AppState {
    installation: Mutex<Option<Installation>>,
    config_path: Mutex<Option<PathBuf>>,
    /// 战役库：存放玩家导入的各个版本（与游戏目录解耦）。
    library: Library,
}

impl AppState {
    /// 读取配置并尝试恢复上次使用的安装。
    fn bootstrap(config_path: PathBuf, library_root: PathBuf) -> Self {
        let config = load_config(&config_path);

        let installation = config
            .game_root
            .and_then(|root| Installation::from_root(root, DiscoverySource::Manual).ok())
            .or_else(|| Installation::discover().ok());

        Self {
            installation: Mutex::new(installation),
            config_path: Mutex::new(Some(config_path)),
            library: Library::new(library_root),
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
#[tauri::command]
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
#[tauri::command]
fn set_installation(path: String, state: State<'_, AppState>) -> Result<Installation, String> {
    let installation = Installation::from_root(&path, DiscoverySource::Manual)
        .map_err(|error| error.to_string())?;

    state.remember(&installation);
    *state.installation.lock().map_err(lock_error)? = Some(installation.clone());
    Ok(installation)
}

/// 战役库根目录（软件同级的 data 目录）。
#[tauri::command]
fn library_root(state: State<'_, AppState>) -> String {
    state.library.root().to_string_lossy().into_owned()
}

/// 读取全部槽位：每个官方资料片下已导入的版本，以及当前启用的是哪个。
#[tauri::command]
fn list_slots(state: State<'_, AppState>) -> Result<Vec<SlotView>, String> {
    let installation = state.installation.lock().map_err(lock_error)?.clone();
    Ok(state.library.slots(installation.as_ref()))
}

/// 安装前预检一个战役包。
#[tauri::command]
fn inspect_package(path: String) -> Result<PackageInspection, String> {
    package::inspect(Path::new(&path)).map_err(|error| error.to_string())
}

/// 导入预览：预检结果 + 目标战役 + 与已有版本的冲突。
#[derive(Debug, serde::Serialize)]
struct ImportPreview {
    path: String,
    inspection: PackageInspection,
    /// 自动判断出的目标战役；None 表示需要用户指定。
    slot: Option<String>,
    /// 与库里已有版本的冲突；None 表示没有冲突。
    conflict: Option<Conflict>,
}

/// 选完文件后的第一步：预检、判断归属、查冲突。**不写任何文件。**
#[tauri::command]
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

    Ok(ImportPreview {
        path,
        inspection,
        slot,
        conflict,
    })
}

/// 把一个战役包导入到某个槽位。
///
/// - 槽位可以不传：按包内声明自动判断（进化包会归到「虫群之心」）
/// - mode 决定遇到已有同名 / 同 ID 版本时，是**覆盖更新**还是**重命名后导入**
#[tauri::command]
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
#[tauri::command]
fn activate_variant(
    slot: String,
    variant_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let installation = require_installation(&state)?;
    library::activate(&state.library, &installation, &slot, variant_id.as_deref())
        .map_err(|error| error.to_string())
}

/// 修改一个已导入版本的元数据（名称 / 作者 / 注册 ID / 描述）。
///
/// 只改启动器自己记录的元数据，不动包内容；改名不会改版本目录，
/// 因此挂在这个版本上的补丁绑定不受影响。
#[tauri::command]
fn update_variant(
    slot: String,
    variant_id: String,
    changes: VariantChanges,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    library::update_variant(&state.library, &slot, &variant_id, changes)
        .map_err(|error| error.to_string())
}

/// 删除库里的某个版本（若正在启用会先切回原版战役）。
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
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
#[tauri::command]
fn pick_package() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择战役包")
        .add_filter("星际争霸 II 战役包", &["zip"])
        .add_filter("所有文件", &["*"])
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 弹出目录选择框，返回用户选中的游戏目录。
#[tauri::command]
fn pick_game_directory() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择星际争霸 II 安装目录")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 读取战役目录内的封面图，返回 data URL 供界面直接显示。
#[tauri::command]
fn campaign_cover(dir: String) -> Option<String> {
    let cover = scanner::find_cover_for(&PathBuf::from(dir))?;
    file_to_data_url(&cover)
}

/// 读取库里某个版本自带的封面图。
///
/// 返回 `None` 表示这个包没配封面，界面会退回该战役的官方美术。
#[tauri::command]
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
            inspect_package,
            prepare_import,
            import_package,
            activate_variant,
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
