//! **统一安装引擎**：所有往游戏目录写东西的操作都走这里。
//!
//! 为什么要有这一层：原来战役和独立模组各有一套安装代码、各写一份清单
//! （`active.json` / `mods-active.json`），于是：
//!
//! - 两边会**互相打架**：战役铺了 `Mods/Alenger/`，独立模组也想铺同一个位置，
//!   谁后写谁赢，两份清单还各说各话
//! - **回滚对不上**：撤战役不知道独立模组放过什么，反之亦然
//! - 同一件事要维护两遍 —— 前两个 bug 就是两条路各错了一遍
//!
//! 现在统一成一条流水线：
//!
//! `@text
//! 谁要装什么  ->  攒一份 Plan  ->  Manifest::apply() 执行
//! `@
//!
//! 白名单校验、备份还原、记账、回滚加起来只有这一份实现。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;

/// 清单文件的格式版本。
const MANIFEST_VERSION: u32 = 1;

/// **谁**要往游戏目录里装东西。
///
/// 记账和回滚都按它来分 —— 「撤掉这个战役」不该连别人装的模组一起撤掉。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Owner {
    /// 某个战役的某个版本。
    Campaign { slot: String, variant: String },
    /// 一个独立模组。
    Mod { id: String },
}

impl Owner {
    /// 清单与备份目录里用的短标识。
    pub fn key(&self) -> String {
        match self {
            Self::Campaign { slot, variant } => format!("campaign-{slot}-{variant}"),
            Self::Mod { id } => format!("mod-{id}"),
        }
    }

    /// 界面上的说法。
    pub fn label(&self) -> String {
        match self {
            Self::Campaign { slot, .. } => format!("战役 {slot}"),
            Self::Mod { id } => format!("模组 {id}"),
        }
    }
}

/// 一条安装项：把 `source` 放到游戏目录的 `target`。
#[derive(Debug, Clone)]
pub struct Item {
    pub source: PathBuf,
    /// 相对**游戏根目录**的路径，用 `/` 分隔。
    pub target: String,
}

/// 一份安装计划。
#[derive(Debug, Clone)]
pub struct Plan {
    pub owner: Owner,
    pub items: Vec<Item>,
}

impl Plan {
    /// 建一份空计划。
    pub fn new(owner: Owner) -> Self {
        Self {
            owner,
            items: Vec::new(),
        }
    }

    /// 加一条（目标用 `/` 分隔的相对路径）。
    pub fn push(&mut self, source: impl Into<PathBuf>, target: impl Into<String>) {
        self.items.push(Item {
            source: source.into(),
            target: target.into(),
        });
    }

    /// 有没有内容。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// 清单里的一条记录：游戏目录里某个位置是我们放的。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installed {
    /// 相对游戏根目录，`/` 分隔。
    pub target: String,
    pub owner: Owner,
    /// 装之前那个位置原本有东西的话，原件备份在哪（相对 `data/backup`）。
    #[serde(default)]
    pub backup: Option<String>,
    /// 内容是从库里哪个文件/目录拷来的。
    ///
    /// 记它是为了能反查「库里这份东西装到游戏目录之后在哪」——
    /// 打开地图时必须用**游戏目录里那一份**，见 `Manifest::target_of`。
    #[serde(default)]
    pub source: Option<String>,
}

/// 安装清单：游戏目录里现在有哪些是我们放的。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    pub files: Vec<Installed>,
}

fn default_version() -> u32 {
    MANIFEST_VERSION
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            files: Vec::new(),
        }
    }
}

impl Manifest {
    /// 读清单。文件不在或读坏了都当空 —— 大不了下次重装，不能让界面起不来。
    pub fn load(data: &Path) -> Self {
        std::fs::read_to_string(Self::path(data))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// 清单文件在哪。
    pub fn path(data: &Path) -> PathBuf {
        data.join("installed.json")
    }

    /// 备份区在哪。
    pub fn backup_root(data: &Path) -> PathBuf {
        data.join("backup")
    }

    /// 写清单。
    pub fn save(&self, data: &Path) -> Result<()> {
        std::fs::create_dir_all(data)?;
        std::fs::write(Self::path(data), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// 某个 owner 现在装了哪些。
    pub fn of(&self, owner: &Owner) -> Vec<Installed> {
        self.files
            .iter()
            .filter(|item| &item.owner == owner)
            .cloned()
            .collect()
    }

    /// 把**老格式的清单**迁过来；已经是新格式就直接读。
    ///
    /// 老代码分成两份：战役的 `active.json`（单槽位、存绝对路径）、
    /// 独立模组的 `mods-active.json`（只存名字）。
    /// 不迁的话，游戏目录里那些我们放的文件就变成**没人认领的孤儿** ——
    /// 用户点「切回原版」也清理不掉，只能手动去游戏目录里翻。
    ///
    /// 迁移只做一次：写完 `installed.json` 就把老文件删掉。
    pub fn load_migrating(data: &Path, installation: &Installation) -> Self {
        let path = Self::path(data);
        if path.is_file() {
            return Self::load(data);
        }

        let legacy_campaign = data.join("active.json");
        let legacy_mods = data.join("mods-active.json");
        if !legacy_campaign.is_file() && !legacy_mods.is_file() {
            return Self::default();
        }

        let mut manifest = Self::default();
        let root = &installation.root;

        // 相对游戏根的路径；不在游戏目录里的（理论上不该有）跳过
        let relative_of = |absolute: &str| -> Option<String> {
            let path = PathBuf::from(absolute);
            path.strip_prefix(root)
                .ok()
                .map(|rest| rest.to_string_lossy().replace('\\', "/"))
        };

        // ---- 战役的 active.json ----
        if let Ok(text) = std::fs::read_to_string(&legacy_campaign)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        {
            let owner = Owner::Campaign {
                slot: value
                    .get("slot")
                    .and_then(|item| item.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                variant: value
                    .get("variant")
                    .and_then(|item| item.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            };

            // 备份：原位置 -> 备份位置
            let mut backups: Vec<(String, String)> = Vec::new();
            if let Some(list) = value.get("backups").and_then(|item| item.as_array()) {
                for item in list {
                    let original = item.get("original").and_then(|v| v.as_str());
                    let backup = item.get("backup").and_then(|v| v.as_str());
                    if let (Some(original), Some(backup)) = (original, backup)
                        && let Some(relative) = relative_of(original)
                        && let Ok(relative_backup) =
                            PathBuf::from(backup).strip_prefix(Self::backup_root(data))
                    {
                        backups.push((
                            relative,
                            relative_backup.to_string_lossy().replace('\\', "/"),
                        ));
                    }
                }
            }

            if let Some(list) = value.get("placed").and_then(|item| item.as_array()) {
                for item in list {
                    let Some(absolute) = item.get("path").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let Some(relative) = relative_of(absolute) else {
                        continue;
                    };
                    let backup = backups
                        .iter()
                        .find(|(original, _)| original == &relative)
                        .map(|(_, backup)| backup.clone());

                    manifest.files.push(Installed {
                        target: relative,
                        owner: owner.clone(),
                        backup,
                        // 老清单没记来源 —— 打开地图时会退回用库里那一份
                        source: None,
                    });
                }
            }
        }

        // ---- 独立模组的 mods-active.json ----
        if let Ok(text) = std::fs::read_to_string(&legacy_mods)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(list) = value.get("placed").and_then(|item| item.as_array())
        {
            for item in list {
                let Some(name) = item.as_str() else {
                    continue;
                };
                manifest.files.push(Installed {
                    target: format!("Mods/{name}"),
                    // 老格式没记是哪个模组，只能记成一个「历史遗留」
                    owner: Owner::Mod {
                        id: "legacy".to_string(),
                    },
                    backup: None,
                    // 老格式没记来源
                    source: None,
                });
            }
        }

        // 迁移完就把老文件收掉，免得下次又迁一遍
        let _ = std::fs::remove_file(&legacy_campaign);
        let _ = std::fs::remove_file(&legacy_mods);
        let _ = manifest.save(data);

        manifest
    }

    /// 库里这个路径的东西，装到游戏目录之后在哪 —— 没装就 `None`。
    ///
    /// **打开地图必须用这个**：库里那份只是留底（切回原版时能还原），
    /// 而游戏和编辑器要读的是装好的那一份。实测从库里打开会报「无法打开地图」——
    /// 库里那份不在游戏目录下，地图里 `GameSetNextMap("Starcraft Mass Recall/…")`
    /// 这种相对 `Maps/` 的串联路径也就落空了。
    pub fn target_of(&self, installation: &Installation, source: &Path) -> Option<PathBuf> {
        let wanted = source.to_string_lossy().to_string();
        let item = self
            .files
            .iter()
            .find(|item| item.source.as_deref() == Some(wanted.as_str()))?;

        resolve(installation, &item.target).ok()
    }

    /// 游戏目录里这个位置是谁装的。
    pub fn owner_of(&self, target: &str) -> Option<&Owner> {
        self.files
            .iter()
            .find(|item| item.target == target)
            .map(|item| &item.owner)
    }

    /// **应用一份计划**：先撤掉同一个 owner 的旧东西，再按计划装。
    ///
    /// 返回冲突提示（目标被别的 owner 占着）—— 不阻断，但要让用户知道
    /// 谁盖了谁。以前是静默覆盖，出了问题根本查不出来。
    pub fn apply(
        &mut self,
        data: &Path,
        installation: &Installation,
        plan: &Plan,
    ) -> Result<Vec<String>> {
        // 1) 这个 owner 之前装的先全撤掉 —— 保证「计划就是当前状态」
        self.remove(data, installation, &plan.owner)?;

        let mut warnings = Vec::new();

        for item in &plan.items {
            let target = resolve(installation, &item.target)?;

            // 2) 目标被**别人**占着：先撤别人的（连同它的账），并如实提示
            if let Some(other) = self.owner_of(&item.target).cloned()
                && other != plan.owner
            {
                warnings.push(format!(
                    "{} 也要用 {}，现在由{}占着 —— 已改由{}接管",
                    plan.owner.label(),
                    item.target,
                    other.label(),
                    plan.owner.label()
                ));
                self.remove(data, installation, &other)?;
            }

            // 3) 目标已存在（多半是官方文件，或用户自己放的）：先挪进备份区
            let backup = if target.exists() {
                let relative = format!("{}/{}", plan.owner.key(), item.target);
                let backup = safety::ensure_within(
                    &Self::backup_root(data),
                    &Self::backup_root(data).join(&relative),
                )?;
                if let Some(parent) = backup.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if backup.exists() {
                    let _ = std::fs::remove_dir_all(&backup);
                    let _ = std::fs::remove_file(&backup);
                }
                std::fs::rename(&target, &backup)?;
                Some(relative)
            } else {
                None
            };

            // 4) 铺进去
            copy_entry(&item.source, &target)?;

            // 5) 记账
            self.files.push(Installed {
                target: item.target.clone(),
                owner: plan.owner.clone(),
                backup,
                source: Some(item.source.to_string_lossy().to_string()),
            });
        }

        self.save(data)?;
        Ok(warnings)
    }

    /// **撤掉某个 owner 装的东西**：删掉我们放的，还原被挪走的官方文件。
    ///
    /// 只动清单里记过的文件 —— 绝不递归删官方目录。
    pub fn remove(
        &mut self,
        data: &Path,
        installation: &Installation,
        owner: &Owner,
    ) -> Result<()> {
        let leaving: Vec<Installed> = self.of(owner);
        if leaving.is_empty() {
            return Ok(());
        }

        for item in &leaving {
            let target = resolve(installation, &item.target)?;

            // 我们放的：删掉。目录先整删，免得留下空壳
            if target.is_dir() {
                let _ = std::fs::remove_dir_all(&target);
            } else if target.is_file() {
                let _ = std::fs::remove_file(&target);
            }

            // 原本有官方文件被挪走的：原样还原
            if let Some(relative) = &item.backup {
                let backup = Self::backup_root(data).join(relative);
                if backup.exists() {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::rename(&backup, &target)?;
                }
            }
        }

        self.files.retain(|item| &item.owner != owner);
        self.save(data)?;

        // 备份区里这个 owner 的目录空了就收掉
        let dir = Self::backup_root(data).join(owner.key());
        if dir.is_dir() {
            let _ = std::fs::remove_dir_all(&dir);
        }

        Ok(())
    }
}

/// 把相对路径解析成游戏目录下的绝对路径，并过白名单校验。
fn resolve(installation: &Installation, relative: &str) -> Result<PathBuf> {
    let path = installation
        .root
        .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    allowed_target(installation, &path)
}

/// 校验目标落在游戏目录的**内容区**之内。
///
/// 白名单是这三个根（与 AGENTS.md §10.5 一致）：
///
/// `@text
/// <游戏>/Maps/        游戏扫的所有地图 —— 官方战役、自制战役、以及
///                     作者按自己结构摆的整包（SCMR 就是 Maps/Starcraft Mass Recall/…）
/// <游戏>/Mods/        模组
/// <游戏>/Interfaces/  界面
/// `@
///
/// **别收窄成 Maps/Campaign 那种子目录** —— 收窄过，代价是复刻战役整包装不进去：
/// 地图被正确放到 `Maps/Starcraft Mass Recall/…`（启动器地图按这个相对 Maps/ 的
/// 路径联动关卡），却被自己的闸门拒绝，用户看到的是「拒绝往 … 写东西」。
/// 只要还在 `Maps/` 底下就不会污染别的地方，游戏也确实会去扫。
///
/// 这是写盘的最后一道闸门 —— 不依赖上游校验过没有。
fn allowed_target(installation: &Installation, path: &Path) -> Result<PathBuf> {
    safety::ensure_within(&installation.maps_root, path)
        .or_else(|_| safety::ensure_within(&installation.mods_root, path))
        .or_else(|_| safety::ensure_within(&installation.interfaces_root, path))
        .map_err(|_| {
            Error::PackageRejected(format!(
                "拒绝往 {} 写东西 —— 只允许写游戏目录下的 Maps、Mods 和 Interfaces",
                path.display()
            ))
        })
}

/// 拷一个文件或一棵目录树。
fn copy_entry(source: &Path, target: &Path) -> Result<()> {
    if source.is_dir() {
        std::fs::create_dir_all(target)?;
        for entry in walkdir::WalkDir::new(source)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let relative = entry
                .path()
                .strip_prefix(source)
                .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
            let dest = target.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dest)?;
            } else {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(entry.path(), &dest)?;
            }
        }
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, target)?;
    Ok(())
}
