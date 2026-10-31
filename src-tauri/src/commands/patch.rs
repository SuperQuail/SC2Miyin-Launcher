//! 补丁：导入 / 挂载 / 配置 / 导出 / 合成预览。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use std::path::Path;

use miyin_core::library::{self, Binding, Patch};
use tauri::State;

use crate::AppState;

/// 库里全部补丁。
#[tauri::command(async)]
pub(crate) fn list_patches(state: State<'_, AppState>) -> Vec<Patch> {
    state.library.index().patches.into_values().collect()
}

/// 某个战役挂着的补丁（含未启用的），带上它此刻能不能自动匹配。
#[derive(Debug, serde::Serialize)]
pub(crate) struct BoundPatch {
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
pub(crate) fn list_bindings(slot: String, state: State<'_, AppState>) -> Vec<BoundPatch> {
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
pub(crate) fn list_available_patches(slot: String, state: State<'_, AppState>) -> Vec<BoundPatch> {
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
pub(crate) fn import_patch(
    path: String,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<Patch, String> {
    library::patch::import_patch_with_password(
        &state.library,
        Path::new(&path),
        password.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// 把库里所有**声明了依赖且匹配得上**的补丁自动挂到对应战役上。
#[tauri::command(async)]
pub(crate) fn auto_bind_patches(state: State<'_, AppState>) -> Result<Vec<String>, String> {
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
pub(crate) fn bind_patch(
    slot: String,
    patch_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    library::patch::bind(&state.library, &slot, &patch_id).map_err(|e| e.to_string())
}

/// 解绑（补丁本身还在库里）。
#[tauri::command(async)]
pub(crate) fn unbind_patch(
    slot: String,
    patch_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    library::patch::unbind(&state.library, &slot, &patch_id).map_err(|e| e.to_string())
}

/// 改挂载设置：启用状态 / 优先级。**改完需要重新启用一次战役才会生效。**
#[tauri::command(async)]
pub(crate) fn configure_patch(
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
pub(crate) fn update_patch(
    patch_id: String,
    changes: library::patch::PatchChanges,
    state: State<'_, AppState>,
) -> Result<Patch, String> {
    library::patch::update_patch(&state.library, &patch_id, changes).map_err(|e| e.to_string())
}

/// 单独导出一个补丁包。
#[tauri::command(async)]
pub(crate) fn export_patch(
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
pub(crate) fn delete_patch(patch_id: String, state: State<'_, AppState>) -> Result<(), String> {
    library::patch::remove_patch(&state.library, &patch_id).map_err(|e| e.to_string())
}

/// 预览某个版本打上当前补丁后的合成清单。
#[tauri::command(async)]
pub(crate) fn preview_composition(
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
