//! 启用 / 停用战役：把某个版本的内容铺进游戏目录，或者切回原版。
//!
//! **安装本身不在这里做** —— 白名单校验、备份还原、记账、回滚全在
//! `super::install` 那个统一引擎里。这里只负责一件事：
//! 把「这个版本该有哪些文件、各落到哪」算出来，攒成一份 `Plan` 交出去。
//!
//! 这样战役和独立模组走的是**同一条写盘路径**，不会再出现
//! 「两边各写一份清单、互相不知道对方放了什么」那种事。

use serde::Serialize;

use crate::error::{Error, Result};
use crate::sc2::Installation;

use super::compose::{self};
use super::install::{Manifest, Owner, Plan};
use super::{Library, require_slot};

/// 撤下**所有战役**装的东西（独立模组不动），索引里的「启用」与存档组一并回原版。
///
/// 按槽位切回原版请用 `activate(library, installation, slot, None)` ——
/// 那个只撤这一个槽位的。
///
/// **按安装清单撤，不看 `index.slots[..].active`**：后者只记「最后选中的那个」，
/// 崩溃 / 手改 / 历史上多装的那几部都不在里面，只按它撤会留下永远没人认领的幽灵文件。
pub fn deactivate(library: &Library, installation: &Installation) -> Result<Vec<String>> {
    // 用迁移版：老清单（active.json）里的东西也要认得出来，
    // 否则那些文件会变成没人认领的孤儿
    let mut manifest = Manifest::load_migrating(library.root(), installation);
    let mut warnings = Vec::new();

    let slots = campaign_slots(&manifest);
    for slot in &slots {
        clear_slot(&mut manifest, library, installation, slot)?;
    }

    // 索引里的「启用」也跟着清 —— 不清的话界面还写着「已启用」，游戏目录已经空了。
    let mut index = library.index();
    for entry in index.slots.values_mut() {
        entry.active = None;
    }
    library.save_index(&index)?;

    // 存档组回各自的「原版组」
    for slot in &slots {
        switch_saves(library, installation, slot, None, &mut warnings);
    }

    Ok(warnings)
}

/// 清单里出现过的战役槽位（去重，保持清单里的先后顺序）。
fn campaign_slots(manifest: &Manifest) -> Vec<String> {
    let mut slots: Vec<String> = Vec::new();
    for item in &manifest.files {
        if let Owner::Campaign { slot, .. } = &item.owner
            && !slots.contains(slot)
        {
            slots.push(slot.clone());
        }
    }
    slots
}

/// 撤掉某个槽位里**所有**装着的东西（按清单）。
///
/// `activate` 换版本时先调它。以前只撤 `index.slots[..].active` 那一个 ——
/// 历史上多装的那几部、以及上次半途失败留下来的，都会永远躺在游戏目录里。
fn clear_slot(
    manifest: &mut Manifest,
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
) -> Result<()> {
    let mut owners: Vec<Owner> = Vec::new();
    for item in &manifest.files {
        if let Owner::Campaign { slot, .. } = &item.owner
            && slot == slot_slug
            && !owners.contains(&item.owner)
        {
            owners.push(item.owner.clone());
        }
    }
    for owner in owners {
        manifest.remove(library.root(), installation, &owner)?;
    }
    Ok(())
}

/// 让存档组跟着战役走：切到 `variant_id` 对应的那一组（`None` = 这个槽位的原版组）。
///
/// 目标不是「一个 Banks 目录」：游戏读的是 `Accounts/<账号>/<地域>/Banks`，
/// 可能有不止一个账号 —— 用 [`crate::saves::SavesRoots::discover`] 把它们**一起**切。
///
/// **失败只记警告**：存档组是用户数据，游戏文件已经铺好了 —— 为了切组去回滚文件是本末倒置。
/// 隔离开着才会真的切（`saves::ensure_for_slot_paths` 内部判断）。
fn switch_saves(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    variant_id: Option<&str>,
    warnings: &mut Vec<String>,
) {
    let Some(documents) = installation.documents_root.as_deref() else {
        // 这台机器上还没有「我的文档 / StarCraft II」，没什么可切的
        return;
    };
    let roots = crate::saves::SavesRoots::discover(documents);
    let key = crate::saves::isolation_key(library, slot_slug, variant_id);
    let label = crate::saves::group_label(library, &key);
    if let Err(error) = crate::saves::ensure_for_slot_paths(library.root(), &roots, &key, &label) {
        warnings.push(format!("存档组没能跟着切到「{label}」：{error}"));
    }
}

/// 启用某个版本；`variant_id` 传 `None` 表示**切回原版**。
///
/// 只影响这一个槽位 —— 别的槽位、以及独立模组装的东西都不动。
///
/// 三件事一次做完（用户点一下「启用」，这三件必须一起成立）：
/// 1. 按**安装清单**撤掉这个槽位**所有**装着的东西（不是只看 `slot.active`）；
/// 2. 铺新版本；**失败就回滚**成原版，不留半装状态；
/// 3. 存档组跟着切（隔离开着才有动作；切组失败只记警告，不回滚游戏文件）。
///
/// 先算计划再撤旧版本：新包本身有问题（比如没有可用载荷）时，不该先把旧版本卸了才发现装不上。
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
    let mut warnings = Vec::new();

    // **切回原版**：把这一槽位装着的东西按清单全撤掉，启用状态清掉，存档组回原版。
    let Some(variant_id) = variant_id else {
        clear_slot(&mut manifest, library, installation, slot_slug)?;
        if let Some(slot) = index.slots.get_mut(slot_slug) {
            slot.active = None;
        }
        library.save_index(&index)?;
        switch_saves(library, installation, slot_slug, None, &mut warnings);
        return Ok(warnings);
    };

    // 先把计划算出来（这一步只读）：包有问题就在动盘之前失败
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

    // 换版本：把这个槽位之前装着的**全部**撤下（清单里可能不止一个 owner）
    clear_slot(&mut manifest, library, installation, slot_slug)?;

    // 铺新版本；失败回滚。
    //
    // 回滚的终点是「原版」—— 旧版本在上一步已经撤下了，这一点要在错误信息里说清楚，
    // 否则用户会以为「安装失败 = 什么都没变」。
    match manifest.apply(library.root(), installation, &plan) {
        Ok(found) => warnings.extend(found),
        Err(error) => {
            let rollback = manifest.remove(library.root(), installation, &plan.owner);
            if let Some(slot) = index.slots.get_mut(slot_slug) {
                slot.active = None;
            }
            // 这里保存索引失败也不该盖住真正的失败原因，忽略即可
            let _ = library.save_index(&index);
            // 游戏目录回了原版，**存档组也要跟着回去**：只回文件不回组的话，游戏里是原版、
            // 启动器却还停在上次那部战役的组上，下一次切换会把两份进度搅在一起。
            switch_saves(library, installation, slot_slug, None, &mut warnings);
            let suffix = match rollback {
                Ok(()) => String::new(),
                Err(again) => {
                    format!("（回滚时又出错：{again}；游戏目录里可能有个别文件没清干净）")
                }
            };
            return Err(activation_failure(&variant.name, &error, &suffix));
        }
    }

    if let Some(slot) = index.slots.get_mut(slot_slug) {
        slot.active = Some(variant.id.clone());
    }
    library.save_index(&index)?;

    // 游戏文件已经好了，存档组跟着走
    switch_saves(
        library,
        installation,
        slot_slug,
        Some(&variant.id),
        &mut warnings,
    );

    Ok(warnings)
}

/// 撤掉**一个** owner 已经铺在游戏目录里的东西（独立模组也行）。
///
/// 和 `activate` 的区别：只撤指定的那一份，不动同槽位其它版本 ——
/// 「删除某个版本」用它：删之前先把它自己撤下来，别把同槽位别的版本一起卸掉。
///
/// 撤完如果索引里的「启用」正指着它、或这个槽位已经什么都不剩，就一并清掉，
/// 存档组回原版（否则删掉之后存档还留在那一版的组里，下次玩会串档）。
pub fn remove_owner(
    library: &Library,
    installation: &Installation,
    owner: &Owner,
) -> Result<Vec<String>> {
    let mut manifest = Manifest::load_migrating(library.root(), installation);
    manifest.remove(library.root(), installation, owner)?;

    // 独立模组：存档组与槽位的「启用」都不归它管
    let Owner::Campaign { slot, variant } = owner else {
        return Ok(Vec::new());
    };

    let slot_still_installed = manifest
        .files
        .iter()
        .any(|item| matches!(&item.owner, Owner::Campaign { slot: other, .. } if other == slot));
    let points_at_removed = index_active(library, slot).as_deref() == Some(variant.as_str());
    if !points_at_removed && slot_still_installed {
        // 槽位里还装着别的版本，而且「启用」也不是刚删的这个 —— 别动它
        return Ok(Vec::new());
    }

    let mut index = library.index();
    if let Some(entry) = index.slots.get_mut(slot) {
        entry.active = None;
    }
    library.save_index(&index)?;

    let mut warnings = Vec::new();
    switch_saves(library, installation, slot, None, &mut warnings);
    Ok(warnings)
}

/// 索引里某个槽位当前选中的版本（只读，给内部判断用）。
fn index_active(library: &Library, slot_slug: &str) -> Option<String> {
    library
        .index()
        .slots
        .get(slot_slug)
        .and_then(|entry| entry.active.clone())
}

/// 现在**装在游戏目录里**的战役版本（`(槽位, 版本 id)`），按激活清单 `active.json` 算。
///
/// **别拿 `index.slots[..].active` 当「装了没」**：它只记得最后选中的那一个。
/// 自制战役可以同时装好几部 —— 清单里有几条就是几部在游戏目录里。
pub fn enabled_variants(library: &Library, installation: &Installation) -> Vec<(String, String)> {
    let manifest = Manifest::load_migrating(library.root(), installation);
    let mut out: Vec<(String, String)> = Vec::new();
    for file in &manifest.files {
        let Owner::Campaign { slot, variant } = &file.owner else {
            continue;
        };
        let row = (slot.clone(), variant.clone());
        if !out.contains(&row) {
            out.push(row);
        }
    }
    out
}

/// **完整性检查**：上次装上 / 启用的版本，文件是不是还在游戏目录里。
///
/// 启动游戏前问一句。返回空 = 没问题；否则每条是**给人看的一句话**（哪一版缺几个文件，
/// 该去哪修）。只读，一个字节都不写。
///
/// 为什么要它：游戏目录被清过 / 重装过游戏 / 上次铺盘半途失败之后，索引与界面都还写着
/// 「已启用」，游戏却直接退到主菜单 —— 用户看到的是「点启动没反应」。自动挂载只修
/// `Mods/`（见 [`sync_mods`]），地图落点不在它的范围里，所以这里补一次**检测**。
pub fn incomplete_installation(library: &Library, installation: &Installation) -> Vec<String> {
    let index = library.index();

    // 要查的版本：**清单里装着的每一部**（真值），再加**槽位当前选中的那版**兜底
    // （清单被清掉、但索引还记得上次启用了什么 —— 这正是「游戏重装过」的样子）。
    let mut wanted = enabled_variants(library, installation);
    for (slot, entry) in &index.slots {
        let Some(variant) = &entry.active else {
            continue;
        };
        if wanted
            .iter()
            .any(|(wanted_slot, wanted_variant)| wanted_slot == slot && wanted_variant == variant)
        {
            continue;
        }
        wanted.push((slot.clone(), variant.clone()));
    }

    let mut out = Vec::new();
    for (slot, variant_id) in wanted {
        let Ok(kind) = require_slot(&slot) else {
            continue;
        };
        let Some(variant) = index
            .slots
            .get(&slot)
            .and_then(|entry| entry.variants.iter().find(|item| item.id == variant_id))
            .cloned()
        else {
            continue;
        };
        // 库里的内容都没了：不是「不完整」，是这一版整个没了 —— 交给启用那条路的报错
        if !library.slot_dir(&slot).join(&variant.id).is_dir() {
            continue;
        }
        let Ok(plan) = plan_for(library, &slot, &variant, &kind) else {
            continue;
        };
        let missing = plan
            .items
            .iter()
            .filter(|item| {
                !installation
                    .root
                    .join(item.target.replace('/', std::path::MAIN_SEPARATOR_STR))
                    .exists()
            })
            .count();
        if missing == 0 {
            continue;
        }
        out.push(format!(
            "「{}」在游戏目录里的文件不完整（缺 {missing} 个），请到战役页重新启用",
            variant.name
        ));
    }

    out
}
/// **启动时自动挂载**：把已启用版本的模组铺回 `<游戏>/Mods/`。
///
/// 模组挂载由启动器自己管 —— 用户不该为了「重装过 / `Mods` 被删过 / 上次没铺完」
/// 再去点一次启用。返回的是提示（冲突、失败），不阻断启动。
///
/// 幂等：清单已经一致且文件都在就跳过，不重拷。只动 `Mods/` ——
/// 地图落点与补丁归 `activate` / 补丁那套管，这里一概不碰。
pub fn sync_mods(library: &Library, installation: &Installation) -> Result<Vec<String>> {
    let index = library.index();
    let mut manifest = Manifest::load_migrating(library.root(), installation);

    // 要保证挂载的版本：**以激活清单为准**（真的装着的那些）。
    // 清单里没有这个槽位时（游戏目录被整个清过），才退回「槽位当前选中的那版」；
    // 两个口径打架时以清单为准，否则会同时铺两版模组。
    let installed = enabled_variants(library, installation);
    let installed_slots: Vec<&str> = installed.iter().map(|(slot, _)| slot.as_str()).collect();
    let mut wanted: Vec<(String, String)> = Vec::new();
    for (slot, entry) in &index.slots {
        if installed_slots.contains(&slot.as_str()) {
            continue;
        }
        if let Some(variant) = &entry.active {
            wanted.push((slot.clone(), variant.clone()));
        }
    }
    wanted.extend(installed);

    let mut warnings = Vec::new();
    for (slot, variant_id) in wanted {
        let Ok(kind) = require_slot(&slot) else {
            continue;
        };
        let Some(variant) = index
            .slots
            .get(&slot)
            .and_then(|entry| entry.variants.iter().find(|item| item.id == variant_id))
            .cloned()
        else {
            continue;
        };
        // 库里的内容没了就没什么可铺的（activate 会给出更准确的报错）
        if !library.slot_dir(&slot).join(&variant.id).is_dir() {
            continue;
        }

        let plan = plan_mods(library, &slot, &variant, &kind)?;

        // 自动挂载是**无人在场**的操作，不能悄悄抢别人的位置：
        // - 目标归另一个战役 / 一个独立模组：两边每次启动会互相 takeover + 重拷；
        // - 目标归谁都不知道、但盘上已经有一份：那是用户自己放的，动它要问人。
        // 两种情况都跳过并留一句提示；真要处理，走「模组」页或重新启用（那条路有预演）。
        let blocked = plan.items.iter().find_map(|item| {
            let target = installation
                .root
                .join(item.target.replace('/', std::path::MAIN_SEPARATOR_STR));
            match manifest.owner_of(&item.target) {
                Some(owner) if *owner != plan.owner => Some(format!(
                    "「{}」的模组 {} 现在归{}，自动挂载已跳过",
                    variant.name,
                    item.target,
                    owner.label()
                )),
                Some(_) => None,
                None => {
                    // 盘上有一份不是我们放的就别抢：文件本身，或它所属模组的根目录
                    // （目录树形态的模组，包内是一堆文件，用户手动放的是整个目录）
                    let mod_root = crate::library::mod_key_of(&item.target)
                        .map(|found| installation.root.join("Mods").join(found.key));
                    let exists = target.exists() || mod_root.is_some_and(|path| path.exists());
                    exists.then(|| {
                        format!(
                            "「{}」的模组 {} 在游戏目录里已有一份不是启动器放的，自动挂载已跳过",
                            variant.name, item.target
                        )
                    })
                }
            }
        });
        if let Some(line) = blocked {
            warnings.push(line);
            continue;
        }

        if manifest.settled(installation, &plan, "Mods/") {
            continue;
        }
        match manifest.apply_scoped(library.root(), installation, &plan, "Mods/") {
            Ok(found) => warnings.extend(found),
            Err(error) => warnings.push(format!("「{}」的模组自动挂载失败：{error}", variant.name)),
        }
    }

    Ok(warnings)
}

/// 「这个版本这次要铺的模组」那一层计划 —— 只取 `Mods/` 落点。
fn plan_mods(
    library: &Library,
    slot_slug: &str,
    variant: &super::Variant,
    kind: &crate::campaign::CampaignType,
) -> Result<Plan> {
    let sub = variant
        .target_sub
        .clone()
        .or_else(|| kind.sub_directory().map(str::to_string));
    let composition = compose::compose(library, slot_slug, variant, sub.as_deref());

    let mut plan = Plan::new(Owner::Campaign {
        slot: slot_slug.to_string(),
        variant: variant.id.clone(),
    });
    for item in &composition.files {
        if item.target.starts_with("Mods/") {
            plan.push(item.source.clone(), item.target.clone());
        }
    }
    Ok(plan)
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
/// 一条「本战役要写、但已经被另一个**已启用**的战役占着」的目标。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ActiveConflict {
    /// 相对游戏根目录，`/` 分隔。
    pub target: String,
    /// 占着它的那个战役槽位。
    pub slot: String,
    /// 槽位显示名（界面直接念，不用自己再映射一遍）。
    pub slot_name: String,
    /// 那个战役当前启用的版本 id。
    pub variant: String,
    /// 版本显示名。
    pub variant_name: String,
}

/// 两个目标路径是不是同一个位置。
///
/// **按平台规则**：Windows 的文件系统不区分大小写（`Mods/Foo.SC2Mod` 与
/// `mods/foo.sc2mod` 是同一个位置），别的平台区分。只折 ASCII 大小写 ——
/// 地图 / 模组名都是 ASCII，非 ASCII 大小写在 Windows 上依赖区域设置，不值得为此加复杂度。
pub(crate) fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// 槽位的显示名；认不出来就退回 slug。
fn slot_label(slug: &str) -> String {
    crate::campaign::metadata::CampaignType::from_slug(slug)
        .map(|kind| kind.display_name())
        .unwrap_or_else(|| slug.to_string())
}

/// 某个槽位里某个版本的显示名；找不到就退回 id。
fn variant_label(index: &super::LibraryIndex, slot: &str, variant: &str) -> String {
    index
        .slots
        .get(slot)
        .and_then(|entry| entry.variants.iter().find(|item| item.id == variant))
        .map(|item| item.name.clone())
        .unwrap_or_else(|| variant.to_string())
}

/// **只读**：启用这个版本会覆盖哪些**别的已启用战役**的东西。
///
/// 判三件事（这三条错一条，用户就会看到假警告、或者在真冲突时被静默覆盖）：
/// 1. **只跟已启用的比** —— 数据源是安装清单（`installed.json`；老库的 `active.json`
///    会先迁过来），也就是"真的已经铺在游戏目录里的"。没启用的版本不占任何位置。
/// 2. **同一个战役自己上次留下的不算冲突** —— 换版本 / 重新启用同一版只是重铺自己的东西。
/// 3. 路径按**平台大小写规则**比较（见 [`same_path`]）。
///
/// 和 `preview` 共用 `plan_for` 与同一份清单，所以"预演说会撞、实际却没撞"这种分叉不会有。
/// 调用方（界面）必须在**写盘之前**调它 —— 它一个字节都不写。
pub fn conflicts(
    library: &Library,
    installation: &Installation,
    slot_slug: &str,
    variant_id: &str,
) -> Result<Vec<ActiveConflict>> {
    let kind = require_slot(slot_slug)?;
    let index = library.index();
    let variant = index
        .slots
        .get(slot_slug)
        .and_then(|slot| slot.variants.iter().find(|item| item.id == variant_id))
        .cloned()
        .ok_or_else(|| Error::CampaignNotFound(variant_id.to_string()))?;

    let plan = plan_for(library, slot_slug, &variant, &kind)?;
    let manifest = Manifest::load_migrating(library.root(), installation);

    let mut found: Vec<ActiveConflict> = Vec::new();
    for item in &plan.items {
        for installed in &manifest.files {
            // 独立模组不算"另一个战役"：它在模组页自己管启停
            let Owner::Campaign {
                slot,
                variant: owner_variant,
            } = &installed.owner
            else {
                continue;
            };
            // 自己这个槽位上次留下的东西：重铺一遍而已
            if slot == slot_slug {
                continue;
            }
            if !same_path(&installed.target, &item.target) {
                continue;
            }
            found.push(ActiveConflict {
                target: item.target.clone(),
                slot: slot.clone(),
                slot_name: slot_label(slot),
                variant: owner_variant.clone(),
                variant_name: variant_label(&index, slot, owner_variant),
            });
        }
    }

    // 同一条目标可能被多个人记着（历史遗留），去重后按路径排序 —— 界面才好稳定显示
    found.sort();
    found.dedup();
    Ok(found)
}

/// 启用失败时的错误：权限错误必须保留 `[PERMISSION]` 前缀。
///
/// 界面靠这个前缀判断要不要走提权重放；以前这里统一包成 InstallRolledBack，
/// 前缀被“「x」安装失败…”盖住，结果启用战役卡在普通权限里报错，
/// 而安装同一份包却能正常弹提权。
fn activation_failure(variant_name: &str, error: &Error, suffix: &str) -> Error {
    let reason = error.to_string();
    let reason = reason.strip_prefix("[PERMISSION] ").unwrap_or(&reason);
    let message = format!("「{variant_name}」安装失败，已回滚到原版状态：{reason}{suffix}");
    if error.is_permission_denied() {
        Error::PermissionDenied(message)
    } else {
        Error::InstallRolledBack(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::library::install::{Manifest, Owner, Plan};
    use crate::sc2::{DiscoverySource, Installation};

    /// 造一个能通过校验的假游戏目录。
    fn fake_game(tmp: &std::path::Path) -> Installation {
        let game = tmp.join("game");
        std::fs::create_dir_all(&game).expect("建目录");
        std::fs::write(game.join("StarCraft II.exe"), b"x").expect("写 exe");
        Installation::from_root(&game, DiscoverySource::Manual).expect("假安装")
    }

    #[test]
    fn enabled_variants_comes_from_the_manifest_not_the_selection() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let library = Library::new(tmp.path().join("data"));
        let installation = fake_game(tmp.path());
        let source = tmp.path().join("pkg.SC2Map");
        std::fs::write(&source, b"x").expect("写源文件");

        // 游戏目录里实际装了两部自制战役（清单两条）—— 槽位 `active` 只有一个，
        // 所以「装了没」只能看清单
        let mut manifest = Manifest::default();
        for variant in ["甲", "乙"] {
            let mut plan = Plan::new(Owner::Campaign {
                slot: "custom".to_string(),
                variant: variant.to_string(),
            });
            plan.push(
                source.clone(),
                format!("Maps/CustomCampaigns/{variant}/{variant}.SC2Map"),
            );
            manifest
                .apply(library.root(), &installation, &plan)
                .expect("装");
        }

        let mut enabled = enabled_variants(&library, &installation);
        enabled.sort();
        let mut expected = vec![
            ("custom".to_string(), "甲".to_string()),
            ("custom".to_string(), "乙".to_string()),
        ];
        expected.sort();
        assert_eq!(enabled, expected, "两部自制都装着 -> 两条都在");

        // 撤掉一部 -> 只剩一条
        manifest
            .remove(
                library.root(),
                &installation,
                &Owner::Campaign {
                    slot: "custom".to_string(),
                    variant: "甲".to_string(),
                },
            )
            .expect("撤");
        assert_eq!(
            enabled_variants(&library, &installation),
            vec![("custom".to_string(), "乙".to_string())]
        );
    }

    #[test]
    fn activation_failure_keeps_permission_prefix() {
        let error = Error::PermissionDenied("数据根写不进去".to_string());
        let mapped = activation_failure("虫群之心", &error, "");
        assert!(mapped.is_permission_denied());
        let text = mapped.to_string();
        assert!(text.starts_with("[PERMISSION] "), "{text}");
        assert!(text.contains("已回滚到原版状态"), "{text}");

        let plain = Error::PackageRejected("包坏了".to_string());
        let mapped = activation_failure("虫群之心", &plain, "");
        assert!(!mapped.is_permission_denied());
        assert!(!mapped.to_string().starts_with("[PERMISSION]"));
    }
}
