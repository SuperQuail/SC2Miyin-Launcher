//! 开发者页：扫描 / 导出 / 提交历史 / 文件预览。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use tauri::{Emitter, State};

use crate::{AppState, lock_error};

/// 开发者页：扫游戏目录 + 用户自己加的目录。
///
/// 只读。越界（不在游戏目录里）不作拒绝，标成 `external` 交给界面提示。
#[tauri::command(async)]
pub(crate) fn dev_scan(
    extra: Vec<String>,
    state: State<'_, AppState>,
) -> Result<miyin_core::dev::DevScan, String> {
    let guard = state.installation.lock().map_err(lock_error)?;
    let installation = guard.as_ref().ok_or("还没找到星际争霸 II 的安装目录")?;
    miyin_core::dev::scan(installation, &extra).map_err(|error| error.to_string())
}

/// 开发者页：选一个自定义目录（可以是游戏目录外面的）。
#[tauri::command(async)]
pub(crate) fn dev_pick_directory() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("选一个目录加进这个包")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 游戏目录里已经装着的自制战役 —— 供「收编进库」用。只读。
#[tauri::command(async)]
pub(crate) fn list_installed_campaigns(
    state: State<'_, AppState>,
) -> Result<Vec<miyin_core::campaign::collect::InstalledCampaign>, String> {
    let guard = state.installation.lock().map_err(lock_error)?;
    let installation = guard.as_ref().ok_or("还没找到星际争霸 II 的安装目录")?;
    miyin_core::campaign::collect::installed_campaigns(installation)
        .map_err(|error| error.to_string())
}

/// 开发者页：选一个导出路径。
#[tauri::command(async)]
pub(crate) fn dev_pick_export_path(default_name: String) -> Option<String> {
    rfd::FileDialog::new()
        .set_title("导出到哪里")
        .set_file_name(&default_name)
        .add_filter("压缩包", &["zip"])
        .save_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 开发者页：选一个要带进包的文件（封面图 / 说明书 PDF）。
#[tauri::command(async)]
pub(crate) fn dev_pick_doc(kind: String) -> Option<String> {
    let (title, extensions): (&str, &[&str]) = match kind.as_str() {
        "cover" => ("选一张封面图", &["png", "jpg", "jpeg", "webp", "gif"]),
        _ => ("选一份说明书（PDF）", &["pdf"]),
    };
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("文件", extensions)
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

/// 开发者页：把勾选的文件打成一个包（附元数据）。
#[tauri::command(async)]
pub(crate) fn dev_export(
    dest: String,
    meta: miyin_core::dev::export::PackageMeta,
    files: Vec<miyin_core::dev::export::ExportFile>,
    window: tauri::Window,
) -> Result<miyin_core::dev::export::ExportReport, String> {
    // 打包大包要几十秒，界面得看得见进度 —— 每写完一个文件报一次
    miyin_core::dev::export::export(std::path::Path::new(&dest), &meta, &files, |done, total| {
        let _ = window.emit("dev://export", ExportProgress { done, total });
    })
    .map_err(|error| error.to_string())
}

/// 导出进度（发给界面画进度条）。
#[derive(Clone, serde::Serialize)]
pub(crate) struct ExportProgress {
    done: usize,
    total: usize,
}

/// 开发者页：这个包的提交历史。
#[tauri::command(async)]
pub(crate) fn dev_history(
    pkg: String,
    state: State<'_, AppState>,
) -> Result<Vec<miyin_core::dev::history::Commit>, String> {
    miyin_core::dev::history::log(state.library.root(), &pkg).map_err(|error| error.to_string())
}

/// 开发者页：删掉一条提交记录（只删记录，不动文件）。
#[tauri::command(async)]
pub(crate) fn dev_forget_commit(
    pkg: String,
    id: u32,
    state: State<'_, AppState>,
) -> Result<Vec<miyin_core::dev::history::Commit>, String> {
    miyin_core::dev::history::forget(state.library.root(), &pkg, id)
        .map_err(|error| error.to_string())
}

/// 开发者页：一次提交里要记的文件。
#[derive(serde::Deserialize)]
pub(crate) struct CommitFile {
    /// 给用户看的路径
    path: String,
    /// 绝对路径
    abs: String,
}

/// 开发者页：提交一次 —— 把当前这批文件记成一个版本。
#[tauri::command(async)]
pub(crate) fn dev_commit(
    pkg: String,
    message: String,
    label: Option<String>,
    files: Vec<CommitFile>,
    meta: Option<miyin_core::dev::export::PackageMeta>,
    state: State<'_, AppState>,
) -> Result<miyin_core::dev::history::Commit, String> {
    let pairs: Vec<(String, std::path::PathBuf)> = files
        .into_iter()
        .map(|file| (file.path, std::path::PathBuf::from(file.abs)))
        .collect();
    miyin_core::dev::history::commit(
        state.library.root(),
        &pkg,
        &message,
        label.as_deref(),
        &pairs,
        meta,
    )
    .map_err(|error| error.to_string())
}

/// 开发者页：比两版差在哪。
#[tauri::command(async)]
pub(crate) fn dev_diff(
    pkg: String,
    from: u32,
    to: u32,
    state: State<'_, AppState>,
) -> Result<miyin_core::dev::history::Diff, String> {
    let history =
        miyin_core::dev::history::log(state.library.root(), &pkg).map_err(|e| e.to_string())?;
    let find = |id: u32| history.iter().find(|item| item.id == id).cloned();
    match (find(from), find(to)) {
        (Some(old), Some(new)) => Ok(miyin_core::dev::history::diff(&old, &new)),
        _ => Err("找不到这两个版本".to_string()),
    }
}

/// 开发者页：只读预览一个文件（文本 / 图片 / 二进制）。
#[tauri::command(async)]
pub(crate) fn dev_read_file(path: String) -> Result<miyin_core::dev::FilePreview, String> {
    miyin_core::dev::read_preview(std::path::Path::new(&path)).map_err(|error| error.to_string())
}

/// 开发者页：取消正在进行的导出（导出会尽早停下）。
#[tauri::command(async)]
pub(crate) fn dev_cancel_export(state: State<'_, AppState>) {
    state
        .dev_cancel
        .store(true, std::sync::atomic::Ordering::SeqCst);
}
