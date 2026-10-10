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

/// 一次铺盘的结果。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncReport {
    /// 这次实际铺进去的名字。
    pub placed: Vec<String>,
    /// 冲突提示 —— 谁盖了谁。以前是静默覆盖，出了问题查不出来。
    pub warnings: Vec<String>,
}

/// 我们铺进游戏目录时用的名字 —— **原样返回，一个字符都不改**。
///
/// 名字是地图跟模组之间的契约：地图里写的是 `Mods\Alenger\…`，
/// 我们就必须铺出 `Mods/Alenger/…`。加后缀、换大小写、去掉空格，
/// 任何一处自作主张都会让地图找不到模组。
pub fn placed_name(record: &StandaloneMod) -> String {
    if record.folder.is_empty() {
        // 老记录（还没有 folder 字段）退回用显示名；这里**不加后缀**，
        // 宁可名字不对也不能凭空造一个游戏不认识的名字
        record.name.clone()
    } else {
        record.folder.clone()
    }
}

/// 把**启用**的独立模组铺进 `<游戏>/Mods/`，顺手撤掉停用的。
///
/// **走的是和战役同一条写盘路径**（`super::install::Manifest`）——
/// 白名单校验、备份还原、记账、回滚只有那一份实现。
/// 以前这里自己写清单（`mods-active.json`），和战役的 `active.json`
/// 互相不知道对方放了什么，两边抢同一个位置时谁也说不清。
///
/// 只动**我们自己记过账**的东西：用户手动放的模组一个都不碰。
pub fn sync(data: &Path, installation: &crate::sc2::Installation) -> Result<SyncReport> {
    // 用迁移版：老格式的 mods-active.json 也要认
    let mut manifest = crate::library::install::Manifest::load_migrating(data, installation);
    let all = list(data);

    // **跟着战役包来的模组，战役没启用就不铺。**
    //
    // 战役停用了、它带的模组却还躺在游戏目录里，那叫"停了个寂寞" ——
    // 而且那些模组本来就是跟着那一版战役来的，战役不在，它们也没有意义。
    // 用户的独立模组不受影响（它们自己那面 enabled 说了算）。
    let index = super::Library::new(data.to_path_buf()).index();
    let active: std::collections::BTreeMap<String, String> = index
        .slots
        .iter()
        .filter_map(|(slot, entry)| entry.active.clone().map(|variant| (slot.clone(), variant)))
        .collect();

    /// 这条记录该不该铺；返回 Some(原因) 表示不该。
    fn why_not(
        record: &StandaloneMod,
        active: &std::collections::BTreeMap<String, String>,
    ) -> Option<String> {
        if !record.enabled {
            return Some("停用".to_string());
        }
        if let ModSource::Campaign { slot, variant, .. } = &record.source
            && active.get(slot) != Some(variant)
        {
            return Some(format!("它跟着的战役「{slot}」这一版没启用"));
        }
        None
    }

    // ---- 1) 先撤掉不该留的 ----
    //
    // 三种情况：用户停用了、**它跟着的战役没启用**、模组被删了/库里没记录了。
    let existing: Vec<String> = all.iter().map(|item| item.id.clone()).collect();
    let mut stale: Vec<crate::library::install::Owner> = Vec::new();

    for record in &all {
        if why_not(record, &active).is_some() {
            stale.push(crate::library::install::Owner::Mod {
                id: record.id.clone(),
            });
        }
    }
    for item in &manifest.files {
        if let crate::library::install::Owner::Mod { id } = &item.owner
            && !existing.contains(id)
            && !stale.iter().any(|owner| owner == &item.owner)
        {
            stale.push(item.owner.clone());
        }
    }

    for owner in stale {
        manifest.remove(data, installation, &owner)?;
    }

    // ---- 2) 装启用中的 ----
    let mods_root = mods_root(data);
    let mut placed: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 落点撞车要说话：两个模组铺到同一个名字，后铺的会盖掉先铺的。
    // "重复的 mod"最常见的形态就是这个 —— 同一个模组导了两份、或者两个包带了同名模组。
    let mut taken: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    for record in all.iter().filter(|item| why_not(item, &active).is_none()) {
        let source = safety::ensure_within(&mods_root, &mods_root.join(&record.id))?;
        let name = placed_name(record);

        if let Some(previous) = taken.insert(name.clone(), record.name.clone()) {
            warnings.push(format!(
                "「{previous}」和「{}」都要铺成 Mods/{name}，后者会盖掉前者 —— 建议停用其中一个",
                record.name
            ));
        }

        let mut plan = crate::library::install::Plan::new(crate::library::install::Owner::Mod {
            id: record.id.clone(),
        });

        // **目标要带 Mods/ 前缀** —— Plan 里的路径是相对**游戏根**的，
        // 而 placed_name() 给的是 Mods/ 底下的名字。
        let target = format!("Mods/{name}");

        // 形态要和原来一致：单个 .SC2Mod 铺成一个**文件**，
        // 目录铺成**目录**。搞错了游戏照样找不到。
        match record.kind {
            ModKind::File => {
                let inner = single_child_file(&source).ok_or_else(|| {
                    Error::PackageRejected(format!("模组 {} 的内容不是一个文件", record.name))
                })?;
                plan.push(inner, target);
            }
            ModKind::Folder => plan.push(source.clone(), target),
        }

        warnings.extend(manifest.apply(data, installation, &plan)?);
        placed.push(name);
    }

    // 因为"战役没启用"而没铺的，明确说一句 —— 默默不铺会让人以为模组丢了
    for record in &all {
        if let Some(reason) = why_not(record, &active)
            && record.enabled
            && reason != "停用"
        {
            warnings.push(format!("「{}」这次没铺：{reason}", record.name));
        }
    }

    Ok(SyncReport { placed, warnings })
}

/// 独立模组铺进游戏目录时的**形态**。
///
/// 真实样本里两种都有：
///
/// `@text
/// Mods/3疯批帝国之翼.SC2Mod      <- 单个文件
/// Mods/Alenger/                  <- 目录，里面 18 个 .SC2Mod
/// Mods/kit_liberty_story.SC2Mod/ <- 目录（名字带后缀的解开形态）
/// `@
///
/// 形态决定了铺过去是「一个文件」还是「一个目录」—— 弄错了地图一样找不到。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModKind {
    /// 单个 `.SC2Mod` 文件。
    File,
    /// 一个目录（连同里面的所有东西）。
    #[default]
    Folder,
}

/// 一个模组的**内容在哪**。
///
/// 不管内容在哪，它都是一条「模组记录」—— 有自己的名字、版本、作者、modid，
/// 可以编辑、可以导出。区别只在内容的落脚点：
///
/// `@text
/// Library   独立导入的，内容在 data/mods/<id>/
/// Campaign  跟着战役包来的，内容在 data/campaigns/<槽位>/<版本>/ 里
/// `@
///
/// 分开记是为了「战役包带来的模组也能改信息」—— 改动存在记录里（覆盖包内的），
/// 不动包本身。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModSource {
    /// 独立导入，内容就在库里。
    #[default]
    Library,
    /// 跟着某个战役版本来的，内容在那个版本的目录里。
    Campaign {
        slot: String,
        variant: String,
        /// 相对版本目录的路径（就是那个模组文件夹）。
        path: String,
    },
}

/// 库里的一个模组记录。
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
    /// 内容的指纹（SHA-256）。用来判定「完全一样，不用再存一份」。
    #[serde(default)]
    pub fingerprint: String,
    /// **铺进游戏目录时的名字，原样保留**。
    ///
    /// 这一点至关重要：地图里声明的是 `Mods\Alenger钢铁.SC2Mod` 这种路径，
    /// 名字对不上游戏就找不到模组，地图直接打不开。
    /// 所以 `Alenger` 就必须是 `Alenger` —— 不能自作主张变成
    /// `Alenger.SC2Mod`（踩过：以前拿显示名拼了个后缀，把这类模组全废了）。
    #[serde(default)]
    pub folder: String,
    /// 铺成文件还是目录。
    #[serde(default)]
    pub kind: ModKind,
    /// 内容在哪（独立导入的 / 跟着战役包来的）。
    #[serde(default)]
    pub source: ModSource,
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
    /// 铺进游戏目录时用的名字（原样保留）。
    pub folder: String,
    /// 这一包里有几个模组 —— 真实样本的 `Mods/` 下有 3 个。
    pub mod_count: usize,
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
    ///
    /// 一个包装了多个模组时，这里是**第一个**；全部在 `records` 里。
    pub record: StandaloneMod,
    /// 这次涉及的所有模组 —— 一个包的 `Mods/` 下可能有好几个。
    #[serde(default)]
    pub records: Vec<StandaloneMod>,
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
fn single_child_file(root: &Path) -> Option<PathBuf> {
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
fn stage(data: &Path, source: &Path) -> Result<Staged> {
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
            crate::campaign::package::extract_to(source, "", &raw)?;
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
            std::fs::rename(&content, slot.join(&folder))?;
        } else {
            std::fs::rename(&content, &slot)?;
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
    let staged = stage(data, source)?;
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
    let staged = stage(data, source)?;
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
    std::fs::rename(&item.content, &target)?;

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

/// 换战役版本时，把「跟着这个版本来的」模组记录改指到新版本上。
///
/// 这类记录**只存元数据，内容就在战役版本目录里** —— 版本一换，内容跟着走，
/// 记录不跟着改就会指向一个已经不存在的目录：列表里还在，铺下去却没有东西。
/// 返回改了几条。
pub fn repoint_campaign(data: &Path, slot: &str, from: &str, to: &str) -> Result<usize> {
    let mut rows = list(data);
    let mut moved = 0;
    for row in rows.iter_mut() {
        if let ModSource::Campaign {
            slot: row_slot,
            variant,
            ..
        } = &mut row.source
            && row_slot == slot
            && variant == from
        {
            *variant = to.to_string();
            moved += 1;
        }
    }
    if moved > 0 {
        save(data, &rows)?;
    }
    Ok(moved)
}

/// 找一条「跟着某个战役版本来的」模组记录。
pub fn find_campaign(data: &Path, slot: &str, variant: &str, path: &str) -> Option<StandaloneMod> {
    list(data).into_iter().find(|item| {
        matches!(
            &item.source,
            ModSource::Campaign {
                slot: item_slot,
                variant: item_variant,
                path: item_path,
            } if item_slot == slot && item_variant == variant && item_path == path
        )
    })
}

/// **记住**一个跟着战役包来的模组，好让它也能改信息、也能单独导出。
///
/// 库里没有就建一条（`ModSource::Campaign`，**只存元数据**，内容仍在战役版本
/// 目录里，不复制一份）；有就原样返回。
///
/// 为什么要这一步：战役包带来的模组本来只能看不能改 ——
/// 名字、版本、作者全是包内写死的。现在它们和独立模组一样是一条**模组记录**，
/// 改的是记录（覆盖包内声明），不动包本身。
#[allow(clippy::too_many_arguments)]
pub fn remember_campaign(
    data: &Path,
    slot: &str,
    variant: &str,
    path: &str,
    folder: &str,
    name: &str,
    version: Option<&str>,
    kind: ModKind,
    parts: usize,
) -> Result<StandaloneMod> {
    if let Some(found) = find_campaign(data, slot, variant, path) {
        return Ok(found);
    }

    // 记录 id 只要唯一、稳定就行 —— 用来源拼一个，不用中文当目录名
    let id = format!("campaign-{slot}-{variant}-{}", short_hash(path));

    let record = StandaloneMod {
        id,
        name: name.to_string(),
        author: None,
        version: version.map(str::to_string),
        description: None,
        modid: Some(name.to_string()),
        fingerprint: String::new(),
        folder: folder.to_string(),
        kind,
        source: ModSource::Campaign {
            slot: slot.to_string(),
            variant: variant.to_string(),
            path: path.to_string(),
        },
        // 战役模组的启停由战役那条线管（mounted_mods），这里不掺和
        enabled: false,
        imported_at: crate::library::now_seconds(),
        size_bytes: 0,
        parts,
    };

    let mut mods = list(data);
    mods.push(record.clone());
    save(data, &mods)?;
    Ok(record)
}

/// 路径的短哈希，用来拼一个稳定的记录 id。
fn short_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string()
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
    // 抢同一个位置有两种情形：同一个 modid 的另一个版本；或者 modid 不同
    // （「作为独立改版」导入的副本是 X#alt）却落到同一个 Mods/<folder>。
    // 两种都得让位 —— 只比 modid 的话，后一种会同时开着互相覆盖。
    let (target_id, target_place) = mods
        .iter()
        .find(|item| item.id == id)
        .map(|item| (effective_id(item), placed_name(item)))
        .ok_or_else(|| Error::CampaignNotFound(id.to_string()))?;

    for item in mods.iter_mut() {
        if item.id == id {
            item.enabled = enabled;
        } else if enabled && (effective_id(item) == target_id || placed_name(item) == target_place)
        {
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
        let name = relative.to_string_lossy().replace('\\', "/");
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
            let name = relative.to_string_lossy().replace('\\', "/");
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
        let tmp = data
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sc2::{DiscoverySource, Installation};

    fn mod_record(id: &str, folder: &str, source: ModSource) -> StandaloneMod {
        StandaloneMod {
            id: id.to_string(),
            name: folder.to_string(),
            author: None,
            version: None,
            description: None,
            modid: None,
            fingerprint: String::new(),
            folder: folder.to_string(),
            kind: ModKind::File,
            source,
            enabled: true,
            imported_at: 0,
            size_bytes: 0,
            parts: 1,
        }
    }

    /// **战役没启用，它带来的模组就不该铺进游戏目录。**
    ///
    /// 战役停了、模组还躺在 `Mods/` 里，那叫"停了个寂寞" —— 而且那些模组本来就是
    /// 跟着那一版战役来的。用户自己导的独立模组不受影响。
    #[test]
    fn campaign_mods_follow_their_campaign() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let game = tmp.path().join("游戏");
        std::fs::create_dir_all(&game).expect("建目录");
        std::fs::write(game.join("StarCraft II.exe"), b"stub").expect("标记");
        let installation = Installation::from_root(&game, DiscoverySource::Manual).expect("安装");

        let data = tmp.path().join("data");

        // 独立模组：内容在库里
        let lib_dir = data.join("mods").join("lib1");
        std::fs::create_dir_all(&lib_dir).expect("建目录");
        std::fs::write(lib_dir.join("独立.SC2Mod"), b"x").expect("写");

        // 战役带来的模组：内容在战役版本目录里
        let camp_dir = data
            .join("campaigns")
            .join("wol")
            .join("v1")
            .join("附带的模组");
        std::fs::create_dir_all(&camp_dir).expect("建目录");
        std::fs::write(camp_dir.join("附带.SC2Mod"), b"y").expect("写");

        let index = serde_json::json!({
            "mods": [
                mod_record("lib1", "独立.SC2Mod", ModSource::Library),
                mod_record(
                    "campaign-wol-v1-abc",
                    "附带的模组",
                    ModSource::Campaign {
                        slot: "wol".to_string(),
                        variant: "v1".to_string(),
                        path: "附带的模组".to_string(),
                    },
                ),
            ]
        });
        std::fs::write(data.join("mods.json"), index.to_string()).expect("写索引");

        // wol 没有任何版本被启用（连 library.json 都没有）
        let report = sync(&data, &installation).expect("同步");

        assert!(
            report.placed.iter().any(|name| name.contains("独立")),
            "独立模组照铺：{:?}",
            report.placed
        );
        assert!(
            !report.placed.iter().any(|name| name.contains("附带")),
            "战役没启用，它带来的模组不该铺：{:?}",
            report.placed
        );
        assert!(
            !game.join("Mods").join("附带的模组").exists(),
            "游戏目录里也不该有"
        );
        assert!(
            report.warnings.iter().any(|line| line.contains("没铺")),
            "要有话说明白为什么没铺：{:?}",
            report.warnings
        );
    }
}
