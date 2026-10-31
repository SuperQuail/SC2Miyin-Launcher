//! 存档：存档组（备份）的看 / 备 / 换 / 还原，以及存档隔离。
//!
//! 命令名与参数名是 IPC 契约（前端 `ui/src/api/`），不要改。

use miyin_core::saves as library_saves;
use tauri::State;

use crate::{AppState, lock_error};

/// 存档隔离状态：开关、当前组、组到战役的指派。
///
/// 读之前先跑一次**迁移/自愈**：老数据里 `active` 那个组可能还绑在战役级键上，
/// 而槽位当前启用的其实是某个改版 —— 不修的话界面会出现「当前启用 X 改版，
/// 但当前在用 Y 战役的组」这种对不上的显示。
#[tauri::command(async)]
pub(crate) fn save_isolation(
    state: State<'_, AppState>,
) -> Result<library_saves::Isolation, String> {
    let root = state.library.root();
    library_saves::reconcile_active_binding(&state.library, root)
        .map_err(|error| error.to_string())?;
    library_saves::isolation(root).map_err(|error| error.to_string())
}

/// 打开 / 关掉存档隔离。**打开的那一刻会把现在这份 Banks 收成「原版」。**
#[tauri::command(async)]
pub(crate) fn set_save_isolation(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<library_saves::Isolation, String> {
    let banks = banks_root(&state)?;
    library_saves::set_isolation(state.library.root(), &banks, enabled).map_err(|e| e.to_string())
}

/// 把现在游戏里这份存档存回它所属的组。
#[tauri::command(async)]
pub(crate) fn save_current_saves(
    state: State<'_, AppState>,
) -> Result<library_saves::Isolation, String> {
    let banks = banks_root(&state)?;
    library_saves::save_active(state.library.root(), &banks).map_err(|e| e.to_string())
}

/// 切到某个存档组（会先把现在这份存回去）。
#[tauri::command(async)]
pub(crate) fn switch_save_profile(
    name: String,
    state: State<'_, AppState>,
) -> Result<library_saves::Isolation, String> {
    let banks = banks_root(&state)?;
    library_saves::switch(state.library.root(), &banks, &name).map_err(|e| e.to_string())
}

/// 手动改一个存档组的归属：指给某个战役，或 null 表示算原版。
#[tauri::command(async)]
pub(crate) fn assign_save_profile(
    name: String,
    slot: Option<String>,
    state: State<'_, AppState>,
) -> Result<library_saves::Isolation, String> {
    library_saves::assign(state.library.root(), &name, slot.as_deref()).map_err(|e| e.to_string())
}

/// 把现在游戏里这份存档存进**指定的组**（存档页的「设为当前」）。
///
/// 和 `save_current_saves` 的区别：那个只存回"当前组"，这个由用户点名。
#[tauri::command(async)]
pub(crate) fn save_into_profile(
    name: String,
    state: State<'_, AppState>,
) -> Result<library_saves::Isolation, String> {
    let banks = banks_root(&state)?;
    library_saves::save_into(state.library.root(), &banks, &name).map_err(|e| e.to_string())
}

/// 候选清单：官方战役（固定菜单顺序）+ 每部自制战役，各带一个 `enabled`。
///
/// `enabled` 按**激活清单**（游戏目录里实际装了什么）算 —— 自制战役可以同时装好几部，
/// 所以别拿槽位的 `active`（那只是最后选中的那个）当「装了没」。
/// 键的算法和 `saves::isolation_key` 同源，界面别自己拼。
#[tauri::command(async)]
pub(crate) fn saves_targets(state: State<'_, AppState>) -> Vec<miyin_core::saves::SaveTarget> {
    let installation = state
        .installation
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let installed = match installation.as_ref() {
        Some(installation) => {
            miyin_core::library::activation::enabled_variants(&state.library, installation)
        }
        None => Vec::new(),
    };
    miyin_core::saves::targets(&state.library, &installed)
}

/// 一个存档组里有哪些存档文件（组详情用：可以打开看、挑某一份挪走）。
///
/// **只读**，而且只读 `data/saves/<组>/` —— 不碰游戏目录的 Banks。
#[tauri::command(async)]
pub(crate) fn save_group_files(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<miyin_core::saves::SaveFile>, String> {
    miyin_core::saves::group_files(state.library.root(), &name).map_err(|error| error.to_string())
}

/// 把某一份存档挪到另一个组（= 改它归到哪个战役）。
///
/// 目标组不存在会建；**同名不覆盖**（报错让用户自己决定）；源组空了也不删。
/// 只动 `data/saves/`，绝不碰游戏目录的 Banks。
#[tauri::command(async)]
pub(crate) fn move_save_file(
    from: String,
    file: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    miyin_core::saves::move_save_file(state.library.root(), &from, &file, &to)
        .map_err(|error| error.to_string())
}

/// 把某一份存档挪到**某个战役 / 版本名下**（键来自 `saves_targets`）。
///
/// 组是**自动生成**的：有现成组就挪进去，没有就按启用同一套规则新建 + 自动绑定 ——
/// 用户改的是「这一份属于谁」，不是把整个组改绑到别的战役（那样组名就没意义了）。
/// 只动 `data/saves`，不碰 Banks；同名不覆盖。返回挪进去的组名。
#[tauri::command(async)]
pub(crate) fn move_save_to_key(
    from: String,
    file: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    miyin_core::saves::move_save_to_key(state.library.root(), &from, &file, &key)
        .map_err(|error| error.to_string())
}

/// 扫用户文档目录里**真实**的存档（`Accounts/<账号>/<地域>/Saves` 与 `Banks`）。
///
/// 只读：不写盘、不改任何东西。**这是新的扫描** —— 真机上根本没有
/// `Documents/StarCraft II/Banks` 这个目录，旧的那套读法永远是 0 个。
#[tauri::command(async)]
pub(crate) fn scan_saves(
    state: State<'_, AppState>,
) -> Result<miyin_core::saves::SaveInventory, String> {
    let installation = state
        .installation
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let documents = installation
        .as_ref()
        .and_then(|installation| installation.documents_root.clone())
        .ok_or_else(|| "找不到星际争霸 II 的用户文档目录".to_string())?;
    Ok(miyin_core::saves::scan(&documents))
}

/// 现在的存档里有什么 —— **游戏真正读的那几份**：`Documents\StarCraft II\Accounts/<账号>/<地域>/Banks`
/// 全部算上（见 `saves::SavesRoots`）；没有 `Accounts` 结构才退回老的 `Documents\StarCraft II\Banks`。
#[tauri::command(async)]
pub(crate) fn list_saves(state: State<'_, AppState>) -> Result<library_saves::SaveSet, String> {
    let banks = banks_root(&state)?;
    library_saves::snapshot(&banks).map_err(|error| error.to_string())
}

/// 把现在的存档备份一份，`label` 是给用户看的备注（一般填战役名）。
#[tauri::command(async)]
pub(crate) fn backup_saves(label: String, state: State<'_, AppState>) -> Result<String, String> {
    let banks = banks_root(&state)?;
    library_saves::backup(&banks, state.library.root(), &label).map_err(|error| error.to_string())
}

/// 已经备份了哪些（= 存档组列表）。
///
/// 读之前同样先跑一次迁移，免得界面把「绑在战役级键上的老组」显示成当前在用。
#[tauri::command(async)]
pub(crate) fn list_save_backups(
    state: State<'_, AppState>,
) -> Result<Vec<library_saves::BackupEntry>, String> {
    let root = state.library.root();
    library_saves::reconcile_active_binding(&state.library, root)
        .map_err(|error| error.to_string())?;
    library_saves::backups(root).map_err(|error| error.to_string())
}

/// 还原一份备份。**还原前会先把现在的存档另存一份**，返回那份安全备份的名字。
#[tauri::command(async)]
pub(crate) fn restore_saves(name: String, state: State<'_, AppState>) -> Result<String, String> {
    let banks = banks_root(&state)?;
    library_saves::restore(&banks, state.library.root(), &name).map_err(|error| error.to_string())
}

/// 老的存档目录锚点：`Documents\StarCraft II\Banks`。找不到就报错，**不猜**。
///
/// 这只是个锚 —— 真正要动的是它旁边 `Accounts/<账号>/<地域>/Banks` 下的**所有**
/// 存档根（`saves::SavesRoots::from_banks` 会就地展开）。游戏不读这个空目录本身。
fn banks_root(state: &State<'_, AppState>) -> Result<std::path::PathBuf, String> {
    let guard = state.installation.lock().map_err(lock_error)?;
    let installation = guard.as_ref().ok_or("还没找到星际争霸 II 的安装目录")?;
    installation
        .banks_root
        .clone()
        .ok_or_else(|| "找不到「我的文档 / StarCraft II」—— 游戏还没产生过存档？".to_string())
}
