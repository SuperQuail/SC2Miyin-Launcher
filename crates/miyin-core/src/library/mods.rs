//! 独立模组库：**不属于任何战役**的模组。
//!
//! 为什么单独一块：模组不总是跟着战役包来的。玩家之间经常单独传一个
//! `.SC2Mod` 或者一包模组，装进游戏 `Mods/` 就能用。这类模组也得能
//! 导入、启停、改信息、再导出。
//!
//! 目录结构（与战役库并列）：
//!
//! `@text
//! data/
//! ├── mods.json              # 索引：有哪些模组、各是什么信息、启用了没
//! ├── mods/<id>/             # 各模组的完整内容
//! └── mods-active.json       # 我们在游戏 Mods/ 里放了什么（回滚用）
//! `@
//!
//! **模组按文件夹分**：一个模组目录里可能有十几个 `.SC2Mod`
//! （真实样本里的 `Alenger` 就是），整包进整包出。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::safety;

/// 我们往游戏目录里放过的独立模组（回滚用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ActiveList {
    #[serde(default)]
    placed: Vec<String>,
}

/// 我们铺进游戏目录时的目录名：`<模组名>.SC2Mod`。
///
/// 用显示名而不是 id —— 游戏和别的工具只认这个名字，用户看着也顺眼。
fn placed_name(record: &StandaloneMod) -> String {
    format!("{}.SC2Mod", record.name)
}

/// 把**启用**的独立模组铺进 `<游戏>/Mods/`，顺手撤掉之前铺过、现在停用的。
///
/// 只动**我们自己记过账**的东西：用户手动放的模组一个都不碰。
/// 返回这次实际铺进去的名字。
pub fn sync(data: &Path, installation: &crate::sc2::Installation) -> Result<Vec<String>> {
    let active_path = data.join("mods-active.json");
    let previous: ActiveList = std::fs::read_to_string(&active_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();

    std::fs::create_dir_all(&installation.mods_root)?;
    let mods_root = mods_root(data);
    let mut placed: Vec<String> = Vec::new();

    for record in list(data) {
        let name = placed_name(&record);
        let source = safety::ensure_within(&mods_root, &mods_root.join(&record.id))?;

        if !record.enabled {
            continue;
        }
        if !source.is_dir() {
            continue;
        }

        let dest =
            safety::ensure_within(&installation.mods_root, &installation.mods_root.join(&name))?;
        if dest.exists() {
            // 已经在了就整体换掉，免得新旧混在一起
            let _ = std::fs::remove_dir_all(&dest);
            let _ = std::fs::remove_file(&dest);
        }
        copy_tree(&source, &dest)?;
        placed.push(name);
    }

    // 撤掉之前铺过、这次不该留的
    for name in &previous.placed {
        if placed.contains(name) {
            continue;
        }
        if let Ok(path) =
            safety::ensure_within(&installation.mods_root, &installation.mods_root.join(name))
        {
            let _ = std::fs::remove_dir_all(&path);
            let _ = std::fs::remove_file(&path);
        }
    }

    let text = serde_json::to_string_pretty(&ActiveList {
        placed: placed.clone(),
    })?;
    std::fs::write(&active_path, text)?;

    Ok(placed)
}

/// 库里的一个独立模组。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneMod {
    /// 目录名，也是本地唯一键。
    pub id: String,
    /// 显示名（可以改）。
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// **modid**：包格式里声明的模组身份。
    ///
    /// 判定「这是同一个模组的不同版本，还是另一个模组」全看它：
    /// id 相同 -> 同一个模组的不同版本；id 不同 -> 新模组。
    /// 没声明时退回用显示名当 id（老记录、随手传的包都没有）。
    #[serde(default)]
    pub modid: Option<String>,
    /// 内容的指纹（blake3）。用来判定「完全一样，不用再存一份」。
    #[serde(default)]
    pub fingerprint: String,
    /// 启用了没 —— 启用才会铺进游戏目录。
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub imported_at: u64,
    #[serde(default)]
    pub size_bytes: u64,
    /// 目录里有多少个 `.SC2Mod`。
    #[serde(default)]
    pub parts: usize,
}

/// 导入前的预检：这个包是谁、库里有没有同族的。
///
/// 界面拿它决定要不要问一句 —— 「这看起来是 X 的另一个版本，
/// 作为它的新版本，还是当成一个独立改版？」不确认就直接并进去太武断。
#[derive(Debug, Clone, Serialize)]
pub struct ModPreview {
    /// 包里的显示名。
    pub name: String,
    /// 认出来的 modid（包内声明的，或退回名字）。
    pub modid: String,
    /// 内容指纹。
    pub fingerprint: String,
    /// 库里同 modid 的已有版本。
    pub existing: Vec<StandaloneMod>,
    /// 库里是否已有一份**内容完全一样**的。
    pub duplicate: bool,
    /// 归到哪个版本号（按库里已有的推）。
    pub suggested_version: String,
}

/// 导入时怎么处理与已有模组的关系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModImportMode {
    /// 自动：同 modid 就归并成版本（默认）。
    #[default]
    Auto,
    /// **作为新版本**：归到同一个模组下面，可切换。
    Version,
    /// **作为独立改版**：即使 modid 相同也单独显示一个。
    ///
    /// 用于「基于别人的模组改的」那种 —— 跟原版是两码事，混在一个版本列表里
    /// 反而让人以为它们是同一个东西的不同时期。
    Separate,
}

/// 导入一个模组时发生了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModImportAction {
    /// 全新导进来的。
    Added,
    /// 同一个 modid 的新版本。
    NewVersion,
    /// **内容完全一样** —— 没重复存，直接用库里那份。
    Duplicate,
    /// 版本号撞了但内容不同 —— 自动改了个版本号存下来。
    Renamed,
}

/// 导入模组的结果。
#[derive(Debug, Clone, Serialize)]
pub struct ModImport {
    /// 库里最终的那条记录（可能是已有的那份，也可能是新建的）。
    pub record: StandaloneMod,
    pub action: ModImportAction,
    /// 库里原来那个（如果有）。
    pub existing: Option<StandaloneMod>,
    /// 界面该怎么说。
    pub message: String,
}

/// 允许用户修改的模组信息；`None` 表示这一项不动。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModChanges {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

impl ModChanges {
    /// 是不是什么都没改。
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.author.is_none()
            && self.version.is_none()
            && self.description.is_none()
    }
}

/// 索引文件。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ModIndex {
    #[serde(default)]
    mods: Vec<StandaloneMod>,
}

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

/// 独立模组库的根目录（`data/mods`）。
fn mods_root(data: &Path) -> PathBuf {
    data.join("mods")
}

/// 索引文件路径。
fn index_path(data: &Path) -> PathBuf {
    data.join("mods.json")
}

/// 读索引；文件不在或读坏了都当空处理，不让界面起不来。
pub fn list(data: &Path) -> Vec<StandaloneMod> {
    let mut rows = std::fs::read_to_string(index_path(data))
        .ok()
        .and_then(|text| serde_json::from_str::<ModIndex>(&text).ok())
        .unwrap_or_default()
        .mods;

    rows.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    rows
}

/// 写索引。
fn save(data: &Path, mods: &[StandaloneMod]) -> Result<()> {
    std::fs::create_dir_all(data)?;
    let index = ModIndex {
        mods: mods.to_vec(),
    };
    let text = serde_json::to_string_pretty(&index)?;
    std::fs::write(index_path(data), text)?;
    Ok(())
}

/// 取一个模组。
pub fn get(data: &Path, id: &str) -> Option<StandaloneMod> {
    list(data).into_iter().find(|item| item.id == id)
}

/// 一个目录里有多少个 `.SC2Mod`，以及总体积。
fn scan_dir(root: &Path) -> (usize, u64) {
    let mut parts = 0usize;
    let mut bytes = 0u64;

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        // 只有 .SC2Mod 本身算「一个部件」；它解开之后的内部文件不算
        if name.ends_with(".sc2mod") {
            parts += 1;
            bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        }
    }

    // 没有裸 .SC2Mod（可能是单文件形态）时，退回按全局体积算
    if parts == 0 {
        bytes = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| entry.metadata().ok())
            .map(|meta| meta.len())
            .sum();
        parts = 1;
    }

    (parts, bytes)
}

/// 算一份内容的指纹（SHA-256）。
///
/// 拿它判定「完全一样」 —— 比逐字节比对省事，也比比名字可靠：
/// 同一个包换个文件名再传一次是很常见的。
fn fingerprint_of(root: &Path) -> Result<String> {
    // 用 sha2 而不是 blake3：它已经在依赖里了，不为一个指纹再加一个 crate
    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }

    // 路径排序后再喂 —— 不然遍历顺序一变指纹就变了
    files.sort();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        // 路径也进哈希：同样的内容摆在不同的子目录里，是两个模组
        hasher.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        hasher.update(&std::fs::read(&path)?);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// 这个记录的身份：优先用声明的 modid，没有就退回名字。
pub fn effective_id(record: &StandaloneMod) -> String {
    record
        .modid
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| record.name.clone())
}

/// 把来源解到暂存目录，返回 (暂存目录, 显示名, 原始文件名)。
///
/// 预检和导入都要用：先算指纹、跟库里比对，比完才知道要不要留下。
fn stage(data: &Path, source: &Path) -> Result<(PathBuf, String, String)> {
    if !source.exists() {
        return Err(Error::PackageRejected(format!(
            "找不到要导入的模组：{}",
            source.display()
        )));
    }

    let raw_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "未命名模组".to_string());
    let name = strip_archive_suffix(&raw_name);
    let display = name
        .strip_suffix(".SC2Mod")
        .or_else(|| name.strip_suffix(".sc2mod"))
        .unwrap_or(&name)
        .to_string();

    let root = mods_root(data);
    std::fs::create_dir_all(&root)?;

    let staging = safety::ensure_within(
        &root,
        &root.join(format!(".staging-{}", crate::library::now_seconds())),
    )?;
    std::fs::create_dir_all(&staging)?;

    let staged = (|| -> Result<()> {
        if source.is_dir() {
            copy_tree(source, &staging)?;
        } else if is_archive(source) {
            crate::campaign::package::extract_to(source, "", &staging)?;
            collapse_single_dir(&staging)?;
        } else {
            std::fs::copy(source, staging.join(&raw_name))?;
        }
        Ok(())
    })();

    if let Err(error) = staged {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }

    Ok((staging, display, raw_name))
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
    let (staging, display, _) = stage(data, source)?;

    let fingerprint = fingerprint_of(&staging)?;
    let _ = std::fs::remove_dir_all(&staging);

    let modid = declared_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| display.clone());

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
    // 包内声明的版本优先；没声明才从库里已有的推
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
        modid,
        fingerprint,
        existing,
        duplicate,
        suggested_version: suggested,
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
    let (staging, display, _raw_name) = stage(data, source)?;
    let root = mods_root(data);

    let fingerprint = fingerprint_of(&staging)?;
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

    let existing_list = list(data);
    let same_id: Vec<StandaloneMod> = existing_list
        .iter()
        .filter(|item| effective_id(item) == modid)
        .cloned()
        .collect();

    // ---- 内容完全一样：不留第二份 ----
    if let Some(found) = same_id
        .iter()
        .find(|item| !item.fingerprint.is_empty() && item.fingerprint == fingerprint)
    {
        let _ = std::fs::remove_dir_all(&staging);
        return Ok(ModImport {
            record: found.clone(),
            action: ModImportAction::Duplicate,
            existing: Some(found.clone()),
            message: format!("「{}」库里已经有一份完全一样的，没有重复存", found.name),
        });
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
            .filter_map(|item| item.version.clone())
            .collect();
        let wanted =
            declared.unwrap_or_else(|| taken.last().cloned().unwrap_or_else(|| "1.0".to_string()));
        Some(unique_version(&wanted, &taken, &mut action))
    };

    let id = unique_dir_name(&root, &display)?;
    let target = safety::ensure_within(&root, &root.join(&id))?;
    std::fs::rename(&staging, &target)?;

    let (parts, size_bytes) = scan_dir(&target);
    let record = StandaloneMod {
        id: id.clone(),
        name: display,
        author: None,
        version,
        description: None,
        modid: Some(modid.clone()),
        fingerprint,
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

    Ok(ModImport {
        record,
        action,
        existing: same_id.into_iter().next(),
        message,
    })
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

/// 只有一层目录时往里收一层 —— 解压出来的 `Alenger/Alenger/…` 很烦人。
fn collapse_single_dir(root: &Path) -> Result<()> {
    let entries: Vec<_> = std::fs::read_dir(root)?
        .filter_map(std::result::Result::ok)
        .collect();
    if entries.len() != 1 || !entries[0].path().is_dir() {
        return Ok(());
    }

    let inner = entries[0].path();
    let temp = root.join(".collapse");
    std::fs::rename(&inner, &temp)?;
    std::fs::remove_dir(root)?;
    std::fs::rename(&temp, root)?;
    Ok(())
}

/// 改模组信息。**只改启动器记录的，不动文件**。
pub fn update(data: &Path, id: &str, changes: ModChanges) -> Result<StandaloneMod> {
    let mut mods = list(data);
    let record = mods
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;

    if let Some(name) = changes.name.filter(|value| !value.trim().is_empty()) {
        record.name = name;
    }
    if changes.author.is_some() {
        record.author = changes.author;
    }
    if changes.version.is_some() {
        record.version = changes.version;
    }
    if changes.description.is_some() {
        record.description = changes.description;
    }

    let updated = record.clone();
    save(data, &mods)?;
    Ok(updated)
}

/// 启用 / 停用一个模组。**只改记录** —— 真正铺进游戏目录要走 `sync`。
///
/// 启用时会**把同 modid 的其它版本关掉**：它们在游戏目录里抢的是同一个文件夹，
/// 同时开着只会互相覆盖，留下一堆说不清是谁的文件。
pub fn set_enabled(data: &Path, id: &str, enabled: bool) -> Result<StandaloneMod> {
    let mut mods = list(data);
    let target_id = mods
        .iter()
        .find(|item| item.id == id)
        .map(effective_id)
        .ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;

    for item in mods.iter_mut() {
        if item.id == id {
            item.enabled = enabled;
        } else if enabled && effective_id(item) == target_id {
            // 同一个模组的另一个版本：让位
            item.enabled = false;
        }
    }

    let updated = mods
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;

    save(data, &mods)?;
    Ok(updated)
}

/// 删掉一个模组。
pub fn remove(data: &Path, id: &str) -> Result<()> {
    let root = mods_root(data);
    let dir = safety::ensure_within(&root, &root.join(id))?;
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }

    let mut mods = list(data);
    mods.retain(|item| item.id != id);
    save(data, &mods)?;
    Ok(())
}

/// 把模组打包导出成一个 zip。
pub fn export(data: &Path, id: &str, target: &Path) -> Result<()> {
    let record = get(data, id).ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;
    let root = mods_root(data);
    let dir = safety::ensure_within(&root, &root.join(&record.id))?;
    if !dir.is_dir() {
        return Err(Error::PackageRejected("模组文件已经不在了".to_string()));
    }

    let file = std::fs::File::create(target)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut wrote = 0usize;
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry
            .path()
            .strip_prefix(&dir)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        let name = relative.to_string_lossy().replace('`', "/");
        if name.is_empty() {
            continue;
        }

        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), options)?;
        } else {
            zip.start_file(name, options)?;
            let mut input = std::fs::File::open(entry.path())?;
            std::io::copy(&mut input, &mut zip)?;
            wrote += 1;
        }
    }

    zip.finish()?;
    if wrote == 0 {
        let _ = std::fs::remove_file(target);
        return Err(Error::PackageRejected("这个模组目录是空的".to_string()));
    }
    Ok(())
}

/// 导出**选中的若干版本**到一个 zip。
///
/// 为什么要能挑：一个模组攒了十几个版本之后，全打包出去又大又没人要；
/// 而「把 1.0 和 1.3 一起发过去让人对比」是很实际的需求。
///
/// 包里的摆法：
///
/// `@text
/// <名字>/
/// ├── mods.txt          索引：这个包里带了哪些版本
/// ├── 1.0/             各版本的完整内容
/// └── 1.3/
/// `@
pub fn export_many(data: &Path, ids: &[String], target: &Path) -> Result<usize> {
    if ids.is_empty() {
        return Err(Error::PackageRejected("没有选中任何版本".to_string()));
    }

    let root = mods_root(data);
    let records: Vec<StandaloneMod> = ids.iter().filter_map(|id| get(data, id)).collect();

    if records.is_empty() {
        return Err(Error::PackageRejected("选中的模组都不在库里".to_string()));
    }

    // 用第一个的名字当顶层目录 —— 同 modid 的多个版本本来就是一家人
    let title = crate::campaign::sanitize::sanitize_dir_name(&records[0].name)
        .unwrap_or_else(|| "mod".to_string());

    let file = std::fs::File::create(target)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 索引
    let mut index = String::new();
    index.push_str(
        "# 这个模组包里带了以下版本
",
    );
    if let Some(modid) = records[0].modid.as_deref() {
        index.push_str(&format!(
            "modid={modid}
"
        ));
    }
    index.push_str(&format!(
        "title={title}
"
    ));
    for record in &records {
        index.push_str(&format!(
            "version={}    # 原始目录 {}，{} MB
",
            record
                .version
                .clone()
                .unwrap_or_else(|| "未标版本".to_string()),
            record.id,
            record.size_bytes / 1024 / 1024
        ));
    }
    zip.start_file(format!("{title}/mods.txt"), options)?;
    std::io::Write::write_all(&mut zip, index.as_bytes())?;

    let mut written = 0usize;
    for record in &records {
        let dir = safety::ensure_within(&root, &root.join(&record.id))?;
        if !dir.is_dir() {
            continue;
        }

        let label = record.version.clone().unwrap_or_else(|| record.id.clone());
        let label = crate::campaign::sanitize::sanitize_dir_name(&label)
            .unwrap_or_else(|| record.id.clone());

        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let relative = entry
                .path()
                .strip_prefix(&dir)
                .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
            let name = relative.to_string_lossy().replace('`', "/");
            if name.is_empty() {
                continue;
            }
            let name = format!("{title}/{label}/{name}");

            if entry.file_type().is_dir() {
                zip.add_directory(format!("{name}/"), options)?;
            } else {
                zip.start_file(name, options)?;
                let mut input = std::fs::File::open(entry.path())?;
                std::io::copy(&mut input, &mut zip)?;
                written += 1;
            }
        }
    }

    zip.finish()?;
    if written == 0 {
        let _ = std::fs::remove_file(target);
        return Err(Error::PackageRejected("选中的版本目录都是空的".to_string()));
    }

    Ok(written)
}

/// 两个模组版本之间的一个文件。
#[derive(Debug, Clone, Serialize)]
pub struct ModFileDiff {
    /// 相对模组目录的路径。
    pub path: String,
    /// `same` / `changed` / `added` / `removed`。
    pub status: String,
    pub size_before: u64,
    pub size_after: u64,
}

/// 两个模组版本的对比结果。
#[derive(Debug, Clone, Serialize)]
pub struct ModComparison {
    pub before: StandaloneMod,
    pub after: StandaloneMod,
    /// 内容完全一样（指纹相同）。
    pub identical: bool,
    /// 逐文件的差异。
    pub files: Vec<ModFileDiff>,
    /// SC2Diff 给出的**语义 diff**（按文件）。
    ///
    /// 只有装了 SC2Diff、而且那个文件是**单文件形态**的 `.SC2Mod` 才有 ——
    /// SC2Diff 处理的是文档，解开成目录树的它得先 pack 回去，这里不做。
    /// 拿不到就留空，界面显示「未做语义 diff」而不是假装没有差异。
    pub semantic: Vec<ModSemanticDiff>,
    /// 没跑语义 diff 的原因。
    pub semantic_note: Option<String>,
}

/// 一份文件的语义 diff。
#[derive(Debug, Clone, Serialize)]
pub struct ModSemanticDiff {
    pub path: String,
    pub text: String,
}

/// 列一个模组目录里的文件（相对路径 -> (是否文件, 大小)）。
fn list_files(root: &Path) -> Result<std::collections::BTreeMap<String, (bool, u64)>> {
    let mut map = std::collections::BTreeMap::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
        let key = relative.to_string_lossy().replace('\\', "/");
        if key.is_empty() {
            continue;
        }
        let is_file = entry.file_type().is_file();
        let size = if is_file {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        } else {
            0
        };
        map.insert(key, (is_file, size));
    }

    Ok(map)
}

/// 比两个模组版本。
///
/// 用途有两个：让用户看清「这两个版本差在哪」，也是**将来只存 diff** 的前置
/// —— 看不见差异就谈不上只存差异。
pub fn compare(data: &Path, before_id: &str, after_id: &str) -> Result<ModComparison> {
    let before =
        get(data, before_id).ok_or_else(|| Error::CampaignNotFound(before_id.to_string()))?;
    let after = get(data, after_id).ok_or_else(|| Error::CampaignNotFound(after_id.to_string()))?;

    let root = mods_root(data);
    let dir_before = safety::ensure_within(&root, &root.join(&before.id))?;
    let dir_after = safety::ensure_within(&root, &root.join(&after.id))?;

    let files_before = list_files(&dir_before)?;
    let files_after = list_files(&dir_after)?;

    let mut files: Vec<ModFileDiff> = Vec::new();
    let mut semantic: Vec<ModSemanticDiff> = Vec::new();

    let mut keys: Vec<&String> = files_before.keys().chain(files_after.keys()).collect();
    keys.sort();
    keys.dedup();

    for key in keys {
        let left = files_before.get(key);
        let right = files_after.get(key);

        let status = match (left, right) {
            (Some(_), None) => "removed",
            (None, Some(_)) => "added",
            (Some((_, a)), Some((_, b))) => {
                if a == b && same_bytes(&dir_before.join(key), &dir_after.join(key)) {
                    "same"
                } else {
                    "changed"
                }
            }
            (None, None) => continue,
        };

        files.push(ModFileDiff {
            path: key.clone(),
            status: status.to_string(),
            size_before: left.map(|(_, size)| *size).unwrap_or(0),
            size_after: right.map(|(_, size)| *size).unwrap_or(0),
        });
    }

    let identical = !before.fingerprint.is_empty() && before.fingerprint == after.fingerprint;

    // 语义 diff：只在装了 SC2Diff、且改动的是**单文件** .SC2Mod 时才做
    let mut note: Option<String> = None;
    if crate::tools::sc2diff::locate(data).is_none() {
        note = Some("没装 SC2Diff，只列出了文件级差异".to_string());
    } else if identical {
        note = Some("两个版本内容完全一样，没什么好比的".to_string());
    } else {
        let tmp = crate::library::default_root(data)
            .join("diff-work")
            .join(crate::library::now_seconds().to_string());

        for item in files.iter().filter(|item| item.status == "changed") {
            let left = dir_before.join(&item.path);
            let right = dir_after.join(&item.path);
            // SC2Diff 吃的是**打包好的文档**；解开成目录树的跳过
            if !left.is_file() || !right.is_file() {
                continue;
            }
            if !item.path.to_ascii_lowercase().ends_with(".sc2mod")
                && !item.path.to_ascii_lowercase().ends_with(".sc2map")
            {
                continue;
            }

            let repo = tmp.join(format!("r{}", semantic.len()));
            match crate::tools::sc2diff::semantic_diff(data, &left, &right, &repo) {
                Ok(text) => semantic.push(ModSemanticDiff {
                    path: item.path.clone(),
                    text,
                }),
                Err(error) => {
                    note = Some(format!("语义 diff 没跑通：{error}"));
                    break;
                }
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);

        if note.is_none() && semantic.is_empty() {
            note = Some("改动都不在可比较的文档上，只列出了文件级差异".to_string());
        }
    }

    Ok(ModComparison {
        before,
        after,
        identical,
        files,
        semantic,
        semantic_note: note,
    })
}

/// 两个文件是不是逐字节一样。
fn same_bytes(left: &Path, right: &Path) -> bool {
    match (std::fs::read(left), std::fs::read(right)) {
        (Ok(a), Ok(b)) => a == b,
        // 读不了（比如是目录）就当作「不一样」，反正上面已经比过大小了
        _ => false,
    }
}

/// 模组目录里有没有 `.SC2Mod`。
pub fn looks_like_mod(path: &Path) -> bool {
    if path.is_file() {
        return path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("sc2mod"));
    }

    walkdir::WalkDir::new(path)
        .max_depth(2)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .any(|entry| {
            entry.file_type().is_dir()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".sc2mod")
        })
        || walkdir::WalkDir::new(path)
            .max_depth(1)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".sc2mod")
            })
}
