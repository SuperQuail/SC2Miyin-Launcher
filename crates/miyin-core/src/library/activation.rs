//! 启用 / 停用战役：把某个版本的内容铺进游戏目录，或者切回原版。
//!
//! **安装本身不在这里做** —— 白名单校验、备份还原、记账、回滚全在
//! `super::install` 那个统一引擎里。这里只负责一件事：
//! 把「这个版本该有哪些文件、各落到哪」算出来，攒成一份 `Plan` 交出去。
//!
//! 这样战役和独立模组走的是**同一条写盘路径**，不会再出现
//! 「两边各写一份清单、互相不知道对方放了什么」那种事。

use crate::error::{Error, Result};
use crate::sc2::Installation;

use super::compose::{self};
use super::install::{Manifest, Owner, Plan};
use super::{Library, require_slot};

/// 撤下**所有战役**装的东西（独立模组不动）。
///
/// 按槽位切回原版请用 `activate(library, installation, slot, None)` ——
/// 那个只撤这一个槽位的。
pub fn deactivate(library: &Library, installation: &Installation) -> Result<Vec<String>> {
    // 用迁移版：老清单（active.json）里的东西也要认得出来，
    // 否则那些文件会变成没人认领的孤儿
    let mut manifest = Manifest::load_migrating(library.root(), installation);

    let owners: Vec<Owner> = manifest
        .files
        .iter()
        .map(|item| item.owner.clone())
        .filter(|owner| matches!(owner, Owner::Campaign { .. }))
        .collect();

    for owner in owners {
        manifest.remove(library.root(), installation, &owner)?;
    }

    Ok(Vec::new())
}

/// 启用某个版本；`variant_id` 传 `None` 表示**切回原版**。
///
/// 只影响这一个槽位 —— 别的槽位、以及独立模组装的东西都不动。
pub fn activate(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    variant_id: Option<&str>,
) -> Result<Vec<String>> {
    let kind = require_slot(slot_slug)?;
    let mut index = library.index();
    // 用迁移版：老清单（active.json）里的东西也要认得出来，
    // 否则那些文件会变成没人认领的孤儿
    let mut manifest = Manifest::load_migrating(library.root(), installation);

    // 先撤下**这个槽位**原来启用的版本。
    // 只撤这个槽位的：别的战役和独立模组是别人的账，不该跟着一起没。
    if let Some(previous) = index
        .slots
        .get(slot_slug)
        .and_then(|slot| slot.active.clone())
    {
        manifest.remove(
            library.root(),
            installation,
            &Owner::Campaign {
                slot: slot_slug.to_string(),
                variant: previous,
            },
        )?;
    }

    // 切回原版：上面已经撤干净了，把启用状态清掉就完事
    let Some(variant_id) = variant_id else {
        if let Some(slot) = index.slots.get_mut(slot_slug) {
            slot.active = None;
        }
        library.save_index(&index)?;
        return Ok(Vec::new());
    };

    let slot = index.slots.get(slot_slug).cloned().unwrap_or_default();
    let variant = slot
        .variants
        .iter()
        .find(|variant| variant.id == variant_id)
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

    let variant_dir = library.slot_dir(slot_slug).join(&variant.id);
    if !variant_dir.is_dir() {
        return Err(Error::CampaignNotFound(variant.id.clone()));
    }

    let plan = plan_for(library, slot_slug, &variant, &kind)?;

    let warnings = manifest.apply(library.root(), installation, &plan)?;

    if let Some(slot) = index.slots.get_mut(slot_slug) {
        slot.active = Some(variant.id.clone());
    }
    library.save_index(&index)?;

    Ok(warnings)
}

/// 攒出「启用这个版本」要铺的那份计划。`activate` 与 `preview` 共用同一份 ——
/// 两处各写一遍的话，预演说"会覆盖 A"，实际却动了 B，那种错最要命。
fn plan_for(
    library: &Library,
    slot_slug: &str,
    variant: &super::Variant,
    kind: &crate::campaign::CampaignType,
) -> Result<Plan> {
    // 地图进 Maps/Campaign[/子目录]，模组进 Mods，自制战役的地图留在库里。
    // 目标子目录以**版本自己声明的**为准（进化包 -> swarm/evolution），槽位只作兜底。
    let sub = variant
        .target_sub
        .clone()
        .or_else(|| kind.sub_directory().map(str::to_string));

    // 分层合成：战役本体 + 这个战役上启用的补丁（按优先级叠加）
    let composition = compose::compose(library, slot_slug, variant, sub.as_deref());
    if composition.files.is_empty() {
        return Err(Error::PackageRejected(
            "该版本里没有可用的地图或模组".to_string(),
        ));
    }

    // 算好的合成结果 -> 一份安装计划
    let mut plan = Plan::new(Owner::Campaign {
        slot: slot_slug.to_string(),
        variant: variant.id.clone(),
    });
    for item in &composition.files {
        plan.push(item.source.clone(), item.target.clone());
    }

    Ok(plan)
}

/// **预演**：不写盘，只说清楚「启用这个版本会把游戏目录改成什么样」。
///
/// 这是「落点放开到整个游戏目录」之后的安全网 —— 用户先看见要动哪些文件
/// （新增 / 覆盖 / 接管 / 删除，各自多大），再决定按不按。
pub fn preview(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    variant_id: &str,
) -> Result<super::Preview> {
    let kind = require_slot(slot_slug)?;
    let index = library.index();
    let variant = index
        .slots
        .get(slot_slug)
        .and_then(|slot| slot.variants.iter().find(|item| item.id == variant_id))
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

    let plan = plan_for(library, slot_slug, &variant, &kind)?;
    // 用迁移版：老清单里的东西也要认得出来，否则预演会漏报"接管"
    let manifest = Manifest::load_migrating(library.root(), installation);
    manifest.preview(installation, &plan)
}
