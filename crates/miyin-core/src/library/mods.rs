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
pub fn import(data: &Path, source: &Path, declared_id: Option<&str>) -> Result<ModImport> {
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

    // 先解到暂存目录：还要算指纹、跟库里比对，比完才知道要不要留下
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

    let fingerprint = fingerprint_of(&staging)?;
    let modid = declared_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| display.clone());

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

    // ---- 决定版本号与目录名 ----
    let mut version = same_id
        .iter()
        .filter_map(|item| item.version.clone())
        .next_back();
    let mut action = ModImportAction::Added;

    if !same_id.is_empty() {
        // 传进来的包自己声明的版本还不知道（元数据在调用方解析），
        // 这里按「库里已有同 id」处理：先当新版本，撞版本号再改名
        action = ModImportAction::NewVersion;

        let taken: Vec<String> = same_id
            .iter()
            .filter_map(|item| item.version.clone())
            .collect();
        version = Some(unique_version(
            version.as_deref().unwrap_or("1.0"),
            &taken,
            &mut action,
        ));
    } else {
        version = Some("1.0".to_string());
    }

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
pub fn set_enabled(data: &Path, id: &str, enabled: bool) -> Result<StandaloneMod> {
    let mut mods = list(data);
    let record = mods
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;
    record.enabled = enabled;

    let updated = record.clone();
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
