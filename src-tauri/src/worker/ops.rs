//! 助手的操作白名单：JSON 参数 -> miyin-core 的写函数。
//!
//! 这里是提权进程里唯一会动磁盘的地方，所以两条规矩写死：
//! 1. op 必须在白名单里（[OPS]），其余一律「未知操作」；
//! 2. **根目录自己再验一遍** —— 数据根必须是「游戏目录同级 / miyin_launcher_data」，
//!    包来源路径必须是真实存在的绝对路径、且不在系统目录里。核心库的写盘闸门
//!    管游戏目录内部，这里管「调用方有没有骗我们换根」。
//!
//! 只代理**写游戏目录 / 库根**的操作（设计文档 §3）。存档、导出、config / network
//! 的写入留在 UI 进程，永远不会走到这个管理员进程里；读操作同理。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use miyin_core::campaign::package;
use miyin_core::library::{
    self, DATA_DIR_NAME, ImportMode, Library, ModChanges, Replaced, VariantChanges,
    find_target_for_package_with_password, merge, mods,
};
use miyin_core::sc2::{DiscoverySource, Installation};
use serde::Serialize;
use serde_json::Value;

use crate::ensure_game_not_running;

/// 白名单。**只有这里列出的 op 会被执行**，加 op 时把单测一起补上。
///
/// 这是设计文档 §3 的完整清单：全部是写游戏目录或库根的操作。
/// 存档 / 导出 / config / network 不在这里 —— 那些留在 UI 进程。
pub(crate) const OPS: &[&str] = &[
    "activate",
    "deactivate_all",
    "import_package",
    "update_variant",
    "update_variant_from_package",
    "delete_variant",
    "import_patch",
    "mods_import",
    "mods_edit",
    "mods_remove",
    "mods_set_enabled",
    "mods_sync",
    "merge_apply",
];

/// op 名在白名单里吗。
fn known(op: &str) -> bool {
    OPS.contains(&op)
}

/// 执行一条 op。参数里的 dataRoot / gameRoot 由 UI 进程注入（前端管不着）。
pub(crate) fn run(op: &str, args: &Value) -> Result<Value, String> {
    if !known(op) {
        return Err(format!("未知操作：{op}"));
    }
    let context = Context::load(args)?;

    match op {
        "activate" => {
            ensure_game_not_running()?;
            let slot = text(args, "slot")?;
            let variant = optional_text(args, "variantId");
            value(library::activate(
                &context.library,
                &context.installation,
                &slot,
                variant.as_deref(),
            ))
        }
        "deactivate_all" => {
            ensure_game_not_running()?;
            value(library::deactivate(&context.library, &context.installation))
        }
        "import_package" => {
            let path = source(args, "path")?;
            let password = optional_text(args, "password");
            let inspection = package::inspect_with_password(&path, password.as_deref())
                .map_err(|error| error.to_string())?;
            let slot = optional_text(args, "slot")
                .filter(|slot| !slot.trim().is_empty())
                .or_else(|| library::slot_for(&inspection.campaign_type).map(str::to_string))
                .ok_or_else(|| "无法从包内识别它属于哪个战役，请手动指定".to_string())?;
            value(library::import_with_password(
                &context.library,
                &path,
                &slot,
                import_mode(args, "mode")?,
                password.as_deref(),
            ))
        }
        "update_variant" => {
            let slot = text(args, "slot")?;
            let variant_id = text(args, "variantId")?;
            let changes: VariantChanges =
                serde_json::from_value(args.get("changes").cloned().unwrap_or(Value::Null))
                    .map_err(|error| format!("changes 不合法：{error}"))?;
            value(library::update_variant(
                &context.library,
                &slot,
                &variant_id,
                changes,
            ))
        }
        "update_variant_from_package" => {
            ensure_game_not_running()?;
            let slot = text(args, "slot")?;
            let path = source(args, "path")?;
            let password = optional_text(args, "password");
            // 换了哪一版：点明了就是它，没点明就按包自己的身份去库里找；找不到就当新装
            let target = match optional_text(args, "variantId") {
                Some(id) => Some(id),
                None => find_target_for_package_with_password(
                    &context.library,
                    &slot,
                    &path,
                    password.as_deref(),
                )
                .map_err(|error| error.to_string())?
                .map(|variant| variant.id),
            };
            let replaced = match target {
                Some(id) => library::replace_variant_with_password(
                    &context.library,
                    &context.installation,
                    &slot,
                    &id,
                    &path,
                    password.as_deref(),
                ),
                None => library::import_with_password(
                    &context.library,
                    &path,
                    &slot,
                    ImportMode::Rename,
                    password.as_deref(),
                )
                .map(|variant| Replaced {
                    variant,
                    added_mods: Vec::new(),
                }),
            };
            value(replaced)
        }
        "delete_variant" => {
            ensure_game_not_running()?;
            let slot = text(args, "slot")?;
            let variant_id = text(args, "variantId")?;
            library::remove_variant(&context.library, &context.installation, &slot, &variant_id)
                .map_err(|error| error.to_string())?;
            Ok(Value::Null)
        }
        "import_patch" => {
            let path = source(args, "path")?;
            let password = optional_text(args, "password");
            value(library::patch::import_patch_with_password(
                &context.library,
                &path,
                password.as_deref(),
            ))
        }
        "mods_import" => {
            let path = source(args, "path")?;
            let password = optional_text(args, "password");
            // 包内声明的 modid / 版本号：裸目录与 .SC2Mod 读不出来，退回 None
            let (declared_id, declared_version) = if path.is_file() {
                match package::inspect_with_password(&path, password.as_deref()) {
                    Ok(found) => (found.modid, found.version),
                    Err(_) => (None, None),
                }
            } else {
                (None, None)
            };
            value(mods::import_with_password(
                context.library.root(),
                &path,
                declared_id.as_deref(),
                declared_version.as_deref(),
                mod_mode(args, "mode")?,
                password.as_deref(),
            ))
        }
        "mods_edit" => {
            let changes: ModChanges =
                serde_json::from_value(args.get("changes").cloned().unwrap_or(Value::Null))
                    .map_err(|error| format!("changes 不合法：{error}"))?;
            // 有 recordId 就改那条记录；只有 identity 的是「跟着战役包来的、还没记录」，
            // 先按它的定位建一条记录再改（与命令层同一条路）。
            let id = match optional_text(args, "recordId") {
                Some(id) => id,
                None => {
                    let identity: ModIdentity = serde_json::from_value(
                        args.get("identity").cloned().unwrap_or(Value::Null),
                    )
                    .map_err(|_| "不知道该改哪个模组：recordId 与 identity 都没给".to_string())?;
                    let kind = if identity.kind == "file" {
                        mods::ModKind::File
                    } else {
                        mods::ModKind::Folder
                    };
                    mods::remember_campaign(
                        context.library.root(),
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
            value(mods::update(context.library.root(), &id, changes))
        }
        "mods_remove" => {
            let id = text(args, "id")?;
            mods::remove(context.library.root(), &id).map_err(|error| error.to_string())?;
            Ok(Value::Null)
        }
        "mods_set_enabled" => {
            ensure_game_not_running()?;
            let id = text(args, "id")?;
            let enabled = flag(args, "enabled")?;
            // 先落状态再同步 —— sync 读的就是这份记录
            let record = mods::set_enabled(context.library.root(), &id, enabled)
                .map_err(|error| error.to_string())?;
            mods::sync(context.library.root(), &context.installation)
                .map_err(|error| error.to_string())?;
            value(Ok::<_, miyin_core::Error>(record))
        }
        "mods_sync" => {
            ensure_game_not_running()?;
            // 两条挂载路径都要跑：跟着战役包来的模组归 activation，独立模组归 mods。
            // 启动流程里也是这么成对调的（见 AppState::bootstrap）。
            let warnings = library::activation::sync_mods(&context.library, &context.installation)
                .map_err(|error| error.to_string())?;
            let report = mods::sync(context.library.root(), &context.installation)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({ "warnings": warnings, "report": report }))
        }
        "merge_apply" => {
            // 合并案原本保存在 UI 进程里；助手没有那份内存，所以由 UI 把旧数据根
            // 一起传过来，服务端重算（而不是信一份过期的 plan）。
            let local = merge_local(args, context.library.root(), &context.installation)?;
            let choices = merge_choices(args)?;
            let plan =
                merge::plan(&local, context.library.root()).map_err(|error| error.to_string())?;
            value(merge::apply(context.library.root(), &plan, &choices))
        }
        other => Err(format!("未知操作：{other}")),
    }
}

/// 战役包带来的模组的定位信息（界面的 `mods_edit` 会带上它）。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModIdentity {
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

/// 一次请求的上下文：数据根 + 游戏根，都在这里验过再往下传。
#[derive(Debug)]
struct Context {
    library: Library,
    installation: Installation,
}

impl Context {
    fn load(args: &Value) -> Result<Self, String> {
        let data_root = PathBuf::from(text(args, "dataRoot")?);
        let game_root = PathBuf::from(text(args, "gameRoot")?);
        if !data_root.is_absolute() || !game_root.is_absolute() {
            return Err("数据根与游戏根必须是绝对路径".to_string());
        }

        // 顺带校验目录里真有 StarCraft II.exe —— 不存在就不是游戏目录
        // 直接透出核心库的错误文本：权限错误带 `[PERMISSION] ` 前缀，界面靠它决定
        // 要不要再提权（在这里加前缀会把那个标记挤到句子中间）。
        let installation = Installation::from_root(&game_root, DiscoverySource::Manual)
            .map_err(|error| error.to_string())?;

        // 数据根固定是「游戏目录同级 / miyin_launcher_data」，不接受调用方随便指一个目录
        // （否则等于把管理员写权限交给任意路径）
        let expected = installation
            .root
            .parent()
            .map(|parent| parent.join(DATA_DIR_NAME));
        if !expected
            .as_deref()
            .is_some_and(|path| same_path(path, &data_root))
        {
            return Err(format!(
                "数据根必须是游戏目录同级的 {DATA_DIR_NAME}：{}",
                data_root.display()
            ));
        }

        Ok(Self {
            library: Library::new(&data_root),
            installation,
        })
    }
}

/// 参数里的一个字符串。
fn text(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("缺少参数 {key}"))
}

fn optional_text(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

/// 参数里的一个布尔。
fn flag(args: &Value, key: &str) -> Result<bool, String> {
    args.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("缺少参数 {key}（布尔）"))
}

/// 导入模式；不传就按核心库的默认值（重命名后导入）。
fn import_mode(args: &Value, key: &str) -> Result<ImportMode, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(ImportMode::default()),
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|_| format!("{key} 只能是 overwrite 或 rename")),
    }
}

/// 模组导入模式；不传就是自动。
fn mod_mode(args: &Value, key: &str) -> Result<mods::ModImportMode, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(mods::ModImportMode::default()),
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|_| format!("{key} 只能是 auto / version / separate")),
    }
}

/// 把核心库的 Result 换成 JSON。
fn value<T: Serialize>(result: miyin_core::Result<T>) -> Result<Value, String> {
    let value = result.map_err(|error| error.to_string())?;
    serde_json::to_value(value).map_err(|error| format!("结果序列化失败：{error}"))
}

/// 包来源（只读）：必须是真实存在的绝对路径，且不在系统目录里。
///
/// 导入的来源本来就在用户自己的目录（下载 / 桌面 / 移动硬盘），不该被提权助手
/// 当成「读管理员才能读的系统文件」的通道 —— 所以 %SystemRoot% 与 Program Files
/// 一律不认。真正的写目标由核心库的闸门管，这里只管来源。
fn source(args: &Value, key: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(text(args, key)?);
    if !raw.is_absolute() {
        return Err(format!("{key} 必须是绝对路径：{}", raw.display()));
    }
    // 先解析再判目录：用符号链接把系统目录绕回白名单的路子走不通
    let resolved = miyin_core::safety::resolve(&raw)
        .map_err(|error| format!("{key} 路径不合法（{}）：{error}", raw.display()))?;
    if !resolved.exists() {
        return Err(format!("找不到 {key}：{}", resolved.display()));
    }
    if let Some(root) = system_locations()
        .into_iter()
        .find(|root| within(&resolved, root))
    {
        return Err(format!(
            "{key} 不能在系统目录里（{}）：{}",
            root.display(),
            resolved.display()
        ));
    }
    Ok(resolved)
}

/// 不该当导入来源的系统目录：Windows 目录 + Program Files（x86）。
fn system_locations() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    #[cfg(windows)]
    if let Some(windows) = super::win::windows_dir() {
        roots.push(windows);
    }
    roots
}

/// 合并的旧数据根：绝对路径、目录名必须是 data / miyin_launcher_data。
///
/// 核心库的 `merge::apply` 会从它搬版本目录、并在搬空后删掉它；管理员进程里
/// 不能接受一个调用方随手指的空目录（那等于「删任意目录」通道）。
fn merge_local(
    args: &Value,
    target: &Path,
    installation: &Installation,
) -> Result<PathBuf, String> {
    let local = source(args, "local")?;
    if !local.is_dir() {
        return Err(format!("local 必须是目录：{}", local.display()));
    }
    let name = local
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name != "data" && name != DATA_DIR_NAME {
        return Err(format!(
            "local 只能是旧数据根 data / {DATA_DIR_NAME}：{}",
            local.display()
        ));
    }
    if same_path(&local, target) || within(&local, target) || within(&local, &installation.root) {
        return Err(format!(
            "local 不能是当前数据根或游戏目录：{}",
            local.display()
        ));
    }
    Ok(local)
}

/// 合并选择表：key 是 `<slot>/<variant>`，值是 `target` / `local` / `both`。
///
/// UI 现在习惯直接把选择表当 args 传（命令 merge_apply 只收 choices），
/// 也接受包一层 `choices` 的写法；两种都认，未知值直接报错而不是默默当成 target。
fn merge_choices(args: &Value) -> Result<BTreeMap<String, merge::MergeChoice>, String> {
    use merge::MergeChoice;

    let object = match args.get("choices") {
        Some(inner) => inner
            .as_object()
            .ok_or_else(|| "choices 必须是对象".to_string())?,
        None => args
            .as_object()
            .ok_or_else(|| "merge_apply 需要 choices 对象".to_string())?,
    };
    let mut choices = BTreeMap::new();
    for (key, value) in object {
        if matches!(key.as_str(), "local" | "dataRoot" | "gameRoot" | "choices") {
            continue;
        }
        let choice = match value.as_str().unwrap_or_default() {
            "target" => MergeChoice::Target,
            "local" => MergeChoice::Local,
            "both" => MergeChoice::Both,
            other => {
                return Err(format!(
                    "合并选择不合法（{key} = {other}）：只能是 target / local / both"
                ));
            }
        };
        choices.insert(key.clone(), choice);
    }
    Ok(choices)
}

/// target 落在 root 里面吗。Windows 路径大小写不敏感，按组件比会把 C:\WINDOWS
/// 和 C:\Windows 当成两个目录 —— 这里统一转小写按前缀比。
fn within(target: &Path, root: &Path) -> bool {
    let root = root.to_string_lossy().to_lowercase();
    let root = root.trim_end_matches('\\');
    let target = target.to_string_lossy().to_lowercase();
    target == root || target.starts_with(&format!("{root}\\"))
}

/// 两个路径是不是同一个（先解析符号链接，再按 Windows 的大小写不敏感比）。
fn same_path(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        miyin_core::safety::resolve(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    normalize(left) == normalize(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用完就删的临时目录（src-tauri 没有 tempfile 依赖）。
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("miyin-worker-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("建临时目录");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 造一个「游戏目录 + 同级数据根」的布局。
    fn layout(scratch: &Scratch) -> (PathBuf, PathBuf) {
        let game = scratch.0.join("StarCraft II");
        std::fs::create_dir_all(&game).expect("建游戏目录");
        std::fs::write(game.join("StarCraft II.exe"), b"").expect("放启动器文件");
        let data = scratch.0.join(DATA_DIR_NAME);
        std::fs::create_dir_all(&data).expect("建数据根");
        (game, data)
    }

    #[test]
    fn unknown_op_is_rejected_before_touching_anything() {
        let error = run("rm -rf", &serde_json::json!({})).expect_err("未知 op 必须被拒");
        assert!(error.contains("未知操作"), "{error}");
    }

    #[test]
    fn context_accepts_the_designed_layout() {
        let scratch = Scratch::new("layout");
        let (game, data) = layout(&scratch);
        let context = Context::load(&serde_json::json!({
            "dataRoot": data.to_string_lossy(),
            "gameRoot": game.to_string_lossy(),
        }))
        .expect("这套布局应当被接受");
        assert_eq!(context.library.root(), data);
    }

    #[test]
    fn context_rejects_a_data_root_somewhere_else() {
        let scratch = Scratch::new("other-root");
        let (game, _) = layout(&scratch);
        // 数据根指到别处：等于让管理员往任意目录写，必须拒
        let elsewhere = scratch.0.join("别处");
        std::fs::create_dir_all(&elsewhere).expect("建目录");
        let error = Context::load(&serde_json::json!({
            "dataRoot": elsewhere.to_string_lossy(),
            "gameRoot": game.to_string_lossy(),
        }))
        .expect_err("数据根对不上必须被拒");
        assert!(error.contains(DATA_DIR_NAME), "{error}");
    }

    #[test]
    fn context_rejects_relative_roots() {
        let error = Context::load(&serde_json::json!({
            "dataRoot": "miyin_launcher_data",
            "gameRoot": ".",
        }))
        .expect_err("相对路径必须被拒");
        assert!(error.contains("绝对路径"), "{error}");
    }

    #[test]
    fn package_source_must_exist_and_be_absolute() {
        let scratch = Scratch::new("source");
        let missing = scratch.0.join("没有这个包.zip");
        let error = source(
            &serde_json::json!({ "path": missing.to_string_lossy() }),
            "path",
        )
        .expect_err("不存在的包必须被拒");
        assert!(error.contains("找不到"), "{error}");

        let error =
            source(&serde_json::json!({ "path": "包.zip" }), "path").expect_err("相对路径必须被拒");
        assert!(error.contains("绝对路径"), "{error}");

        let real = scratch.0.join("包.zip");
        std::fs::write(&real, b"x").expect("写文件");
        source(
            &serde_json::json!({ "path": real.to_string_lossy() }),
            "path",
        )
        .expect("存在的包应当通过");
    }

    #[test]
    fn package_source_in_a_system_directory_is_rejected() {
        // 系统目录里一定存在的文件：用它验证「不许把提权助手当读系统文件的通道」
        let Some(probe) = system_locations()
            .into_iter()
            .map(|root| root.join("System32").join("kernel32.dll"))
            .find(|probe| probe.is_file())
        else {
            return;
        };
        let error = source(
            &serde_json::json!({ "path": probe.to_string_lossy() }),
            "path",
        )
        .expect_err("系统目录里的文件必须被拒");
        assert!(error.contains("系统目录"), "{error}");
    }

    #[test]
    fn whitelist_is_exactly_the_game_and_library_writes() {
        let expected = [
            "activate",
            "deactivate_all",
            "import_package",
            "update_variant",
            "update_variant_from_package",
            "delete_variant",
            "import_patch",
            "mods_import",
            "mods_edit",
            "mods_remove",
            "mods_set_enabled",
            "mods_sync",
            "merge_apply",
        ];
        assert_eq!(OPS, expected, "白名单必须与设计文档 §3 一字不差");

        // 这些留在 UI 进程：存档 / 导出 / config / network，以及只读的启动与合并查询
        for gone in [
            "saves_backup",
            "saves_restore",
            "saves_set_isolation",
            "saves_switch",
            "export_variant",
            "export_patch",
            "set_network_settings",
            "merge_status",
            "launch_game",
            "import_mod",
            "remove_mod",
        ] {
            assert!(!known(gone), "{gone} 不该由提权助手执行");
        }
    }

    #[test]
    fn merge_apply_rejects_a_local_path_outside_the_data_root() {
        let scratch = Scratch::new("merge");
        let (game, data) = layout(&scratch);
        let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");

        let missing = run(
            "merge_apply",
            &serde_json::json!({
                "dataRoot": data.to_string_lossy(),
                "gameRoot": game.to_string_lossy(),
            }),
        )
        .expect_err("没有 local 必须被拒");
        assert!(missing.contains("local"), "{missing}");

        let wrong = scratch.0.join("别处");
        std::fs::create_dir_all(&wrong).expect("建目录");
        let error = merge_local(
            &serde_json::json!({ "local": wrong.to_string_lossy() }),
            &data,
            &installation,
        )
        .expect_err("目录名不是数据根必须被拒");
        assert!(error.contains("旧数据根"), "{error}");

        let inside = data.join(DATA_DIR_NAME);
        std::fs::create_dir_all(&inside).expect("建目录");
        let error = merge_local(
            &serde_json::json!({ "local": inside.to_string_lossy() }),
            &data,
            &installation,
        )
        .expect_err("当前数据根里面不能当合并来源");
        assert!(error.contains("不能是当前数据根"), "{error}");
    }

    #[test]
    fn merge_apply_applies_a_valid_choice_table() {
        let scratch = Scratch::new("merge-ok");
        let (game, data) = layout(&scratch);
        let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");
        let local = scratch.0.join("data");
        std::fs::create_dir_all(&local).expect("旧数据根");

        assert_eq!(
            merge_local(
                &serde_json::json!({ "local": local.to_string_lossy() }),
                &data,
                &installation,
            )
            .expect("合法的旧数据根"),
            local
        );

        let choices = merge_choices(&serde_json::json!({
            "local": local.to_string_lossy(),
            "wol/v1": "local",
            "hots/v2": "both",
        }))
        .expect("选择表");
        assert_eq!(choices.len(), 2);
        assert_eq!(choices.get("wol/v1"), Some(&merge::MergeChoice::Local));
        assert_eq!(choices.get("hots/v2"), Some(&merge::MergeChoice::Both));
        assert!(
            merge_choices(&serde_json::json!({ "wol/v1": "删掉" })).is_err(),
            "未知选择必须报错"
        );
    }
}
