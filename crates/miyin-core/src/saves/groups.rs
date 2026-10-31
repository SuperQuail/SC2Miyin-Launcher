//! 存档隔离：开关、组绑定、切换，以及归属候选清单。

use std::path::{Path, PathBuf};

use super::backup::root as saves_root;
use super::backup::{
    backup_paths, backups, clear, copy_into, dir_name, restore_paths, snapshot_paths, stamp,
};
use super::roots::SavesRoots;
use super::{Isolation, ORIGINAL, SaveTarget};
use crate::error::Result;
use crate::library::{Library, LibraryIndex, Variant};

// ---------------------------------------------------------------------------
// 存档隔离（按安装的战役分开）
// ---------------------------------------------------------------------------

pub(super) fn state_file(root: &Path) -> PathBuf {
    root.join("saves").join("state.json")
}

/// 读隔离状态。没写过就是默认（关着）。
///
/// 状态文件**读不出来**时不静默当成「关着」：那等于把用户的组绑定全丢掉还不吭声。
/// 现在返回同样的默认值，但带上 `warning`（界面在隔离卡里原样展示）。
pub fn isolation(root: &Path) -> Result<Isolation> {
    let file = state_file(root);
    if !file.is_file() {
        return Ok(Isolation::default());
    }
    let text = std::fs::read_to_string(&file)?;
    match serde_json::from_str(&text) {
        Ok(state) => Ok(state),
        Err(error) => Ok(Isolation {
            warning: Some(format!(
                "存档隔离的状态文件读不出来（{error}），现在按「关着」处理：原来的分组与归属暂时显示不出来。请先在存档页把当前进度另存一份备份，再重新打开隔离。"
            )),
            ..Isolation::default()
        }),
    }
}

pub(super) fn write_isolation(root: &Path, state: &Isolation) -> Result<()> {
    let dir = root.join("saves");
    std::fs::create_dir_all(&dir)?;
    let file = state_file(root);
    // 读不出来的旧文件先挪到一边再覆盖：直接盖掉的话，里面还记得的归属就彻底没了
    if let Ok(old) = std::fs::read_to_string(&file)
        && serde_json::from_str::<Isolation>(&old).is_err()
    {
        let _ = std::fs::rename(&file, dir.join(format!("state.json.corrupt-{}", stamp())));
    }
    // warning 只是这一趟读出来的提示，不落盘
    let clean = Isolation {
        warning: None,
        ..state.clone()
    };
    let tmp = dir.join("state.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&clean)?)?;
    std::fs::rename(&tmp, file)?;
    Ok(())
}

/// 打开 / 关掉隔离。
///
/// 打开时若还没归过档，**先把现在这份 Banks 存成「原版」** —— 这就是
/// "初始存档按照原版存"。Banks 还是空的（还没玩过）就只把开关打开：没有东西可收，
/// 第一份进度会在第一次存回时归成「原版」。关掉只是不再自动切，已经存下的组一个都不删。
///
/// **关掉 / 重新打开时把 active 清掉**：关着的那段时间 Banks 里是谁的进度不可知
/// （用户可能直接进游戏玩了别的战役），旧的 active 已经不成立了 —— 留着它，
/// 重新打开后第一次切组会把那份进度写进上一个战役的组。重新打开时当前这份会被
/// 当成「原版」收下（一组新的，不是接着改上一次那个组）。
pub fn set_isolation(root: &Path, banks: &Path, enabled: bool) -> Result<Isolation> {
    set_isolation_paths(root, &SavesRoots::from_banks(banks), enabled)
}

/// 同 [`set_isolation`]，但对**所有真实存档根**整体操作 —— 打开那一刻把所有账号的
/// 进度一起收成一组（同一部战役的进度不在账号之间拆开）。
pub fn set_isolation_paths(root: &Path, roots: &SavesRoots, enabled: bool) -> Result<Isolation> {
    let mut state = isolation(root)?;
    if !enabled || !state.enabled {
        state.active = None;
    }
    if enabled && state.active.is_none() {
        // 还没玩过（Banks 不存在 / 是空的）就**没有可收的**，开关照常打开；
        // 第一份进度会在第一次存回时归成「原版」。
        // 其它失败（权限、磁盘满）照旧往上抛 —— 不能把「没存成」说成「存好了」。
        if !snapshot_paths(roots)?.files.is_empty() {
            state.active = Some(backup_paths(roots, root, ORIGINAL)?);
        }
    }
    state.enabled = enabled;
    write_isolation(root, &state)?;
    Ok(state)
}

/// 把一个组的内容换成**现在游戏里这份**（覆盖式）。
///
/// 先清掉旧内容再铺 —— 否则被删掉的档会留在组里，两份混在一起。
fn save_into_roots(roots: &SavesRoots, dir: &Path) -> Result<()> {
    // 先把现在的 Banks 读一遍，再动目标组：顺序反过来的话，读失败（权限 / 盘掉了）
    // 发生在清空之后，组里那份进度就没了。Banks 目录整个不存在同样不是「空」——
    // 那是「不知道存哪儿」，绝不能拿它覆盖组里的存档。
    let (missing, files) = roots.files()?;
    if missing {
        return Err(crate::error::Error::PackageRejected(
            "找不到存档目录（Banks），这一组没有覆盖".to_string(),
        ));
    }
    std::fs::create_dir_all(dir)?;
    clear(dir)?;
    copy_into(&files, dir)
}

/// 把**现在游戏里这份** Banks 存回它所属的组。没归过档就先归到「原版」。
pub fn save_active(root: &Path, banks: &Path) -> Result<Isolation> {
    save_active_paths(root, &SavesRoots::from_banks(banks))
}

/// 同 [`save_active`]，但对**所有真实存档根**整体操作。
pub fn save_active_paths(root: &Path, roots: &SavesRoots) -> Result<Isolation> {
    let mut state = isolation(root)?;
    let name = match state.active.clone() {
        Some(name) => name,
        None => {
            let name = backup_paths(roots, root, ORIGINAL)?;
            state.active = Some(name.clone());
            name
        }
    };

    save_into_roots(roots, &saves_root(root).join(&name))?;
    write_isolation(root, &state)?;
    Ok(state)
}

/// 把**现在游戏里这份** Banks 存进**指定的组**，并把它记为当前组。
///
/// 和 `save_active` 的区别：那个只会存回"当前组"，这个由用户点名 ——
/// 存档页上的「设为当前（把游戏里这份存回它）」就是它。
pub fn save_into(root: &Path, banks: &Path, name: &str) -> Result<Isolation> {
    save_into_paths(root, &SavesRoots::from_banks(banks), name)
}

/// 同 [`save_into`]，但对**所有真实存档根**整体操作。
pub fn save_into_paths(root: &Path, roots: &SavesRoots, name: &str) -> Result<Isolation> {
    let dir = saves_root(root).join(name);
    if !dir.is_dir() {
        return Err(crate::error::Error::PackageRejected(format!(
            "没有这个存档组：{name}"
        )));
    }

    let mut state = isolation(root)?;
    state.active = Some(name.to_string());
    save_into_roots(roots, &dir)?;
    write_isolation(root, &state)?;
    Ok(state)
}

/// 切到某个组：**先把现在这份存回去**，再把目标组铺出来。
///
/// 目标组是空的（新建的战役组）=> Banks 会被清空，也就是"这个战役从头开始"。
/// 原来那份已经安全地存在它自己的组里了。
pub fn switch(root: &Path, banks: &Path, name: &str) -> Result<Isolation> {
    switch_paths(root, &SavesRoots::from_banks(banks), name)
}

/// 同 [`switch`]，但对**所有真实存档根**整体操作：现在这份先存回自己那组，
/// 再把目标组按账号 / 地域层级铺回每一个根。
pub fn switch_paths(root: &Path, roots: &SavesRoots, name: &str) -> Result<Isolation> {
    let dir = saves_root(root).join(name);
    if !dir.is_dir() {
        return Err(crate::error::Error::PackageRejected(format!(
            "没有这个存档组：{name}"
        )));
    }

    // 现在这份先回自己的组。
    //
    // **没归过档也要先收成「原版」** —— 用户刚打开隔离、还没归过档时第一次切组
    // 走的就是这条路；少收这一步，那段进度就只剩 restore() 里那份「还原前」
    // 安全备份，界面上找不到「原版」组，用户会以为进度没了。
    // 只有 Banks 完全是空的时候才跳过（没东西可存）。
    let mut state = isolation(root)?;
    if state.active.is_some() || !snapshot_paths(roots)?.files.is_empty() {
        state = save_active_paths(root, roots)?;
    }

    // 铺目标组（内部会先给现在这份留后路，但我们已经存过了，这里是幂等的）
    restore_paths(roots, root, name)?;
    state.active = Some(name.to_string());
    write_isolation(root, &state)?;
    Ok(state)
}

/// 手动改一个组的归属：指给某个战役，或 `None` 表示"这是原版 / 不属于任何战役"。
pub fn assign(root: &Path, name: &str, slot: Option<&str>) -> Result<Isolation> {
    let mut state = isolation(root)?;
    match slot {
        Some(slot) if !slot.trim().is_empty() => {
            // 一个战役只能占一个组：先把别的组身上这个归属摘掉
            state.assignments.retain(|_, value| value != slot);
            state.assignments.insert(name.to_string(), slot.to_string());
        }
        _ => {
            state.assignments.remove(name);
        }
    }
    write_isolation(root, &state)?;
    Ok(state)
}

/// 这个战役用哪个组。没指派过就是 `None`。
pub fn profile_for(root: &Path, slot: &str) -> Result<Option<String>> {
    let state = isolation(root)?;
    Ok(state
        .assignments
        .iter()
        .find(|(_, value)| value.as_str() == slot)
        .map(|(name, _)| name.clone()))
}

/// 「原版」那个组叫什么（切回原版战役时用）。还没归过档就是 `None`。
pub fn profile_for_original(root: &Path) -> Result<Option<String>> {
    let state = isolation(root)?;
    match state.active.clone() {
        Some(name) if name.starts_with(ORIGINAL) => Ok(Some(name)),
        _ => Ok(backups(root)?
            .into_iter()
            .find(|item| item.label == ORIGINAL)
            .map(|item| item.name)),
    }
}

/// **迁移/自愈**：把 `active` 那个组的绑定修正到它真正代表的东西上。四种情况：
/// 1. 绑着**战役级键**（`lotv`）但那个槽位当前启用的是某个改版 -> 重绑到版本键 ——
///    但那个版本键**已经有组认领**时不抢（见下）；
/// 2. **完全没有绑定**、而某个槽位正用着原版 -> 绑到那个槽位键（唯一候选才动手，见
///    [`bind_unbound_active`]）；
/// 3. 键是自制战役的（`custom` / `custom/<身份>`）-> 不动：旧键说明不了是哪一部；
/// 4. 其余（已经绑对了 / 没有 `active`）：原样返回。
///
/// 为什么会有这种数据：升级前隔离键只到战役级、而且「原版」那个组是开隔离时自动收的、
/// 从来没有绑定。用户切到改版再切回原版之后，`active` 还指着那个组 —— 界面上就成了
/// 「当前在用 未分类」，其实那是原版存档。
///
/// **只改 `assignments`**：不动组名、不动文件、不新建组、不碰 Banks。幂等。
/// 返回是否真的改过（测试与日志用）。
pub fn reconcile_active_binding(library: &Library, root: &Path) -> Result<bool> {
    let state = isolation(root)?;
    let Some(active) = state.active.clone() else {
        return Ok(false);
    };
    // 没有任何绑定：可能该绑到「原版」的槽位键上
    let Some(key) = state.assignments.get(&active).cloned() else {
        return bind_unbound_active(library, root, &active);
    };
    // 已经是「槽位/身份」这种版本级键：不用管。
    //
    // 自制战役的键一律不动：老数据里的 custom 说明不了是哪一部，而槽位上
    // 「选中」的版本也不等于这组进度就属于它（自制战役可以同时装好几部）——
    // 猜错就是把两部战役的存档串在一起（见 isolation_key 的说明）。
    if is_custom_key(&key) || key.contains('/') {
        return Ok(false);
    }

    // 战役级键 + 槽位当前启用的是某个版本 -> 重绑到那个版本的真实键
    let index = library.index();
    let Some(variant) = index.slots.get(&key).and_then(|entry| entry.active.clone()) else {
        // 槽位不存在，或者正用着原版：战役级键本来就是对的
        return Ok(false);
    };
    let real = isolation_key(library, &key, Some(&variant));
    if real == key {
        return Ok(false);
    }
    // 这个版本键已经有别的组认领了：那个组才是这一版的存档。硬绑过来只是
    // 把两份进度换了标签（还可能让真正的这一版变成「未分类」）。先不动，让用户自己指。
    if state
        .assignments
        .iter()
        .any(|(name, value)| value == &real && name != &active)
    {
        return Ok(false);
    }

    // 用 assign 而不是直接插：它保证「一个键只归一个组」
    assign(root, &active, Some(&real))?;
    Ok(true)
}

/// `active` 那个组没有任何绑定：如果有**唯一**一个候选槽位（正用着原版、而且它的键
/// 还没有组绑着），就把它绑到那个槽位键上。
///
/// 为什么要求唯一：一个组只能绑一个键。四个官方槽位通常都「正用着原版」，挑哪个都是猜 ——
/// 猜错就是把别的槽位的键抢过来。宁可先不绑（界面显示「未分类」），让用户指一次。
fn bind_unbound_active(library: &Library, root: &Path, active: &str) -> Result<bool> {
    use crate::campaign::metadata::CampaignType;

    let state = isolation(root)?;
    let index = library.index();
    let mut candidates: Vec<String> = Vec::new();
    for kind in CampaignType::MENU {
        if kind.is_custom() {
            continue;
        }
        let slot = kind.slug();
        // 库里根本没有这个槽位：不算候选（它就是没启用过，不是「正用着原版」）
        let Some(entry) = index.slots.get(slot) else {
            continue;
        };
        // 槽位得是「正用着原版」（没有启用的版本）
        if entry.active.is_some() {
            continue;
        }
        // 这个键已经有组绑着（可能就是这个组自己，也可能不是）→ 不算候选
        if state.assignments.values().any(|value| value == slot) {
            continue;
        }
        candidates.push(slot.to_string());
    }

    if candidates.len() != 1 {
        return Ok(false);
    }
    assign(root, active, Some(&candidates[0]))?;
    Ok(true)
}

/// 建一个空组，返回组名。
pub(super) fn fresh_group(root: &Path, label: &str) -> Result<String> {
    let name = dir_name(label);
    std::fs::create_dir_all(saves_root(root).join(&name))?;
    Ok(name)
}

/// 自制战役隔离键的前缀。
const CUSTOM_PREFIX: &str = "custom/";

/// 这个隔离键是不是自制战役的。
///
/// 两种都算：新格式 `custom/<身份>`，以及老数据里光秃秃的 `custom`
/// （老数据不自动迁移 —— 见 [`isolation_key`] 的说明）。
pub fn is_custom_key(key: &str) -> bool {
    key == "custom" || key.starts_with(CUSTOM_PREFIX)
}

/// 变体的「身份」：注册 ID 优先（大小写归一），没写才退回版本目录名。
///
/// **为什么不用版本 id**：版本 id 是「这一份包」，同一部战役（改版 / 自制）的每个版本都不同 ——
/// 拿它当隔离键的话，用户「更新了同一部战役的新版本」就等于换了一份存档，
/// 旧进度会被甩在一个再也不会被用到的组里（那是真丢数据）。
/// **代价**：两部不同的战役如果声明了同一个注册 ID，会被认成同一部、共用一个组 ——
/// 可以接受：`conflict_for` 认「同一部战役的新版本」用的也是注册 ID，口径一致；
/// 而且注册 ID 本来就是作者声明的稳定身份。
fn identity_of(variant: &Variant) -> String {
    let declared = variant
        .registration_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if declared.is_empty() {
        variant.id.clone()
    } else {
        declared.to_lowercase()
    }
}

/// 这个版本的**隔离键**。
///
/// - `variant_id` 为 `None`（**原版**）-> 键就是槽位本身（`wol` / `lotv` …）。
///   旧数据里的 `lotv` / `wol` 就是原版组，语义不变，**不用迁移**。
/// - 有变体（**官方改版也算**）-> `<槽位>/<身份>`，例如 `lotv/肉鸽`。
///   用户原话：「存档对应的应该是各个导入的战役 mod 版本和原版，而不是其他战役，
///   比如虚空之遗的肉鸽 mod 和虚空之遗原版互相隔离」—— 改版是另一个游戏。
/// - 自制战役同理：`custom/<身份>`（每部自制战役一组）。
///
/// 身份见 [`identity_of`]：注册 ID 优先，没写才退回版本目录名。
/// 没写注册 ID 的包换版本会各自一组，用户可以在存档页手动把归属改到同一组
/// （不猜 —— 猜测才会串档）。
///
/// 老数据里的 `custom` 键**不会被新键匹配到**（新键形如 `custom/<身份>`）：
/// 宁可让用户手动指一次，也不要猜错版本、把两部自制战役的存档串在一起。
pub fn isolation_key(library: &Library, slot: &str, variant_id: Option<&str>) -> String {
    // 原版：就是这个槽位本身
    let Some(variant_id) = variant_id else {
        return slot.to_string();
    };

    let index = library.index();
    let variant = index
        .slots
        .get(slot)
        .and_then(|entry| entry.variants.iter().find(|item| item.id == variant_id));
    match variant {
        Some(item) => format!("{slot}/{}", identity_of(item)),
        // 索引里查不到这个版本（极端情况）：退回版本目录名，别 panic
        None => format!("{slot}/{variant_id}"),
    }
}

/// 一个槽位里的变体按隔离键归并：同一部（共享注册 ID）只留一条。
///
/// 名字取「当前启用那版」，都没启用就取导入时间最新的 —— 稳定，不随遍历顺序漂
/// （不然页面上同一个条目会忽然换名字）。
fn picked_variants(
    index: &LibraryIndex,
    slot: &str,
    installed: &[(String, String)],
) -> Vec<(String, String, Option<String>, bool)> {
    struct Picked {
        key: String,
        name: String,
        version: Option<String>,
        active: bool,
        imported_at: u64,
        enabled: bool,
    }

    let Some(entry) = index.slots.get(slot) else {
        return Vec::new();
    };

    let mut picked: Vec<Picked> = Vec::new();
    for variant in &entry.variants {
        let key = format!("{slot}/{}", identity_of(variant));
        let active = entry.active.as_deref() == Some(variant.id.as_str());
        // 装着没：按激活清单，不看 active
        let installed_variant = installed.iter().any(|(installed_slot, installed_id)| {
            installed_slot == slot && installed_id == &variant.id
        });
        match picked.iter_mut().find(|item| item.key == key) {
            None => picked.push(Picked {
                key,
                name: variant.name.clone(),
                version: variant.version.clone(),
                active,
                imported_at: variant.imported_at,
                enabled: installed_variant,
            }),
            Some(item) => {
                // 同一部战役的任一版本装着，这条就算「装着」
                item.enabled |= installed_variant;
                // 已启用那版不换；否则「这个启用」或「这个更新」就换掉它。
                // 都没启用就比导入时间（新的赢），时间相同就保持先出现的。
                let better = !item.active && (active || variant.imported_at > item.imported_at);
                if better {
                    item.name = variant.name.clone();
                    item.version = variant.version.clone();
                    item.active = active;
                    item.imported_at = variant.imported_at;
                }
            }
        }
    }

    picked
        .into_iter()
        .map(|item| (item.key, item.name, item.version, item.enabled))
        .collect()
}

/// 把「键 + 版本名」变成下拉里的一条，**重名时补上版本号**。
///
/// 两个不同的组可能版本名一模一样（不同注册 ID、名字相同），
/// 不补版本的话下拉里就是两条完全一样的选项，用户根本没法选。
fn named_targets(
    campaign: &str,
    picked: Vec<(String, String, Option<String>, bool)>,
) -> Vec<SaveTarget> {
    let bases: Vec<String> = picked
        .iter()
        .map(|(_, name, _, _)| format!("{campaign} · {name}"))
        .collect();
    let mut out: Vec<SaveTarget> = Vec::with_capacity(picked.len());
    for ((key, _, version, enabled), base) in picked.into_iter().zip(bases.iter()) {
        let duplicated = bases.iter().filter(|item| *item == base).count() > 1;
        let label = if !duplicated {
            base.clone()
        } else {
            match version {
                Some(version) if !version.trim().is_empty() => format!("{base} {version}"),
                // 连版本号都没有（两个不同注册 ID、同名同版本）：退回身份（目录名），
                // 它一定唯一 —— 下拉里绝不能出现两条一模一样的选项。
                _ => format!("{base}（{}）", key.rsplit('/').next().unwrap_or(&key)),
            }
        };
        out.push(SaveTarget {
            key,
            label,
            enabled,
        });
    }
    out
}

/// 某个隔离键对应的**组名**（`dir_name` 之前那份，给人看的）。
///
/// 规则和启用时造组一致：
/// - 原版（键 = 槽位）-> 战役名；
/// - 有版本 -> 官方「战役名 · 版本名」，自制用版本名；
/// - 认不出来的键（老数据里的 `custom`、或那个版本已经从库里删了）-> 原样用键。
pub fn group_label(library: &Library, key: &str) -> String {
    use crate::campaign::metadata::CampaignType;

    let index = library.index();
    for kind in CampaignType::MENU {
        let slot = kind.slug();
        let name = kind.display_name();
        if key == slot {
            return name;
        }
        let Some(entry) = index.slots.get(slot) else {
            continue;
        };
        for variant in &entry.variants {
            if format!("{slot}/{}", identity_of(variant)) != key {
                continue;
            }
            return if kind.is_custom() {
                variant.name.clone()
            } else {
                format!("{name} · {}", variant.name)
            };
        }
    }
    key.to_string()
}

/// 归属下拉的候选清单。
///
/// - **官方**每个槽位两条起：`<战役名>（原版）`（键就是槽位，老数据直接对得上）
///   再跟上它的每个改版 `<战役名> · <版本名>`（键 `slot/<身份>`）；
/// - **自制**每个变体一条（同一部只列一条的规则不变）。
///
/// 顺序：官方按固定菜单顺序，每个槽位先原版后版本；自制排在最后。
/// 和 [`isolation_key`] 用同一套规则算键 —— 界面不要自己拼键，免得两处规则漂移。
pub fn targets(library: &Library, installed: &[(String, String)]) -> Vec<SaveTarget> {
    use crate::campaign::metadata::CampaignType;

    let index = library.index();
    let mut out: Vec<SaveTarget> = Vec::new();

    for kind in CampaignType::MENU {
        // 自制战役不按槽位列，最后单独列
        if kind.is_custom() {
            continue;
        }
        let slot = kind.slug();
        let name = kind.display_name();
        // 原版「装着」= 这个槽位一个版本都没装
        let vanilla = !installed
            .iter()
            .any(|(installed_slot, _)| installed_slot == slot);
        out.push(SaveTarget {
            key: slot.to_string(),
            label: format!("{name}（原版）"),
            enabled: vanilla,
        });
        out.extend(named_targets(
            &name,
            picked_variants(&index, slot, installed),
        ));
    }

    if index.slots.contains_key("custom") {
        let prefix = CampaignType::Custom.display_name();
        out.extend(named_targets(
            &prefix,
            picked_variants(&index, "custom", installed),
        ));
    }

    out
}

/// 启用某个战役之前调它：隔离开着就切到它的存档组。
///
/// `key` 是 [`isolation_key`] 算出来的隔离键（原版 = 槽位，改版 / 自制 = `槽位/身份`）；
/// 调用方负责算，别在这里再推一次规则。
///
/// 没有记录时用哪一份：
/// - **原版键**（战役级，不含斜杠）：先复用「原版」那个**还没归属**的老组 ——
///   当年开隔离时收下的那份就是原版进度，**不能**因为没绑定就新建一个空组把它晾在
///   一边（用户会把一份好好的原版存档看成「未分类」）。已经绑在别的键上的组不碰
///   （复用等于把另一个战役的组抢过来）；组名、文件都不动，只补一条绑定。
/// - **自制战役的某个版本**（`custom/<身份>`）：永远空组（每部各管各的，不继承）。
///   光秃秃的 `custom`（老数据）直接返回 `None`：没有「自制战役原版」这种东西，
///   不建组、不切档。
/// - **还没归过档**（刚升级上来 / 刚打开隔离，`active == None`）：现在游戏里这份就是
///   「我在玩的那部」（自制战役除外），把它收成这个组的。切组前 `save_active` 还会
///   先把同一份收进「原版」，所以「原版」和这个组会**同时**有它 —— 刻意的，谁也没被顶掉。
/// - **其余（某个改版）**：空组。这时 Banks 是**别的组**的进度，抄进新组就是
///   串档（用户抱怨的正是这个：改版是另一个游戏）。
///
/// 已经有组就照旧切过去。返回切到了哪个组（没开隔离返回 `None`）。
pub fn ensure_for_slot(
    root: &Path,
    banks: &Path,
    key: &str,
    label: &str,
) -> Result<Option<String>> {
    ensure_for_slot_paths(root, &SavesRoots::from_banks(banks), key, label)
}

/// 同 [`ensure_for_slot`]，但对**所有真实存档根**整体操作。
pub fn ensure_for_slot_paths(
    root: &Path,
    roots: &SavesRoots,
    key: &str,
    label: &str,
) -> Result<Option<String>> {
    let state = isolation(root)?;
    if !state.enabled {
        return Ok(None);
    }

    // 自制战役没有「原版」组：光秃秃的 custom 只出现在老数据里，新键一律是
    // custom/<身份>。不建组、也不切过去（见 isolation_key 的说明）。
    if is_custom_key(key) && !key.contains('/') {
        return Ok(None);
    }

    if let Some(name) = profile_for(root, key)? {
        switch_paths(root, roots, &name)?;
        return Ok(Some(name));
    }

    // 只有「还没归过档」才把当前这份收成新组；其余情况从空组开始
    let adopt_current = !is_custom_key(key) && state.active.is_none();
    let adopt = || -> Result<String> {
        // Banks 还没东西（第一次玩）时没得接，退回空组
        match backup_paths(roots, root, label) {
            Ok(name) => Ok(name),
            Err(_) => fresh_group(root, label),
        }
    };
    let name = if is_campaign_key(key) {
        // 原版键：只复用**还没归属**的老组（那份内容就是原版进度），
        // 别新建一个空组把它晾在一边；也别抢别人已经绑好的组
        match original_group(root)? {
            Some(name) => name,
            None if adopt_current => adopt()?,
            None => fresh_group(root, label)?,
        }
    } else if adopt_current {
        adopt()?
    } else {
        fresh_group(root, label)?
    };

    assign(root, &name, Some(key))?;
    switch_paths(root, roots, &name)?;
    Ok(Some(name))
}

/// 战役级键（原版）：就是槽位本身，不带 `/`。
fn is_campaign_key(key: &str) -> bool {
    !is_custom_key(key) && !key.contains('/')
}

/// 切回原版时该用哪个组。
///
/// 只找**还没有归属**的那个老组（标签是「原版」、且没有任何绑定，见
/// [`super::BackupEntry::legacy`]）。**已经绑在别的键上的组绝不能复用**：
/// 一个组只归一个键，复用就等于把它从另一个战役身上抢走 —— 两个官方战役会
/// 从此共用一份存档，谁切换谁改它的归属。
///
/// 没有可复用的就返回 `None`，由调用方决定（新建空组 / 收下当前进度）。
/// 只读：返回组名，绑定交给调用方。
fn original_group(root: &Path) -> Result<Option<String>> {
    Ok(backups(root)?
        .into_iter()
        .find(|item| item.legacy)
        .map(|item| item.name))
}
