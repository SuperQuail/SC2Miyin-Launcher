//! 导入流水线：解开来源 -> 暂存算指纹 -> 定位模组 -> 逐条入库。

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::safety;

use super::index::{effective_id, fingerprint_of, list, mods_root, save, scan_dir};
use super::{
    ModImport, ModImportAction, ModImportMode, ModKind, ModPreview, ModSource, StandaloneMod,
};

/// 去掉压缩包后缀，拿到主体的名字。
fn strip_archive_suffix(raw: &str) -> String {
    const EXTS: &[&str] = &[".zip", ".rar", ".7z", ".tar", ".gz", ".tar.gz"];
    let lower = raw.to_ascii_lowercase();
    for ext in EXTS {
        if lower.ends_with(ext) {
            return raw[..raw.len() - ext.len()].to_string();
        }
    }
    raw.to_string()
}

/// 目录名安全化 + 撞名加序号。
fn unique_dir_name(root: &Path, wanted: &str) -> Result<String> {
    let base =
        crate::campaign::sanitize::sanitize_dir_name(wanted).unwrap_or_else(|| "mod".to_string());

    let mut candidate = base.clone();
    let mut index = 2;
    while root.join(&candidate).exists() {
        candidate = format!("{base}-{index}");
        index += 1;
        if index > 999 {
            return Err(Error::PackageRejected("同名模组太多了".to_string()));
        }
    }
    Ok(candidate)
}

/// 一包里找到的一个模组。
pub struct StagedMod {
    /// 内容本身（在暂存区里，已经搬好）。
    pub content: PathBuf,
    /// 显示名。
    pub name: String,
    /// **铺进游戏目录时的名字，原样保留**（见 `StandaloneMod::folder`）。
    pub folder: String,
    /// 铺成文件还是目录。
    pub kind: ModKind,
}

/// 暂存好的待导入内容。
///
/// **一个包里可能有多个模组**：真实样本 `疯批帝国军械库.zip` 的 `Mods/` 下就有
/// 三个（一个单文件 + 两个目录）。所以这里是一个列表，不是单个。
pub struct Staged {
    /// 整个暂存目录 —— 用完删的是**它**。
    ///
    /// 不要拿 `mods[0].content.parent()` 代替：内容没嵌套时那就是
    /// `data/mods/` 本身，一删就把整个模组库删了（写的时候差点踩到）。
    pub root: PathBuf,
    /// 这一包里找到的模组。
    pub mods: Vec<StagedMod>,
}

/// 只有一层子目录时返回它。
fn single_child_dir(root: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .filter_map(std::result::Result::ok)
        .collect();
    if entries.len() == 1 && entries[0].path().is_dir() {
        Some(entries[0].path())
    } else {
        None
    }
}

/// 目录里唯一的一个文件。
pub(super) fn single_child_file(root: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .filter_map(std::result::Result::ok)
        .collect();
    if entries.len() == 1 && entries[0].path().is_file() {
        Some(entries[0].path())
    } else {
        None
    }
}

/// 从解压出来的内容里找出这一包提供了**哪些**模组。
///
/// 真实样本教我们的：
///
/// `@text
/// 疯批帝国军械库2.3/
/// ├── Maps/…                         <- 战役地图，不要
/// └── Mods/
///     ├── 3疯批帝国之翼.SC2Mod        <- 模组一（单文件）
///     ├── Alenger/                    <- 模组二（目录，里面 18 个）
///     └── kit_liberty_story.SC2Mod/   <- 模组三（解开形态的目录）
/// `@
///
/// 所以判定是：收掉一层「包名」目录之后，`Mods/` 里**每个条目都是一个模组**。
/// 只有一个的包也走同一条路，不用特判。
fn locate_mods(root: &Path, fallback: &str) -> Vec<(PathBuf, String, ModKind)> {
    let base = single_child_dir(root).unwrap_or_else(|| root.to_path_buf());

    let mods_dir = base.join("Mods");
    if mods_dir.is_dir() {
        let mut found: Vec<(PathBuf, String, ModKind)> = std::fs::read_dir(&mods_dir)
            .into_iter()
            .flatten()
            .filter_map(std::result::Result::ok)
            .map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let kind = if path.is_file() {
                    ModKind::File
                } else {
                    ModKind::Folder
                };
                (path, name, kind)
            })
            .collect();

        if !found.is_empty() {
            // 稳定顺序，导入结果可复现
            found.sort_by(|left, right| left.1.cmp(&right.1));
            return found;
        }
    }

    // 没有 Mods/：整包当成一个目录模组
    vec![(base, fallback.to_string(), ModKind::Folder)]
}

/// 把来源解到暂存目录。
///
/// 预检和导入都要用：先算指纹、跟库里比对，比完才知道要不要留下。
///
/// **名字和形态在这一步就定死**，而且原样保留 —— 见 `StandaloneMod::folder`。
fn stage(data: &Path, source: &Path, password: Option<&str>) -> Result<Staged> {
    if !source.exists() {
        return Err(Error::PackageRejected(format!(
            "找不到要导入的模组：{}",
            source.display()
        )));
    }

    // 用来给「整包当一个模组」兜底时起名字
    let raw_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "未命名模组".to_string());
    let stripped = strip_archive_suffix(&raw_name);

    let root = mods_root(data);
    std::fs::create_dir_all(&root)?;

    let staging = safety::ensure_within(
        &root,
        &root.join(format!(".staging-{}", crate::library::now_seconds())),
    )?;
    let raw = staging.join("raw");
    std::fs::create_dir_all(&raw)?;

    // ---- 1) 先把来源原样弄进暂存区 ----
    let copied = (|| -> Result<()> {
        if source.is_dir() {
            copy_tree(source, &raw)?;
        } else if is_archive(source) {
            crate::campaign::package::extract_to_with_password(source, "", &raw, password)?;
        } else {
            std::fs::copy(source, raw.join(&raw_name))?;
        }
        Ok(())
    })();

    if let Err(error) = copied {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    // ---- 2) 找出这一包给了哪些模组 ----
    //
    // 目录和单文件就是「一个模组」，名字原样；压缩包往里找 Mods/。
    let found: Vec<(PathBuf, String, ModKind)> = if source.is_dir() {
        vec![(raw.clone(), raw_name.clone(), ModKind::Folder)]
    } else if is_archive(source) {
        locate_mods(&raw, &stripped)
    } else {
        vec![(raw.clone(), raw_name.clone(), ModKind::File)]
    };

    // ---- 3) 每个模组搬进自己的格子，保证清理统一 ----
    let mut mods: Vec<StagedMod> = Vec::new();
    for (index, (content, folder, kind)) in found.into_iter().enumerate() {
        let slot = staging.join(format!("m{index}"));

        // **每个模组都落在自己的目录里**，哪怕是单文件形态 ——
        // 统一形状之后，下面搬到库里、算指纹、铺到游戏目录都只有一条路。
        // （踩过：单文件直接 rename 过去，库里那条记录指向的是个文件，
        //   sync 时 read_dir 直接失败。）
        if content.is_file() {
            std::fs::create_dir_all(&slot)?;
            crate::platform::move_path(&content, &slot.join(&folder))?;
        } else {
            crate::platform::move_path(&content, &slot)?;
        }

        let display = folder
            .strip_suffix(".SC2Mod")
            .or_else(|| folder.strip_suffix(".sc2mod"))
            .unwrap_or(&folder)
            .to_string();

        mods.push(StagedMod {
            content: slot,
            name: display,
            folder,
            kind,
        });
    }

    // raw 里剩下的（Maps/ 之类）不属于任何模组，连同它一起丢掉
    let _ = std::fs::remove_dir_all(&raw);

    if mods.is_empty() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::PackageRejected(
            "这个包里没有找到模组（既没有 .SC2Mod 也没有 Modules 目录）".to_string(),
        ));
    }

    Ok(Staged {
        root: staging,
        mods,
    })
}

/// 导入前的预检：认一下这个包是谁、库里有没有同族的。
///
/// 会**先把内容解到暂存目录算指纹**，算完就删掉 —— 界面拿这份信息去问用户，
/// 用户选了再真正导一次。模组通常不大，多解一遍换来「不武断地替用户决定」，值。
pub fn preview(
    data: &Path,
    source: &Path,
    declared_id: Option<&str>,
    declared_version: Option<&str>,
) -> Result<ModPreview> {
    preview_with_password(data, source, declared_id, declared_version, None)
}

/// 同上，带密码（加密的模组压缩包）。
pub fn preview_with_password(
    data: &Path,
    source: &Path,
    declared_id: Option<&str>,
    declared_version: Option<&str>,
    password: Option<&str>,
) -> Result<ModPreview> {
    let staged = stage(data, source, password)?;
    let total = staged.mods.len();
    let single = total == 1;

    // 预检只看**第一个**模组：一包里好几个的时候，界面本来也该逐个问，
    // 但绝大多数包就是一个模组，先按常见情况来。
    let first = staged
        .mods
        .first()
        .ok_or_else(|| Error::PackageRejected("这个包里没有找到模组".to_string()))?;

    let fingerprint = fingerprint_of(&first.content)?;
    let display = first.name.clone();
    let folder = first.folder.clone();

    // 删的是整个暂存目录，不是内容目录 —— 见 Staged::root 的注释
    let _ = std::fs::remove_dir_all(&staged.root);

    // 一包多个模组时，包内声明的 modid 不知道该归给谁 —— 各用各的目录名
    let modid = if single {
        declared_id
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| display.clone())
    } else {
        display.clone()
    };

    let existing: Vec<StandaloneMod> = list(data)
        .into_iter()
        .filter(|item| effective_id(item) == modid)
        .collect();

    let duplicate = existing
        .iter()
        .any(|item| !item.fingerprint.is_empty() && item.fingerprint == fingerprint);

    let taken: Vec<String> = existing
        .iter()
        .filter_map(|item| item.version.clone())
        .collect();
    let mut ignored = ModImportAction::Added;
    let wanted = declared_version
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| taken.last().map(String::as_str).unwrap_or("1.0"));
    let suggested = if taken.is_empty() {
        wanted.to_string()
    } else {
        unique_version(wanted, &taken, &mut ignored)
    };

    Ok(ModPreview {
        name: display,
        folder,
        modid,
        fingerprint,
        existing,
        duplicate,
        suggested_version: suggested,
        mod_count: total,
    })
}

/// 从磁盘导入一个模组：可以是目录、`.SC2Mod` 文件，或者压缩包。
///
/// 按 **modid** 判定归属：
///
/// | 情况 | 处理 |
/// | --- | --- |
/// | 库里没有这个 id | 新模组 |
/// | 有，且**内容一模一样** | 不留第二份，直接用库里那份 |
/// | 有，版本不同 | 作为**新版本**存一条 |
/// | 有，版本相同但内容不同 | **自动改个版本号**再存 |
///
/// `declared_id` 是包内声明的 modid；没声明就退回用名字当 id。
pub fn import(
    data: &Path,
    source: &Path,
    declared_id: Option<&str>,
    declared_version: Option<&str>,
    mode: ModImportMode,
) -> Result<ModImport> {
    import_with_password(data, source, declared_id, declared_version, mode, None)
}

/// 同上，带密码（加密的模组压缩包）。
pub fn import_with_password(
    data: &Path,
    source: &Path,
    declared_id: Option<&str>,
    declared_version: Option<&str>,
    mode: ModImportMode,
    password: Option<&str>,
) -> Result<ModImport> {
    let staged = stage(data, source, password)?;
    let total = staged.mods.len();
    let single = total == 1;
    let root = mods_root(data);

    let mut records: Vec<StandaloneMod> = Vec::new();
    let mut actions: Vec<ModImportAction> = Vec::new();
    let mut messages: Vec<String> = Vec::new();
    let mut first_existing: Option<StandaloneMod> = None;

    for item in &staged.mods {
        // 一包多个模组时，包内声明的 modid 不知道该归给谁 —— 各用各的目录名
        let id_hint = if single { declared_id } else { None };

        match import_one(data, &root, item, id_hint, declared_version, mode) {
            Ok((record, action, existing, message)) => {
                if first_existing.is_none() {
                    first_existing = existing;
                }
                actions.push(action);
                messages.push(message);
                records.push(record);
            }
            Err(error) => messages.push(format!("「{}」没能导入：{error}", item.name)),
        }
    }

    // 暂存区用完就删 —— 内容都已经搬进库里了
    let _ = std::fs::remove_dir_all(&staged.root);

    let Some(record) = records.first().cloned() else {
        return Err(Error::PackageRejected(messages.join("；")));
    };

    // 全是重复的话整体也算重复，界面据此换个措辞
    let action = if actions
        .iter()
        .all(|item| *item == ModImportAction::Duplicate)
    {
        ModImportAction::Duplicate
    } else {
        actions.first().copied().unwrap_or(ModImportAction::Added)
    };

    let message = if messages.len() == 1 {
        messages.remove(0)
    } else {
        format!("这一包里 {} 个模组：{}", records.len(), messages.join("；"))
    };

    Ok(ModImport {
        record,
        records,
        action,
        existing: first_existing,
        message,
    })
}

/// 导入**一个**模组 —— 一个包里可能有多个，循环由 `import` 负责。
fn import_one(
    data: &Path,
    root: &Path,
    item: &StagedMod,
    declared_id: Option<&str>,
    declared_version: Option<&str>,
    mode: ModImportMode,
) -> Result<(
    StandaloneMod,
    ModImportAction,
    Option<StandaloneMod>,
    String,
)> {
    let fingerprint = fingerprint_of(&item.content)?;
    let display = item.name.clone();

    let base_id = declared_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| display.clone());

    // 「作为独立改版」：即使 id 相同也**不并进版本列表**，单独显示一个。
    // 做法是给 id 加个后缀 —— 库里认的就是 id，这样它们天然是两个模组。
    let modid = match mode {
        ModImportMode::Separate => {
            let existing_ids: Vec<String> = list(data).iter().map(effective_id).collect();
            let mut candidate = format!("{base_id}#alt");
            let mut index = 2;
            while existing_ids.contains(&candidate) {
                candidate = format!("{base_id}#alt-{index}");
                index += 1;
                if index > 999 {
                    break;
                }
            }
            candidate
        }
        _ => base_id,
    };

    let same_id: Vec<StandaloneMod> = list(data)
        .into_iter()
        .filter(|other| effective_id(other) == modid)
        .collect();

    // ---- 内容完全一样：不留第二份 ----
    if let Some(found) = same_id
        .iter()
        .find(|other| !other.fingerprint.is_empty() && other.fingerprint == fingerprint)
    {
        return Ok((
            found.clone(),
            ModImportAction::Duplicate,
            Some(found.clone()),
            format!("「{}」库里已经有一份完全一样的，没有重复存", found.name),
        ));
    }

    // ---- 决定版本号 ----
    //
    // **包内声明的版本优先**。没声明才从库里已有的推 —— 推出来的多半就是
    // 「跟上一版同号」，于是自然走到「撞号改名」那条路。这正是想要的：
    // 不知道版本就老实说不知道，别硬安一个。
    let declared = declared_version
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let mut action = ModImportAction::Added;
    let version = if same_id.is_empty() {
        Some(declared.unwrap_or_else(|| "1.0".to_string()))
    } else {
        action = ModImportAction::NewVersion;
        let taken: Vec<String> = same_id
            .iter()
            .filter_map(|other| other.version.clone())
            .collect();
        let wanted =
            declared.unwrap_or_else(|| taken.last().cloned().unwrap_or_else(|| "1.0".to_string()));
        Some(unique_version(&wanted, &taken, &mut action))
    };

    let id = unique_dir_name(root, &display)?;
    let target = safety::ensure_within(root, &root.join(&id))?;
    crate::platform::move_path(&item.content, &target)?;

    let (parts, size_bytes) = scan_dir(&target);
    let record = StandaloneMod {
        id: id.clone(),
        name: display,
        author: None,
        version,
        description: None,
        modid: Some(modid.clone()),
        fingerprint,
        folder: item.folder.clone(),
        kind: item.kind,
        // 独立导入的，内容就在库里
        source: ModSource::Library,
        // 默认**不启用** —— 免得悄悄改了游戏状态
        enabled: false,
        imported_at: crate::library::now_seconds(),
        size_bytes,
        parts,
    };

    let mut mods = list(data);
    mods.push(record.clone());
    save(data, &mods)?;

    let message = match action {
        ModImportAction::Renamed => format!(
            "「{}」这个版本号库里已经有了（内容不一样），自动存成了 {}",
            record.name,
            record.version.clone().unwrap_or_default()
        ),
        ModImportAction::NewVersion => format!(
            "「{}」作为新版本 {} 存下来了",
            record.name,
            record.version.clone().unwrap_or_default()
        ),
        _ => format!("已导入「{}」", record.name),
    };

    Ok((record, action, None, message))
}

/// 挑一个没被占用的版本号；撞了就加后缀，并把 action 标成「自动改名」。
fn unique_version(wanted: &str, taken: &[String], action: &mut ModImportAction) -> String {
    if !taken.iter().any(|item| item == wanted) {
        return wanted.to_string();
    }

    *action = ModImportAction::Renamed;
    for index in 2..1000 {
        let candidate = format!("{wanted}.{index}");
        if !taken.iter().any(|item| item == &candidate) {
            return candidate;
        }
    }
    format!("{wanted}.{}", crate::library::now_seconds())
}

/// 是不是压缩包（按扩展名粗判）。
fn is_archive(path: &Path) -> bool {
    const EXTS: &[&str] = &["zip", "rar", "7z", "tar", "gz", "bz2", "xz"];
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|ext| EXTS.contains(&ext.as_str()))
}

/// 递归拷一棵树。
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in walkdir::WalkDir::new(from)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry
            .path()
            .strip_prefix(from)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        let dest = to.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
