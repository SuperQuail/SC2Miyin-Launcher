//! 铺盘：把**启用**的独立模组铺进 `<游戏>/Mods/`，顺手撤掉停用的。
//!
//! 走的是和战役同一条写盘路径（`crate::library::install::Manifest`）：
//! 白名单校验 / 备份还原 / 记账 / 回滚只有那一份实现。

use std::path::Path;

use crate::error::{Error, Result};
use crate::safety;

use super::import::single_child_file;
use super::index::{effective_id, list, mods_root, save};
use super::{ModKind, ModSource, StandaloneMod, SyncReport};

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
/// **走的是和战役同一条写盘路径**（`crate::library::install::Manifest`）——
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
    let index = crate::library::Library::new(data.to_path_buf()).index();
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

        // 已经铺好、文件也都在就跳过 —— 启动时的自动挂载不该每次把整个模组重拷一遍
        if manifest.settled(installation, &plan, "Mods/") {
            placed.push(name);
            continue;
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
