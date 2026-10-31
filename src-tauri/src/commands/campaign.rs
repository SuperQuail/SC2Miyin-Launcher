//! 战役：导入 / 启用 / 删除 / 改版元数据 / 地图 / 编辑器 / 游戏启动。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use std::path::{Path, PathBuf};

use miyin_core::campaign::metadata::PackageKind;
use miyin_core::campaign::package::{self, PackageInspection};
use miyin_core::library::{
    self, Conflict, DocInfo, ImportMode, MainMapChoice, MapEntry, ModEntry, SlotView, Variant,
    VariantChanges,
};
use tauri::State;

use crate::{
    AppState, ensure_game_not_running, installed_target, lock_error, require_installation,
};

/// 游戏是不是正跑着。启用 / 停用前先问它。
#[tauri::command(async)]
pub(crate) fn sc2_running() -> Option<String> {
    miyin_core::platform::game_running()
}

/// 启用前先看：这次会往游戏目录里放什么、覆盖什么、删什么。**不写盘。**
///
/// 写盘闸门放开到「安装目录里任意位置」之后，这就是兜底的那一眼 ——
/// 界面拿它铺确认框，用户点头了才真的 `activate`。
#[tauri::command(async)]
pub(crate) fn preview_activation(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<library::Preview, String> {
    let guard = state.installation.lock().map_err(lock_error)?;
    let installation = guard.as_ref().ok_or("还没找到星际争霸 II 的安装目录")?;
    library::preview(&state.library, installation, &slot, &variant_id)
        .map_err(|error| error.to_string())
}

/// 启用前的冲突检查：要写的目标里有哪些正被**另一个已启用战役**占着。
///
/// **只读**，一个字节都不写 —— 界面必须等用户在这之后点头，才轮到 `activate_variant`。
#[tauri::command(async)]
pub(crate) fn activation_conflicts(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<miyin_core::library::ActiveConflict>, String> {
    let installation = require_installation(&state)?;
    library::conflicts(&state.library, &installation, &slot, &variant_id)
        .map_err(|error| error.to_string())
}

/// 战役库根目录（软件同级的 data 目录）。
#[tauri::command(async)]
pub(crate) fn library_root(state: State<'_, AppState>) -> String {
    state.library.root().to_string_lossy().into_owned()
}

/// 读取全部槽位：每个官方资料片下已导入的版本，以及当前启用的是哪个。
#[tauri::command(async)]
pub(crate) fn list_slots(state: State<'_, AppState>) -> Result<Vec<SlotView>, String> {
    let installation = state.installation.lock().map_err(lock_error)?.clone();
    Ok(state.library.slots(installation.as_ref()))
}

/// 安装前预检一个战役包。
#[tauri::command(async)]
pub(crate) fn inspect_package(path: String) -> Result<PackageInspection, String> {
    package::inspect(Path::new(&path)).map_err(|error| error.to_string())
}

/// 这条导入信息是从哪来的。界面据此决定怎么措辞。
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImportSource {
    /// 读到了 CCM 或弥音约定的元数据 —— **以数据为准**。
    Metadata,
    /// 没有元数据，靠证据链自动识别 —— **尽力而为**。
    Inferred,
    /// 什么线索都没有，需要用户手动指定。
    Manual,
}

/// 导入预览：预检结果 + 目标战役 + 与已有版本的冲突。
#[derive(Debug, serde::Serialize)]
pub(crate) struct ImportPreview {
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
///
/// `entry` 是**用户从哪个入口点的导入**：
///
/// - `None` / `campaign`：正常走识别链
/// - `custom`：用户已经站在「自制战役」页了，**意图够明确，不再判断归属**。
///   没有元数据也不去猜它属于哪部原版战役 —— 它就是自制战役。
///   （踩过：在这里照样跑识别，一个没元数据的包被认出「虚空之遗」，
///   于是用户在自制战役页点导入，战役却进了虚空之遗。）
#[tauri::command(async)]
pub(crate) fn prepare_import(
    path: String,
    entry: Option<String>,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<ImportPreview, String> {
    let inspection = package::inspect_with_password(Path::new(&path), password.as_deref())
        .map_err(|error| error.to_string())?;

    let from_custom = entry.as_deref() == Some("custom");

    let slot = match inspection.kind {
        // 补丁不自动挑战役 —— 它是覆盖层，要挂到哪个战役上由用户定
        PackageKind::Patch => None,
        // 用户站在自制战役页，意图明确，不用猜
        _ if from_custom => Some("custom".to_string()),
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
    let source = if from_custom {
        // 用户自己选的「自制战役」，不是识别出来的，别把它说成自动识别
        ImportSource::Manual
    } else if inspection.identification.is_none()
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
pub(crate) fn import_package(
    path: String,
    slot: Option<String>,
    mode: Option<ImportMode>,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    let inspection = package::inspect_with_password(Path::new(&path), password.as_deref())
        .map_err(|error| error.to_string())?;

    let chosen = slot
        .filter(|value| !value.trim().is_empty())
        .or_else(|| library::slot_for(&inspection.campaign_type).map(str::to_string))
        .ok_or_else(|| "无法从包内识别它属于哪个战役，请手动指定".to_string())?;

    library::import_with_password(
        &state.library,
        Path::new(&path),
        &chosen,
        mode.unwrap_or_default(),
        password.as_deref(),
    )
    .map_err(|error| error.to_string())
}

/// 启用某个版本；variantId 传 null 表示切回**原版战役**。
#[tauri::command(async)]
pub(crate) fn activate_variant(
    slot: String,
    variant_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    ensure_game_not_running()?;
    let installation = require_installation(&state)?;
    // 存档组由 core 的 activate 统一跟着切（所有会 activate 的路径都走它），
    // 命令层不要再切一次 —— 否则每次启用都多切一轮、多一份「还原前」备份。
    library::activate(&state.library, &installation, &slot, variant_id.as_deref())
        .map_err(|error| error.to_string())
}

/// 停用**全部**战役：撤掉启动器写进游戏目录的所有战役文件，索引与存档组回原版。
///
/// 「全部还原」入口用它 —— 用户要一个一键把游戏目录恢复成本体状态的动作，
/// 而不是逐个槽位点停用（历史上多装 / 半装的也能一并清掉）。不删战役库里的内容。
#[tauri::command(async)]
pub(crate) fn deactivate_all_campaigns(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    ensure_game_not_running()?;
    let installation = require_installation(&state)?;
    library::deactivate(&state.library, &installation).map_err(|error| error.to_string())
}

/// 列出某个版本里的地图（自制战役主要用这个）。
///
/// 地图**不进游戏目录**，就躺在库里 —— 界面把它们列出来，
/// 用户挑一张交给编辑器打开。
#[tauri::command(async)]
pub(crate) fn variant_maps(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MapEntry>, String> {
    Ok(state.library.variant_maps(&slot, &variant_id))
}

/// 列出某个版本里的模组，并标出各自挂没挂载。
#[tauri::command(async)]
pub(crate) fn variant_mods(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModEntry>, String> {
    Ok(state.library.variant_mods(&slot, &variant_id))
}

/// 改某个版本的挂载模组清单。
#[tauri::command(async)]
pub(crate) fn set_mounted_mods(
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
pub(crate) fn set_main_map(
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
pub(crate) fn main_map_choice(
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
pub(crate) struct EditorLaunch {
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
pub(crate) fn open_map_in_editor(
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

    // **打开游戏目录里那一份**，不是库里那一份。
    //
    // 库里那份只是留底（切回原版时能还原）。游戏和编辑器要读的是装好的那份 ——
    // 而且复刻战役的启动器地图里写的是 GameSetNextMap("Starcraft Mass Recall/…")，
    // 这种路径**相对 Maps/**，只有装好的那份才处在正确的位置上。
    // 实测从库里打开会报「无法打开地图」。
    let library_path = state
        .library
        .map_path(&slot, &variant_id, &map)
        .map_err(|error| error.to_string())?;

    // **地图已经在游戏目录里在位，就不要再激活。** 点「用编辑器打开」只是为了看一张图，
    // 顺手把整个战役重装一遍（撤下当前版本、铺这一版）不是用户点的那个动作。
    let first = installed_target(&state, &installation, &library_path);
    let (path, switched, stale) = match first {
        Ok(Some(path)) => (path, false, None),
        // 不在位（或清单和磁盘对不上）才装这一次。它会切换当前启用的版本，话得跟用户说清楚。
        first => {
            ensure_game_not_running()?;
            library::activate(&state.library, &installation, &slot, Some(&variant_id))
                .map_err(|error| error.to_string())?;
            match installed_target(&state, &installation, &library_path) {
                Ok(Some(path)) => (path, true, None),
                // 装完还是反查不到：退回库里那份（它在游戏目录外，多半打不开），并且说出来
                again => (
                    library_path,
                    true,
                    Some(again.err().or(first.err()).unwrap_or_else(missing_record)),
                ),
            }
        }
    };

    std::process::Command::new(&editor)
        .arg(&path)
        .spawn()
        .map_err(|error| format!("启动编辑器失败：{error}"))?;

    Ok(EditorLaunch {
        editor: editor.display().to_string(),
        map,
        guidance: match stale {
            // 账实对不上：库里那份在游戏目录之外，得说清楚为什么可能打不开
            Some(reason) => format!(
                "{reason}。已改用库里那份打开 —— 它在游戏目录外，可能打不开；到战役页重新启用一次这个版本可以修好。"
            ),
            None if switched => "编辑器已打开这张地图，按 Ctrl+F9（菜单「测试文档」）就能进入游戏。注意：它刚才不在游戏目录里，所以已经把它所在的版本装了进去 —— 当前启用的战役跟着换了。".to_string(),
            None => "编辑器已打开这张地图，按 Ctrl+F9（菜单「测试文档」）就能进入游戏。".to_string(),
        },
    })
}

/// 某个版本自带的说明文档（PDF）；没有就是 `None`。
#[tauri::command(async)]
pub(crate) fn variant_doc(
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
pub(crate) fn read_doc(
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
pub(crate) fn update_variant(
    slot: String,
    variant_id: String,
    changes: VariantChanges,
    state: State<'_, AppState>,
) -> Result<Variant, String> {
    library::update_variant(&state.library, &slot, &variant_id, changes)
        .map_err(|error| error.to_string())
}

/// 把某个版本导出成战役包（弥音包 / 星际枢纽兼容包）。
#[tauri::command(async)]
pub(crate) fn export_variant(
    slot: String,
    variant_id: String,
    destination: String,
    merge_patches: Option<bool>,
    format: Option<String>,
    state: State<'_, AppState>,
) -> Result<library::export::ExportReport, String> {
    let variant = find_variant(&state, &slot, &variant_id)?;
    let format = parse_export_format(format.as_deref())?;

    library::export::export(
        &state.library,
        &slot,
        &variant,
        &library::export::ExportOptions {
            destination: std::path::PathBuf::from(destination),
            merge_patches: merge_patches.unwrap_or(false),
            format,
        },
    )
    .map_err(|error| error.to_string())
}

/// 导出预览：会带什么、会丢什么。**不写盘。**
#[tauri::command(async)]
pub(crate) fn preview_export_variant(
    slot: String,
    variant_id: String,
    merge_patches: Option<bool>,
    format: Option<String>,
    state: State<'_, AppState>,
) -> Result<library::export::ExportPreview, String> {
    let variant = find_variant(&state, &slot, &variant_id)?;
    let format = parse_export_format(format.as_deref())?;
    library::export::preview(
        &state.library,
        &slot,
        &variant,
        merge_patches.unwrap_or(false),
        format,
    )
    .map_err(|error| error.to_string())
}

/// 找到库里的某个版本。
fn find_variant(
    state: &State<'_, AppState>,
    slot: &str,
    variant_id: &str,
) -> Result<Variant, String> {
    state
        .library
        .index()
        .slots
        .get(slot)
        .and_then(|entry| {
            entry
                .variants
                .iter()
                .find(|item| item.id == variant_id)
                .cloned()
        })
        .ok_or_else(|| "找不到这个版本".to_string())
}

/// 解析前端传来的导出格式；不认识就报错，别默默当弥音包。
fn parse_export_format(raw: Option<&str>) -> Result<library::export::ExportFormat, String> {
    match raw {
        None | Some("") => Ok(library::export::ExportFormat::Miyin),
        Some(value) => library::export::ExportFormat::parse(value)
            .ok_or_else(|| format!("不认识的导出格式：{value}")),
    }
}

/// 弹出"另存为"对话框选导出位置。
#[tauri::command(async)]
pub(crate) fn pick_export_path(default_name: String) -> Option<String> {
    rfd::FileDialog::new()
        .set_file_name(format!("{default_name}.zip"))
        .add_filter("战役包", &["zip"])
        .save_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 删除库里的某个版本（若正在启用会先切回原版战役）。
#[tauri::command(async)]
pub(crate) fn delete_variant(
    slot: String,
    variant_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    ensure_game_not_running()?;
    let installation = require_installation(&state)?;
    library::remove_variant(&state.library, &installation, &slot, &variant_id)
        .map_err(|error| error.to_string())
}

/// 启动游戏。
#[tauri::command(async)]
pub(crate) fn launch_game(state: State<'_, AppState>) -> Result<(), String> {
    let installation = require_installation(&state)?;

    // 模组由启动器负责：点「启动游戏」也先把已启用版本的模组铺齐 ——
    // 不能只靠启动时那一次（用户可能刚挂了新模组）。失败不拦启动。
    if let Err(error) = miyin_core::library::activation::sync_mods(&state.library, &installation) {
        eprintln!("[miyin] 启动前的模组挂载失败：{error}");
    }
    let _ = miyin_core::library::mods::sync(state.library.root(), &installation);

    // 铺完再核对一遍：清单里记着、游戏目录里文件却缺了的战役 —— 这时候进游戏只会进原版，
    // 用户还以为在玩改版。缺了就拦住并指明去哪一页重新启用（**不在这里偷偷补拷地图**：
    // 启动时改 Maps/ 比提示他一趟贵得多）。
    let missing =
        miyin_core::library::activation::incomplete_installation(&state.library, &installation);
    if !missing.is_empty() {
        return Err(format!(
            "游戏目录里的战役文件不完整，请到对应战役页重新启用：\n{}",
            missing.join("\n")
        ));
    }

    let launcher = installation.preferred_launcher().to_path_buf();

    std::process::Command::new(&launcher)
        .current_dir(&installation.root)
        .spawn()
        .map_err(|error| format!("启动失败：{error}"))?;

    Ok(())
}

/// 弹出文件选择框，返回用户选中的战役包路径。
#[tauri::command(async)]
pub(crate) fn pick_package() -> Option<String> {
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

/// 弹出文件夹选择框，返回用户选中的战役包目录。
///
/// 解开的战役包（作者直接发的文件夹）走这条路 —— 后端就着盘读，不先复制一份。
#[tauri::command(async)]
pub(crate) fn pick_package_folder() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选择战役包文件夹")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command(async)]
pub(crate) fn launch_game_with_map(
    slot: String,
    variant_id: String,
    map: String,
    state: State<'_, AppState>,
) -> Result<GameLaunch, String> {
    let (installation, path, stale) = prepared_map(&state, &slot, &variant_id, &map)?;

    // 工作目录必须是游戏根 —— 切换器靠它找 .build.info 与 Versions/
    std::process::Command::new(installation.preferred_launcher())
        .current_dir(&installation.root)
        .arg(&path)
        .spawn()
        .map_err(|error| format!("启动游戏失败：{error}"))?;

    Ok(GameLaunch {
        map,
        guidance: match stale {
            // 账实对不上：库里那份在游戏目录之外，星际争霸多半加载不了
            Some(reason) => format!(
                "{reason}。已改用库里那份启动 —— 它在游戏目录外，星际争霸多半加载不了；到战役页重新启用一次这个版本可以修好。"
            ),
            None => "已经交给星际争霸，正在加载这张地图。".to_string(),
        },
    })
}

#[tauri::command(async)]
pub(crate) fn update_variant_from_package(
    slot: String,
    variant_id: Option<String>,
    path: String,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<miyin_core::library::Replaced, String> {
    ensure_game_not_running()?;
    let installation = require_installation(&state)?;

    // 换哪一个**已安装的包**：
    // - 点明了（在那一版的卡片上点「更新」）→ 就是它
    // - 没点明（主页面「更新战役包…」）→ 按包自己的身份去库里找；
    //   找不到说明库里还没有这一版，那就装成新的
    let target = match variant_id {
        Some(id) => Some(id),
        None => library::find_target_for_package_with_password(
            &state.library,
            &slot,
            Path::new(&path),
            password.as_deref(),
        )
        .map_err(|error| error.to_string())?
        .map(|variant| variant.id),
    };

    match target {
        Some(id) => library::replace_variant_with_password(
            &state.library,
            &installation,
            &slot,
            &id,
            Path::new(&path),
            password.as_deref(),
        ),
        None => library::import_with_password(
            &state.library,
            Path::new(&path),
            &slot,
            miyin_core::library::ImportMode::Rename,
            password.as_deref(),
        )
        .map(|variant| miyin_core::library::Replaced {
            variant,
            added_mods: Vec::new(),
        }),
    }
    .map_err(|error| error.to_string())
}

fn prepared_map(
    state: &State<'_, AppState>,
    slot: &str,
    variant_id: &str,
    map: &str,
) -> Result<(miyin_core::sc2::Installation, PathBuf, Option<String>), String> {
    ensure_game_not_running()?;
    let installation = require_installation(state)?;

    library::activate(&state.library, &installation, slot, Some(variant_id))
        .map_err(|error| error.to_string())?;

    let library_path = state
        .library
        .map_path(slot, variant_id, map)
        .map_err(|error| error.to_string())?;

    // 装完还是反查不到：退回库里那份（它在游戏目录外，多半加载不了），原因带回给界面
    let (path, stale) = match installed_target(state, &installation, &library_path) {
        Ok(Some(path)) => (path, None),
        again => (
            library_path,
            Some(again.err().unwrap_or_else(missing_record)),
        ),
    };

    Ok((installation, path, stale))
}

/// 清单里压根没有这条记录时的说法（`target_of` 返回 `Ok(None)`，而这里刚装过）。
fn missing_record() -> String {
    "清单里没有这张图的安装记录".to_string()
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct GameLaunch {
    /// 要打开哪张地图（库内相对路径）。
    map: String,
    /// 界面照着念的一句话。
    guidance: String,
}
