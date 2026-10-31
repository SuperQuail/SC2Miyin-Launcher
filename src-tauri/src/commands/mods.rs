//! 模组：独立模组库的导入 / 元数据 / 启停 / 导出，以及全库汇总。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use std::path::Path;

use miyin_core::campaign::package;
use miyin_core::library::{LibraryMod, ModChanges, StandaloneMod, mods};
use miyin_core::sc2::GameModEntry;
use tauri::State;

use crate::{AppState, ensure_game_not_running, require_installation};

/// 独立模组库：列表。
#[tauri::command(async)]
pub(crate) fn list_standalone_mods(
    state: State<'_, AppState>,
) -> Result<Vec<StandaloneMod>, String> {
    Ok(mods::list(state.library.root()))
}

/// 选一个模组包：文件（压缩包 / .SC2Mod）或文件夹。
#[tauri::command(async)]
pub(crate) fn pick_mod_source(kind: String) -> Option<String> {
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

/// 导入前的预检：这个包是谁、库里有没有同族的。
///
/// 界面拿它决定要不要问一句「作为新版本还是独立改版」。
#[tauri::command(async)]
pub(crate) fn preview_mod(
    path: String,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<mods::ModPreview, String> {
    let source = Path::new(&path);
    let (declared_id, declared_version) = read_mod_meta(source, password.as_deref());

    mods::preview_with_password(
        state.library.root(),
        source,
        declared_id.as_deref(),
        declared_version.as_deref(),
        password.as_deref(),
    )
    .map_err(|error| error.to_string())
}

/// 从包里读 modid 与版本号（裸目录 / .SC2Mod 没有元数据，返回 None）。
///
/// **版本号必须一起读**：不读的话，库里只能从已有版本去推，
/// 结果「作者发的 1.2 版」会被当成「1.0 撞号」而改名叫 1.0.2。
fn read_mod_meta(source: &Path, password: Option<&str>) -> (Option<String>, Option<String>) {
    if !source.is_file() {
        return (None, None);
    }
    match package::inspect_with_password(source, password) {
        Ok(found) => (found.modid, found.version),
        Err(_) => (None, None),
    }
}

/// 导入一个独立模组包（目录 / .SC2Mod / 压缩包都行）。
///
/// `mode` 决定跟库里同 modid 的模组怎么处：
/// - `version` —— 作为它的**新版本**归到一起，可切换
/// - `separate` —— 作为**独立改版**单独显示一个
/// - 不传 —— 自动（同 id 就归并）
#[tauri::command(async)]
pub(crate) fn import_mod(
    path: String,
    mode: Option<mods::ModImportMode>,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<mods::ModImport, String> {
    let source = Path::new(&path);
    let (declared_id, declared_version) = read_mod_meta(source, password.as_deref());

    mods::import_with_password(
        state.library.root(),
        source,
        declared_id.as_deref(),
        declared_version.as_deref(),
        mode.unwrap_or_default(),
        password.as_deref(),
    )
    .map_err(|error| error.to_string())
}

/// 改模组信息（只改启动器记录的，不动文件）。
#[tauri::command(async)]
pub(crate) fn update_mod(
    id: String,
    changes: ModChanges,
    state: State<'_, AppState>,
) -> Result<StandaloneMod, String> {
    mods::update(state.library.root(), &id, changes).map_err(|error| error.to_string())
}

/// 跟着战役包来的模组的定位信息 —— 第一次编辑时靠它建记录。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModIdentity {
    slot: String,
    variant: String,
    /// 相对版本目录的路径（那个模组文件夹）。
    path: String,
    folder: String,
    name: String,
    version: Option<String>,
    /// `file` / `folder`。
    kind: String,
    parts: usize,
}

/// 改一个模组的元数据。
///
/// **独立模组和战役包带来的模组走同一条路** —— 都是一条模组记录，
/// 改的都是记录，不动包里的原始文件：
///
/// - `record_id` 有值：直接改那条记录
/// - 只有 `identity`：说明是战役包带来的、还没记录 —— 先建一条再改
#[tauri::command(async)]
pub(crate) fn edit_mod(
    record_id: Option<String>,
    identity: Option<ModIdentity>,
    changes: ModChanges,
    state: State<'_, AppState>,
) -> Result<StandaloneMod, String> {
    let data = state.library.root();

    let id = match record_id {
        Some(id) => id,
        None => {
            let identity = identity.ok_or_else(|| "不知道该改哪个模组".to_string())?;
            let kind = if identity.kind == "file" {
                mods::ModKind::File
            } else {
                mods::ModKind::Folder
            };

            mods::remember_campaign(
                data,
                &identity.slot,
                &identity.variant,
                &identity.path,
                &identity.folder,
                &identity.name,
                identity.version.as_deref(),
                kind,
                identity.parts,
            )
            .map_err(|error| error.to_string())?
            .id
        }
    };

    mods::update(data, &id, changes).map_err(|error| error.to_string())
}

/// 删掉一个独立模组。
#[tauri::command(async)]
pub(crate) fn remove_mod(id: String, state: State<'_, AppState>) -> Result<(), String> {
    mods::remove(state.library.root(), &id).map_err(|error| error.to_string())
}

/// 把独立模组打包导出成一个 zip，返回写到了哪。
#[tauri::command(async)]
pub(crate) fn export_mod(id: String, state: State<'_, AppState>) -> Result<String, String> {
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

/// 比两个模组版本差在哪。
///
/// 装了 SC2Diff 的话还会给出**语义 diff**（改了什么字段/物件），
/// 没装就只列文件级差异 —— 界面按 `semantic_note` 如实说明。
#[tauri::command(async)]
pub(crate) fn compare_mods(
    before: String,
    after: String,
    state: State<'_, AppState>,
) -> Result<mods::ModComparison, String> {
    mods::compare(state.library.root(), &before, &after).map_err(|error| error.to_string())
}

/// 把**选中的若干模组版本**打包成一个 zip。
///
/// 让用户挑是刻意的：一个模组攒了十几个版本之后全打包又大又没人要，
/// 而「把 1.0 和 1.3 一起发过去对比」是很实际的需求。
#[tauri::command(async)]
pub(crate) fn export_mods(ids: Vec<String>, state: State<'_, AppState>) -> Result<String, String> {
    let data = state.library.root();

    let first = ids
        .first()
        .and_then(|id| mods::get(data, id))
        .ok_or_else(|| "没有选中任何版本".to_string())?;

    let target = rfd::FileDialog::new()
        .set_title("导出模组版本")
        .set_file_name(format!("{}-版本包.zip", first.name))
        .add_filter("压缩包", &["zip"])
        .save_file()
        .ok_or_else(|| "已取消".to_string())?;

    let count = mods::export_many(data, &ids, &target).map_err(|error| error.to_string())?;
    Ok(format!("{} 个文件 -> {}", count, target.to_string_lossy()))
}

/// 启用 / 停用独立模组。启用会立刻把它铺进 `<游戏>/Mods/`。
#[tauri::command(async)]
pub(crate) fn set_mod_enabled(
    id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<StandaloneMod, String> {
    ensure_game_not_running()?;

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
pub(crate) fn list_library_mods(state: State<'_, AppState>) -> Result<Vec<LibraryMod>, String> {
    let mut rows = state.library.all_mods();

    // 独立模组「真的铺在 Mods/ 里」与否（它们不走战役的 owner 记账）
    let installed: std::collections::BTreeSet<String> =
        miyin_core::library::Manifest::load(state.library.root())
            .files
            .iter()
            .filter_map(|item| match &item.owner {
                miyin_core::library::Owner::Mod { id } => Some(id.clone()),
                miyin_core::library::Owner::Campaign { .. } => None,
            })
            .collect();

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
            in_game: installed.contains(&record.id),
            parts: record.parts,
            origin: miyin_core::library::ModOrigin::Standalone,
            required: false,
            standalone_id: Some(record.id.clone()),
            modid: Some(mods::effective_id(&record)),
            version: record.version.clone(),
            folder: Some(mods::placed_name(&record)),
            kind: Some(
                match record.kind {
                    mods::ModKind::File => "file",
                    mods::ModKind::Folder => "folder",
                }
                .to_string(),
            ),
            mod_record_id: Some(record.id.clone()),
            source_kind: "library".to_string(),
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
pub(crate) fn list_game_mods(state: State<'_, AppState>) -> Result<Vec<GameModEntry>, String> {
    let installation = require_installation(&state)?;
    Ok(installation.game_mods())
}
