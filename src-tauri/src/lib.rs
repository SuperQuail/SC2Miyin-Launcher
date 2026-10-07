//! 弥音启动器 Tauri 后端。
//!
//! 这一层**只做三件事**：持有配置与状态、把命令转发给 `miyin-core`、
//! 把领域错误转成可以展示的字符串。所有业务规则都在 `miyin-core` 里
//! （见 `AGENTS.md` 的依赖方向约定）。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use miyin_core::campaign::Campaign;
use miyin_core::campaign::installer;
use miyin_core::campaign::package::{self, PackageInspection};
use miyin_core::campaign::scanner;
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
#[derive(Default)]
struct AppState {
    installation: Mutex<Option<Installation>>,
    config_path: Mutex<Option<PathBuf>>,
}

impl AppState {
    /// 读取配置并尝试恢复上次使用的安装。
    fn bootstrap(config_path: PathBuf) -> Self {
        let config = load_config(&config_path);

        let installation = config
            .game_root
            .and_then(|root| Installation::from_root(root, DiscoverySource::Manual).ok())
            .or_else(|| Installation::discover().ok());

        Self {
            installation: Mutex::new(installation),
            config_path: Mutex::new(Some(config_path)),
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

/// 列出全部已安装战役（含核对结论）。
#[tauri::command]
fn list_campaigns(state: State<'_, AppState>) -> Result<Vec<Campaign>, String> {
    let installation = require_installation(&state)?;
    // 启用状态由激活模块维护；本版本尚未启用该功能，先给空集合。
    Ok(scanner::scan(&installation, &Default::default()))
}

/// 安装前预检一个战役包。
#[tauri::command]
fn inspect_package(path: String) -> Result<PackageInspection, String> {
    package::inspect(Path::new(&path)).map_err(|error| error.to_string())
}

/// 安装一个战役包。
#[tauri::command]
fn install_package(path: String, state: State<'_, AppState>) -> Result<Campaign, String> {
    let installation = require_installation(&state)?;
    installer::install(&installation, Path::new(&path)).map_err(|error| error.to_string())
}

/// 卸载一个战役。
#[tauri::command]
fn uninstall_campaign(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let installation = require_installation(&state)?;
    installer::uninstall(&installation, &id).map_err(|error| error.to_string())
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
    const MAX_BYTES: u64 = 6 * 1024 * 1024;

    let directory = PathBuf::from(dir);
    let cover = scanner::find_cover_for(&directory)?;

    let metadata = std::fs::metadata(&cover).ok()?;
    if metadata.len() > MAX_BYTES {
        return None;
    }

    let bytes = std::fs::read(&cover).ok()?;
    let mime = match cover
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

            app.manage(AppState::bootstrap(config_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            detect_installation,
            set_installation,
            list_campaigns,
            inspect_package,
            install_package,
            uninstall_campaign,
            launch_game,
            reveal_path,
            pick_package,
            pick_game_directory,
            campaign_cover,
        ])
        .run(tauri::generate_context!())
        .expect("弥音启动器启动失败");
}
