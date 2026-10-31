//! 战役库：让**同一个战役的多个玩家版本共存**，并可自由切换。
//!
//! # 为什么要独立于游戏目录
//!
//! 游戏目录里的 `Maps/CustomCampaigns` 只能放「当前这一份」，不同玩家的改版会互相覆盖，
//! 装第二个版本就得先删掉第一个。所以库放在**启动器自己的数据目录**里：
//!
//! ```text
//! <启动器目录>/data/
//! ├── library.json      # 索引：每个槽位下有哪些版本、当前启用哪个
//! ├── installed.json    # 安装清单：游戏目录里哪些东西是我们放的、谁放的
//! ├── campaigns/
//! │   └── wol/
//! │       ├── 自由之翼：重生 v1.4.2/
//! │       └── 自由之翼：重生 v1.5.0/
//! └── backup/           # 被挪走的官方文件，切回原版时原样还原
//! ```
//!
//! # 槽位
//!
//! 「槽位」= 一个官方资料片（自由之翼 / 虫群之心 / 进化 / 虚空之遗 / 序章 / 诺娃）。
//! 每个槽位默认是**原版战役**（`active = None`），导入的版本作为可切换的选项。

use std::path::{Path, PathBuf};

use crate::campaign::metadata::CampaignType;
use crate::error::{Error, Result};
use crate::safety;
use crate::sc2::Installation;

pub mod activation;
pub mod compat;
pub mod compose;
pub mod install;
pub mod known;
pub mod mods;
pub mod naming;

pub use mods::{ModChanges, StandaloneMod};
pub mod export;
pub mod merge;
pub mod patch;
pub mod store;

mod conflict;
mod data_root;
mod main_map;
mod mod_keys;
mod model;
mod slot;

#[cfg(test)]
mod tests;

pub use activation::{ActiveConflict, activate, conflicts, deactivate, preview};
pub use conflict::{
    Conflict, ImportMode, VersionRelation, compare_versions, conflict_for, slot_for,
};
pub use data_root::{
    AppPaths, DATA_DIR_NAME, DataRootPlan, app_paths, data_root, prepare_app_paths,
    resolve_data_root,
};
pub use install::{Installed, Item, Manifest, Owner, Plan, Preview, PreviewEntry};
pub use main_map::resolve_main_map;
pub use mod_keys::{
    ModKey, effective_mounted_mods, group_mods, mod_identity, mod_key_of, normalize_mod_key,
};
pub use model::{
    Binding, DocInfo, LibraryIndex, LibraryMod, MainMapChoice, MapEntry, ModEntry, ModOrigin,
    Patch, Variant,
};
pub use slot::{CampaignSlot, SlotView};
pub use store::{
    Replaced, VariantChanges, find_target_for_package, find_target_for_package_with_password,
    import, import_with_password, remove_variant, replace_variant, replace_variant_with_password,
    update_variant,
};

/// 原子写：先写同目录的临时文件，再改名覆盖。
///
/// 直接 fs::write 在断电 / 被强杀 / 杀软截断时会留下半个 JSON，
/// 而那个半个文件下次会被当成**空**索引读进来 —— 整个库就在界面上消失了。
pub fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    // Windows 上 rename 会覆盖已存在的目标（MoveFileEx 语义）
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// 战役库。
#[derive(Debug, Clone)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    /// 指定库根目录。默认位置见 [`data_root`]。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 库根目录。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 索引文件。
    pub fn index_path(&self) -> PathBuf {
        self.root.join("library.json")
    }

    /// 各版本的实际存放目录。
    pub fn campaigns_dir(&self) -> PathBuf {
        self.root.join("campaigns")
    }

    /// 补丁目录：`<库根>/patches`。
    pub fn patches_dir(&self) -> PathBuf {
        self.root.join("patches")
    }

    /// 某个补丁的内容目录。
    pub fn patch_dir(&self, id: &str) -> PathBuf {
        self.patches_dir().join(id)
    }

    /// 被挪走的官方文件的暂存目录。
    pub fn backup_dir(&self) -> PathBuf {
        self.root.join("backup")
    }

    /// 某个槽位的存放目录。
    pub fn slot_dir(&self, slug: &str) -> PathBuf {
        self.campaigns_dir().join(slug)
    }

    /// 读取索引。
    ///
    /// 文件不存在时返回空索引 —— 还没有库是很正常的事，不该报错。
    /// 但**读坏了要留证据**：直接当空索引的话，紧接着的一次保存
    /// 就会把整个库顶成 {}，用户看到的是一夜之间什么都没了。
    pub fn index(&self) -> LibraryIndex {
        let path = self.index_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return LibraryIndex::default();
        };
        match serde_json::from_str(&text) {
            Ok(index) => index,
            Err(_) => {
                let _ = std::fs::rename(&path, path.with_extension("json.corrupt"));
                LibraryIndex::default()
            }
        }
    }

    /// 写回索引。
    pub fn save_index(&self, index: &LibraryIndex) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let text = serde_json::to_string_pretty(index)
            .map_err(|error| Error::Parse(format!("索引序列化失败：{error}")))?;
        write_atomic(&self.index_path(), &text)
    }

    /// 按**官方发布顺序**生成槽位快照。
    pub fn slots(&self, installation: Option<&Installation>) -> Vec<SlotView> {
        let index = self.index();
        // 「装了没」看激活清单，不看槽位的 `active`（那只记最后选中的那个）
        let installed = installation
            .map(|installation| self::activation::enabled_variants(self, installation))
            .unwrap_or_default();

        // 主菜单列五个条目：四大原版战役（进化归虫群之心、序章归虚空之遗）+ 自制战役
        CampaignType::MENU
            .iter()
            .map(|kind| {
                let slug = kind.slug();
                let slot = index.slots.get(slug).cloned().unwrap_or_default();
                let active_name = slot
                    .active
                    .as_ref()
                    .and_then(|id| slot.variants.iter().find(|variant| &variant.id == id))
                    .map(|variant| variant.name.clone());

                let has_content = !slot.variants.is_empty() || slot.active.is_some();
                let notice = if has_content {
                    installation.and_then(|installation| notice_for(installation, kind))
                } else {
                    None
                };

                let enabled_variants: Vec<String> = installed
                    .iter()
                    .filter(|(installed_slot, _)| installed_slot == slug)
                    .map(|(_, variant)| variant.clone())
                    .collect();

                SlotView {
                    slug: slug.to_string(),
                    display_name: kind.display_name(),
                    sub_directory: kind.sub_directory().map(str::to_string),
                    variants: slot.variants,
                    active: slot.active,
                    active_name,
                    vanilla_enabled: enabled_variants.is_empty(),
                    enabled_variants,
                    notice,
                }
            })
            .collect()
    }

    /// 列出某个版本里的模组，并标出各自挂没挂载。
    ///
    /// **按文件夹去重**：一个模组可能由十几个 `.SC2Mod` 组成，
    /// 界面上一行就够了（见 `mod_identity`）。
    pub fn variant_mods(&self, slot_slug: &str, variant_id: &str) -> Vec<ModEntry> {
        let Some(variant) = self.variant(slot_slug, variant_id) else {
            return Vec::new();
        };

        // `mounted` 是**用户配置**（勾选框状态），不是「铺进游戏目录了没」。
        // 以前这里按 `slot.active` 门控：没启用的版本一律报未挂载，于是
        // 1) 用户勾了模组，下次读回来又变未勾；
        // 2) 启动前的挂载检查把「配置里全挂着」误判成「一个都没挂」，
        //    把自制战役的启动直接拦住（issue 反馈）。
        // 实际铺盘状态看 `in_game`。
        let mounted = effective_mounted_mods(&variant);
        let in_game = self.installed_mod_keys(slot_slug, variant_id);
        group_mods(&variant.payloads, &mounted, &in_game)
    }

    /// 这个版本的模组里，**真的铺进 `<游戏>/Mods/`** 的那些（按安装清单算）。
    ///
    /// shortcut: 只信清单、不查盘上文件是否还在 —— 用户手删 `Mods/` 里的文件后这里仍报
    /// 已铺，等 `activation::sync_mods`（启动 / 启动游戏时）补回来。真要逐文件校验时再升级。
    fn installed_mod_keys(&self, slot_slug: &str, variant_id: &str) -> Vec<String> {
        let owner = install::Owner::Campaign {
            slot: slot_slug.to_string(),
            variant: variant_id.to_string(),
        };
        let manifest = install::Manifest::load(self.root());
        let mut keys: Vec<String> = manifest
            .files
            .iter()
            .filter(|item| item.owner == owner)
            .filter_map(|item| mod_key_of(&item.target).map(|found| found.key))
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// **全库的模组汇总**：模组管理菜单用。
    ///
    /// 每一行都带着它属于谁 —— 用户看的是「自由之翼 · 重生 v1.4 的 Alenger」，
    /// 而不是一堆孤零零的文件名。
    pub fn all_mods(&self) -> Vec<LibraryMod> {
        let index = self.index();
        let mut rows = Vec::new();

        for (slug, slot) in &index.slots {
            let slot_name = CampaignType::from_slug(slug)
                .map(|kind| kind.display_name())
                .unwrap_or_else(|| slug.clone());
            let is_custom = CampaignType::from_slug(slug).is_some_and(|kind| kind.is_custom());

            for variant in &slot.variants {
                // `mounted` 是用户配置、`in_game` 才是实际铺盘（同 variant_mods）。
                let mounted_list = effective_mounted_mods(variant);
                let in_game = self.installed_mod_keys(slug, &variant.id);
                let declared = &variant.declared_mods;

                for entry in group_mods(&variant.payloads, &mounted_list, &in_game) {
                    // 有模组记录的话**以记录为准** —— 用户可能改过名字和版本，
                    // 改的是记录（覆盖包内声明），不动包本身。
                    let record = crate::library::mods::find_campaign(
                        self.root(),
                        slug,
                        &variant.id,
                        &entry.path,
                    );

                    rows.push(LibraryMod {
                        slot: slug.clone(),
                        slot_name: slot_name.clone(),
                        variant_id: variant.id.clone(),
                        variant_name: variant.name.clone(),
                        origin: if is_custom {
                            ModOrigin::CustomCampaign
                        } else {
                            ModOrigin::OfficialCampaign
                        },
                        required: declared.iter().any(|item| item == &entry.path),
                        path: entry.path,
                        name: record
                            .as_ref()
                            .map(|item| item.name.clone())
                            .unwrap_or(entry.name),
                        mounted: entry.mounted,
                        in_game: entry.in_game,
                        parts: entry.parts,
                        standalone_id: record.as_ref().map(|item| item.id.clone()),
                        modid: None,
                        version: record
                            .as_ref()
                            .and_then(|item| item.version.clone())
                            .or_else(|| variant.version.clone()),
                        folder: None,
                        kind: None,
                        mod_record_id: record.map(|item| item.id),
                        source_kind: "campaign".to_string(),
                    });
                }
            }
        }

        // 稳定顺序：先按战役，再按版本，最后按名字
        rows.sort_by(|left, right| {
            left.slot_name
                .cmp(&right.slot_name)
                .then_with(|| left.variant_name.cmp(&right.variant_name))
                .then_with(|| left.name.cmp(&right.name))
        });
        rows
    }

    /// 改某个版本的挂载模组清单。
    ///
    /// 传进来的键会被**过滤成这个版本里真实存在的模组** —— 免得界面上传来一个
    /// 手改的路径，白白在激活时失败。
    pub fn set_mounted_mods(
        &self,
        slot_slug: &str,
        variant_id: &str,
        mods: &[String],
    ) -> Result<Variant> {
        let mut index = self.index();
        let slot = index
            .slots
            .get_mut(slot_slug)
            .ok_or_else(|| Error::CampaignNotFound(slot_slug.to_string()))?;
        let variant = slot
            .variants
            .iter_mut()
            .find(|item| item.id == variant_id)
            .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

        let known: Vec<String> = variant
            .payloads
            .iter()
            .filter_map(|payload| mod_identity(payload).map(|found| found.key))
            .collect();

        // 写 Some 而不是空 Vec：这样「一个都不挂」才是用户的意思，
        // 而不是"没配过"
        variant.mounted_mods = Some(
            mods.iter()
                .filter(|wanted| known.contains(wanted))
                .cloned()
                .collect(),
        );

        let updated = variant.clone();
        self.save_index(&index)?;
        Ok(updated)
    }

    /// 改某个版本的主地图；传 `None` 表示清空。
    pub fn set_main_map(
        &self,
        slot_slug: &str,
        variant_id: &str,
        map: Option<&str>,
    ) -> Result<Variant> {
        let mut index = self.index();
        let slot = index
            .slots
            .get_mut(slot_slug)
            .ok_or_else(|| Error::CampaignNotFound(slot_slug.to_string()))?;
        let variant = slot
            .variants
            .iter_mut()
            .find(|item| item.id == variant_id)
            .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

        variant.main_map = map
            .map(|value| value.replace('\\', "/"))
            .filter(|value| !value.trim().is_empty());

        let updated = variant.clone();
        self.save_index(&index)?;
        Ok(updated)
    }

    /// 取一个版本。
    pub fn variant(&self, slot_slug: &str, variant_id: &str) -> Option<Variant> {
        self.index()
            .slots
            .get(slot_slug)?
            .variants
            .iter()
            .find(|item| item.id == variant_id)
            .cloned()
    }

    /// 取某个版本自带的说明文档信息；没有就是 `None`。
    pub fn variant_doc(&self, slot_slug: &str, variant_id: &str) -> Option<DocInfo> {
        let variant = self.variant(slot_slug, variant_id)?;
        let relative = variant.doc?;
        let path = self.slot_dir(slot_slug).join(variant_id).join(&relative);
        if !path.is_file() {
            return None;
        }
        let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| relative.clone());

        Some(DocInfo {
            path: relative,
            name,
            size,
        })
    }

    /// 读出说明文档的字节；路径越界或文件不在都报错。
    ///
    /// 界面拿它交给 PDF 渲染器 —— 走 IPC 传字节而不是让 WebView 去读文件，
    /// 好处是不用放开文件系统访问，也省掉 asset 协议的配置。
    pub fn doc_bytes(&self, slot_slug: &str, variant_id: &str) -> Result<Vec<u8>> {
        let variant = self
            .variant(slot_slug, variant_id)
            .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;
        let relative = variant
            .doc
            .ok_or_else(|| Error::PackageRejected("这个版本没有自带的说明文档".to_string()))?;

        let root = self.slot_dir(slot_slug).join(variant_id);
        let path = safety::ensure_within(&root, &root.join(relative.replace('\\', "/")))?;
        if !path.is_file() {
            return Err(Error::PackageRejected(
                "说明文档在库里的文件已经不在了".to_string(),
            ));
        }
        Ok(std::fs::read(&path)?)
    }

    /// 某张地图在库里的绝对路径；越界或不存在都报错。
    ///
    /// 编辑器启动要用它 —— 自制战役的地图不进游戏目录，得直接把库里的路径递给编辑器。
    pub fn map_path(&self, slot_slug: &str, variant_id: &str, map: &str) -> Result<PathBuf> {
        let root = self.slot_dir(slot_slug).join(variant_id);
        let path = safety::ensure_within(&root, &root.join(map.replace('\\', "/")))?;
        if !path.is_file() {
            return Err(Error::PackageRejected(format!("找不到地图文件：{map}")));
        }
        Ok(path)
    }

    /// 列出某个版本里的所有地图。
    ///
    /// 按**相对路径的自然顺序**排 —— 章节 `1. Rebel Yell` 排在 `2. Overmind` 前面，
    /// `Terran2` 排在 `Terran10` 前面（普通字符串排序会把 10 排到 2 前面）。
    pub fn variant_maps(&self, slot_slug: &str, variant_id: &str) -> Vec<MapEntry> {
        let root = self.slot_dir(slot_slug).join(variant_id);
        if !root.is_dir() {
            return Vec::new();
        }

        let declared = self
            .index()
            .slots
            .get(slot_slug)
            .and_then(|slot| slot.variants.iter().find(|item| item.id == variant_id))
            .and_then(|variant| variant.main_map.clone());

        let mut maps: Vec<MapEntry> = walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sc2map"))
            })
            .filter_map(|entry| {
                let relative = entry.path().strip_prefix(&root).ok()?;
                // 统一用 / 分隔，跨平台且与包内写法一致
                let path = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                let name = entry
                    .path()
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                let chapter = path.split_once('/').map(|(folder, _)| folder.to_string());
                let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);

                Some(MapEntry {
                    path,
                    name,
                    chapter,
                    size,
                    is_main: false,
                })
            })
            .collect();

        maps.sort_by(|left, right| natural_cmp(&left.path, &right.path));

        if let Some(main) = declared {
            for map in &mut maps {
                if map.path.eq_ignore_ascii_case(&main) {
                    map.is_main = true;
                }
            }
        }

        maps
    }
}

/// 校验槽位标识，返回对应的资料片。
///
/// **自制战役（`custom`）也是合法槽位** —— 以前这里多加了 `is_main()` 过滤，
/// 结果导入自制战役会直接报「未知的战役槽位：custom」。
/// 落盘位置本来就由 `Placement` 分开管（官方改版进 `Maps/Campaign`，
/// 自制战役进 `Maps/CustomCampaigns`），校验这一层不该再判断它属于哪部原版战役。
pub fn require_slot(slug: &str) -> Result<CampaignType> {
    CampaignType::from_slug(slug)
        .ok_or_else(|| Error::PackageRejected(format!("未知的战役槽位：{slug}")))
}

/// 槽位与当前游戏安装之间的提示。
fn notice_for(installation: &Installation, kind: &CampaignType) -> Option<String> {
    let target = match kind.sub_directory() {
        Some(sub) => installation.campaign_maps_root.join(sub),
        None => installation.campaign_maps_root.clone(),
    };

    if target.is_dir() {
        None
    } else {
        Some(format!(
            "官方目录尚不存在，启用时会自动创建：{}",
            target.display()
        ))
    }
}

/// 生成一个进程内唯一的后缀。
pub(crate) fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

/// 当前 Unix 秒。
pub(crate) fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

/// 自然顺序比较：把连续数字当数值比，而不是按字符比。
///
/// 否则 `Terran10` 会排到 `Terran2` 前面。
fn natural_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let mut a = left.chars().peekable();
    let mut b = right.chars().peekable();

    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.is_ascii_digit() && y.is_ascii_digit() {
                    let mut num_a = String::new();
                    while a.peek().is_some_and(char::is_ascii_digit) {
                        num_a.push(a.next().unwrap_or_default());
                    }
                    let mut num_b = String::new();
                    while b.peek().is_some_and(char::is_ascii_digit) {
                        num_b.push(b.next().unwrap_or_default());
                    }
                    let va: u64 = num_a.parse().unwrap_or(0);
                    let vb: u64 = num_b.parse().unwrap_or(0);
                    match va.cmp(&vb) {
                        std::cmp::Ordering::Equal => {}
                        other => return other,
                    }
                } else {
                    a.next();
                    b.next();
                    match x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase()) {
                        std::cmp::Ordering::Equal => {}
                        other => return other,
                    }
                }
            }
        }
    }
}
