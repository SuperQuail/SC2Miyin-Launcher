//! 存档管理：`Documents\StarCraft II\Banks` 的备份与还原（issue #20）。
//!
//! **存档是用户数据**，不是游戏本体 —— 所以它不走游戏目录那套写盘闸门
//! （那个闸门只管安装目录里的文件）。但同样守两条：**动之前先备份**、
//! **先给人看**（[`SaveSet`] 就是给人看的那一份）。
//!
//! **存档隔离**：**默认关着**，用户主动打开才生效；打开之后每个战役用自己的那份
//! Banks，切换战役时自动换档。官方战役按**槽位**共用一组（同一部的各个改版本来
//! 就该共用进度）；自制战役**每部一组**（键是 `custom/<身份>`，见 [`isolation_key`]）。
//! 关着的时候启动器一律不碰 Banks；关掉之后已经存下的组都留着。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::library::{Library, LibraryIndex, Variant};
use crate::safety;

/// 备份目录名里的时间戳：`20261009-181500`。
fn stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 不引时间库：只要单调、可读、能排序就够了
    format!("{now}")
}

/// 一份存档快照：有哪些文件、多大、最后改动是什么时候。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SaveSet {
    pub files: Vec<SaveFile>,
    pub bytes: u64,
    /// 目录不存在（还没产生过存档）时为 true —— 界面要按「可能缺失」处理
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveFile {
    pub name: String,
    pub bytes: u64,
}

/// 自动备份（`restore()` 动手前留的那份）的标签。
///
/// 它**不是**用户意义上的存档组 —— 只是「刚才那份存哪儿了」的安全网。
/// 界面必须把它从组列表里摘出去（用 [`BackupEntry::auto`] 判断），
/// 但仍要能在还原列表里看到它。
pub const AUTO_BACKUP: &str = "还原前";

/// 一份已备份的存档。
#[derive(Debug, Clone, Serialize)]
pub struct BackupEntry {
    /// 备份目录名，还原时拿它指认
    pub name: String,
    /// 备份时带的标签（战役 / 用户自己写的备注）
    pub label: String,
    pub bytes: u64,
    pub files: usize,
    /// **自动生成**的备份（`restore()` 的安全网，标签见 [`AUTO_BACKUP`]）。
    ///
    /// 界面用它把这类备份从「存档组」列表里剔掉 —— 组是用户能选的东西，
    /// 自动备份不是（它跟着每次还原长一份）。
    pub auto: bool,
    /// **没有绑定任何战役/版本**的老组（升级前收下的那份，或者用户手动备份的）。
    ///
    /// 这种组的名字说明不了内容（当年是开隔离那一刻把整份 Banks 收下来的，
    /// 用户可能正在玩某个自制战役）—— 界面别再叫它「原版」，显示成
    /// 「未分类（升级前的存档）」，让用户自己去归位。
    pub legacy: bool,
}

/// 列一份 Banks 里有什么。目录不存在不算错误（第一次玩之前它就是不存在的）。
pub fn snapshot(banks: &Path) -> Result<SaveSet> {
    if !banks.is_dir() {
        return Ok(SaveSet {
            missing: true,
            ..SaveSet::default()
        });
    }

    let mut files = Vec::new();
    let mut bytes = 0;
    for entry in std::fs::read_dir(banks)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }
        bytes += meta.len();
        files.push(SaveFile {
            name: entry.file_name().to_string_lossy().into_owned(),
            bytes: meta.len(),
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(SaveSet {
        files,
        bytes,
        missing: false,
    })
}

/// 备份根目录：`<启动器目录>/data/saves`。
pub fn root(library_root: &Path) -> PathBuf {
    library_root.join("saves")
}

/// 备份里的目录名规则。`label` 由调用方给（战役名、或用户写的备注）。
fn dir_name(label: &str) -> String {
    let clean: String = label
        .chars()
        .map(|c| {
            // 保留间隔号：组名要能一眼分辨「自由之翼 · 重生 v1.5.0」
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '·' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let clean = clean.trim_matches('_').to_string();
    let clean = if clean.is_empty() {
        "手动".to_string()
    } else {
        clean
    };
    // 截断，免得标签太长把路径撑爆
    let short: String = clean.chars().take(40).collect();
    format!("{}-{}", short, stamp())
}

/// 把现在的 Banks 备份一份。返回备份目录名。
pub fn backup(banks: &Path, library_root: &Path, label: &str) -> Result<String> {
    let set = snapshot(banks)?;
    if set.missing || set.files.is_empty() {
        // 没有存档就别建空目录 —— 还原一个空备份只会让人以为"存档回来了"
        return Err(crate::error::Error::PackageRejected(
            "还没有存档可以备份".to_string(),
        ));
    }

    let name = dir_name(label);
    let target = root(library_root).join(&name);
    std::fs::create_dir_all(&target)?;
    for file in &set.files {
        std::fs::copy(banks.join(&file.name), target.join(&file.name))?;
    }
    Ok(name)
}

/// 列已经备份了哪些。
pub fn backups(library_root: &Path) -> Result<Vec<BackupEntry>> {
    let root = root(library_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    // 迁移期：没有绑定的组标成 legacy（名字不代表内容，界面别叫它「原版」）
    let state = isolation(library_root)?;
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let set = snapshot(&entry.path())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // 目录名是 `<标签>-<时间戳>`，标签可能自带连字符，从右边切
        let label = name
            .rsplit_once('-')
            .map(|(left, _)| left.to_string())
            .unwrap_or_else(|| name.clone());
        out.push(BackupEntry {
            auto: label == AUTO_BACKUP,
            // 只有「开隔离时自动收下的那份」才叫未分类 —— 用户手动备份、只是没绑战役的组
            // 有自己的名字，别也给它们贴「升级前的存档」。
            legacy: label != AUTO_BACKUP
                && label == ORIGINAL
                && !state.assignments.contains_key(&name),
            name,
            label,
            bytes: set.bytes,
            files: set.files.len(),
        });
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// 还原一份备份。
///
/// **还原之前先把现在的存档备份一份**（`还原前-<时间戳>`）—— 用户点错了还能退回来。
/// 返回那份安全备份的目录名。
pub fn restore(banks: &Path, library_root: &Path, name: &str) -> Result<String> {
    let source = root(library_root).join(name);
    if !source.is_dir() {
        return Err(crate::error::Error::PackageRejected(format!(
            "找不到存档备份：{name}"
        )));
    }

    // 先给现在这份留个后路。没有存档（第一次玩）就跳过。
    let safety = backup(banks, library_root, AUTO_BACKUP).ok();

    std::fs::create_dir_all(banks)?;
    // 清掉现有存档再铺 —— 否则会剩下备份里没有的旧文件，成了两份混在一起
    for entry in std::fs::read_dir(banks)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }
    for entry in std::fs::read_dir(&source)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), banks.join(entry.file_name()))?;
        }
    }

    Ok(safety.unwrap_or_default())
}

// ---------------------------------------------------------------------------
// 存档组里的**单个文件**（组详情：可以打开看、把某一份挪到别的组）
// ---------------------------------------------------------------------------

/// 一个组目录。组名只许**裸名字** —— 这条边界保证下面所有操作都只在
/// `<data>/saves/` 里面动，跳不出去。
fn group_dir(root: &Path, name: &str) -> Result<PathBuf> {
    let saves = self::root(root);
    safety::ensure_within(&saves, &saves.join(name))
}

/// 列出一个存档组里的存档文件（名字 + 字节数），按名字排序。
///
/// 组不存在就说清楚是哪一组；空组给空数组（空组是正常状态，不是错误）。
pub fn group_files(root: &Path, name: &str) -> Result<Vec<SaveFile>> {
    let dir = group_dir(root, name)?;
    if !dir.is_dir() {
        return Err(crate::error::Error::PackageRejected(format!(
            "没有这个存档组：{name}"
        )));
    }
    Ok(snapshot(&dir)?.files)
}

/// 把某个组里的**一份**存档挪到另一个组 —— 也就是改它归到哪个战役。
///
/// 三条底线（动的都是用户数据）：
/// - 目标组不存在就建；
/// - **同名不覆盖**，直接报错让用户自己决定（宁可报错，也不替他做决定）；
/// - 只动 `<data>/saves/` 里的文件，**绝不碰游戏目录的 Banks**（Banks 是另一套路径，
///   这个函数自始至终没有 banks 参数）；源组挪空了也不删组（用户可能还想切回来）。
pub fn move_save_file(root: &Path, from: &str, file: &str, to: &str) -> Result<()> {
    if file.is_empty() || file.contains(['/', '\\']) || file == "." || file == ".." {
        return Err(crate::error::Error::PackageRejected(format!(
            "存档文件名不合法：{file}"
        )));
    }
    if from == to {
        return Err(crate::error::Error::PackageRejected(
            "源组和目标组是同一个".to_string(),
        ));
    }

    let saves = self::root(root);
    let source = safety::ensure_within(&saves, &saves.join(from).join(file))?;
    if !source.is_file() {
        return Err(crate::error::Error::PackageRejected(format!(
            "找不到这份存档：{file}"
        )));
    }

    let target_dir = group_dir(root, to)?;
    std::fs::create_dir_all(&target_dir)?;
    let target = safety::ensure_within(&saves, &saves.join(to).join(file))?;
    if target.exists() {
        return Err(crate::error::Error::PackageRejected(format!(
            "「{to}」里已经有同名存档 {file}，没有覆盖"
        )));
    }

    // 同盘 rename 一步到位；跨盘 rename 会失败，退回复制 + 删
    if std::fs::rename(&source, &target).is_err() {
        std::fs::copy(&source, &target)?;
        std::fs::remove_file(&source)?;
    }
    Ok(())
}

/// 把某一份存档挪到**某个战役 / 版本名下**（键来自 [`targets`]）。
///
/// 用户改的是「**这一份**存档属于谁」，不是把整个组改绑到别的战役 ——
/// 组是**自动生成**的，它的名字就是它的意义（「虚空之遗」不该装 AI 盟友的档）。所以：
/// - 已经有组绑着这个键：挪进那个组；
/// - 没有：按 [`group_label`] 的规则建一个新组、自动绑上这个键，再把文件挪进去。
///
/// 底线不变：**同名不覆盖**、只动 `<data>/saves/`、**绝不碰 Banks**。
/// 返回挪进去的组名。
pub fn move_save_to_key(root: &Path, from: &str, file: &str, key: &str) -> Result<String> {
    let library = Library::new(root);
    let target = match profile_for(root, key)? {
        Some(name) => name,
        None => {
            // 新建之前先把两件事确认掉：这个键是不是真的目标、源文件在不在。
            // 不然失败时账上会多一条指向空组的绑定。
            if !targets(&library, &[]).iter().any(|item| item.key == key) {
                return Err(crate::error::Error::PackageRejected(format!(
                    "未知的归属：{key}"
                )));
            }
            let source = group_dir(root, from)?.join(file);
            if !source.is_file() {
                return Err(crate::error::Error::PackageRejected(format!(
                    "找不到这份存档：{file}"
                )));
            }
            let name = fresh_group(root, &group_label(&library, key))?;
            assign(root, &name, Some(key))?;
            name
        }
    };

    move_save_file(root, from, file, &target)?;
    Ok(target)
}
// ---------------------------------------------------------------------------
// 存档隔离（按安装的战役分开）
// ---------------------------------------------------------------------------

/// 隔离状态。存在 `<data>/saves/state.json`。
///
/// **默认关**（2026-10 产品决定）：版本管理还没做完，不替用户动 Banks；用户在设置页
/// 主动打开之后才按战役分开管，换战役不会把上一部的进度冲掉。关掉只是不再自动切，
/// 已经存下的组一个都不删。
///
/// **没有 state.json 就是关着**（`enabled = false`、`active = None`）。用户打开的那一刻
/// 还没归过档，而任何一次切组 / 存回之前都会先 `save_active` —— 也就是先把游戏里这份
/// 收成「原版」。所以打开隔离不会丢进度。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Isolation {
    pub enabled: bool,
    /// 现在游戏里这份 Banks 属于哪个组。没有组名表示还没归过档。
    pub active: Option<String>,
    /// 组 -> 战役槽位。**反查**得到"这个战役用哪份存档"。
    #[serde(default)]
    pub assignments: BTreeMap<String, String>,
}

/// 「原版」组的名字。打开隔离时第一份就是它。
pub const ORIGINAL: &str = "原版";

fn state_file(root: &Path) -> PathBuf {
    root.join("saves").join("state.json")
}

/// 读隔离状态。没写过就是默认（关着）。
pub fn isolation(root: &Path) -> Result<Isolation> {
    let file = state_file(root);
    if !file.is_file() {
        return Ok(Isolation::default());
    }
    let text = std::fs::read_to_string(&file)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn write_isolation(root: &Path, state: &Isolation) -> Result<()> {
    let dir = root.join("saves");
    std::fs::create_dir_all(&dir)?;
    let tmp = dir.join("state.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(state)?)?;
    std::fs::rename(&tmp, state_file(root))?;
    Ok(())
}

/// 打开 / 关掉隔离。
///
/// 打开时若还没归过档，**先把现在这份 Banks 存成「原版」** —— 这就是
/// "初始存档按照原版存"。Banks 还是空的（还没玩过）就只把开关打开：没有东西可收，
/// 第一份进度会在第一次存回时归成「原版」。关掉只是不再自动切，已经存下的组一个都不删。
pub fn set_isolation(root: &Path, banks: &Path, enabled: bool) -> Result<Isolation> {
    let mut state = isolation(root)?;
    if enabled && state.active.is_none() {
        // 还没玩过（Banks 不存在 / 是空的）就**没有可收的**，开关照常打开；
        // 第一份进度会在第一次存回时归成「原版」。
        // 其它失败（权限、磁盘满）照旧往上抛 —— 不能把「没存成」说成「存好了」。
        if !snapshot(banks)?.files.is_empty() {
            state.active = Some(backup(banks, root, ORIGINAL)?);
        }
    }
    state.enabled = enabled;
    write_isolation(root, &state)?;
    Ok(state)
}

/// 把一个组的内容换成**现在游戏里这份**（覆盖式）。
///
/// 先清掉旧内容再铺 —— 否则被删掉的档会留在组里，两份混在一起。
fn save_into_dir(banks: &Path, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }
    let set = snapshot(banks)?;
    for file in &set.files {
        std::fs::copy(banks.join(&file.name), dir.join(&file.name))?;
    }
    Ok(())
}

/// 把**现在游戏里这份** Banks 存回它所属的组。没归过档就先归到「原版」。
pub fn save_active(root: &Path, banks: &Path) -> Result<Isolation> {
    let mut state = isolation(root)?;
    let name = match state.active.clone() {
        Some(name) => name,
        None => {
            let name = backup(banks, root, ORIGINAL)?;
            state.active = Some(name.clone());
            name
        }
    };

    save_into_dir(banks, &self::root(root).join(&name))?;
    write_isolation(root, &state)?;
    Ok(state)
}

/// 把**现在游戏里这份** Banks 存进**指定的组**，并把它记为当前组。
///
/// 和 `save_active` 的区别：那个只会存回"当前组"，这个由用户点名 ——
/// 存档页上的「设为当前（把游戏里这份存回它）」就是它。
pub fn save_into(root: &Path, banks: &Path, name: &str) -> Result<Isolation> {
    let dir = self::root(root).join(name);
    if !dir.is_dir() {
        return Err(crate::error::Error::PackageRejected(format!(
            "没有这个存档组：{name}"
        )));
    }

    let mut state = isolation(root)?;
    state.active = Some(name.to_string());
    save_into_dir(banks, &dir)?;
    write_isolation(root, &state)?;
    Ok(state)
}

/// 切到某个组：**先把现在这份存回去**，再把目标组铺出来。
///
/// 目标组是空的（新建的战役组）=> Banks 会被清空，也就是"这个战役从头开始"。
/// 原来那份已经安全地存在它自己的组里了。
pub fn switch(root: &Path, banks: &Path, name: &str) -> Result<Isolation> {
    let dir = self::root(root).join(name);
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
    if state.active.is_some() || !snapshot(banks)?.files.is_empty() {
        state = save_active(root, banks)?;
    }

    // 铺目标组（内部会先给现在这份留后路，但我们已经存过了，这里是幂等的）
    restore(banks, root, name)?;
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

/// **迁移/自愈**：把 `active` 那个组的绑定修正到它真正代表的东西上。三种情况：
/// 1. 绑着**战役级键**（`lotv`）但那个槽位当前启用的是某个改版 -> 重绑到版本键；
/// 2. **完全没有绑定**、而某个槽位正用着原版 -> 绑到那个槽位键（唯一候选才动手，见
///    [`bind_unbound_active`]）；
/// 3. 其余（已经绑对了 / 没有 `active`）：原样返回。
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
    // 已经是「槽位/身份」这种版本级键：不用管
    if key.contains('/') {
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
fn fresh_group(root: &Path, label: &str) -> Result<String> {
    let name = dir_name(label);
    std::fs::create_dir_all(self::root(root).join(&name))?;
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

/// 存档页「归属」下拉 / 战役卡片的一个候选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SaveTarget {
    /// 写进 `state.json` 的隔离键。
    pub key: String,
    /// 界面上的说法（官方：`<战役名>（原版）` / `<战役名> · <版本名>`）。
    pub label: String,
    /// 这个战役 / 版本现在**装在游戏目录里**（按激活清单算，不看槽位选中项）。
    ///
    /// 官方改版最多一个为 true；**自制战役可以有好几个 true**。
    pub enabled: bool,
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
/// - **原版键**（战役级，不含斜杠）：先复用「原版」那个老组 —— 当年开隔离时收下的
///   那份就是原版进度，**不能**因为没绑定就新建一个空组把它晾在一边（用户会把
///   一份好好的原版存档看成「未分类」）。组名、文件都不动，只补一条绑定。
/// - **自制战役**：永远空组（每部各管各的，不继承）。
/// - **还没归过档**（刚升级上来 / 刚打开隔离，`active == None`）：现在游戏里这份就是
///   「我在玩的那部」，把它收成这个组的。切组前 `save_active` 还会先把同一份收进
///   「原版」，所以「原版」和这个组会**同时**有它 —— 刻意的，谁也没被顶掉。
/// - **其余（某个改版 / 自制）**：空组。这时 Banks 是**别的组**的进度，抄进新组就是
///   串档（用户抱怨的正是这个：改版是另一个游戏）。
///
/// 已经有组就照旧切过去。返回切到了哪个组（没开隔离返回 `None`）。
pub fn ensure_for_slot(
    root: &Path,
    banks: &Path,
    key: &str,
    label: &str,
) -> Result<Option<String>> {
    let state = isolation(root)?;
    if !state.enabled {
        return Ok(None);
    }

    if let Some(name) = profile_for(root, key)? {
        switch(root, banks, &name)?;
        return Ok(Some(name));
    }

    // 只有「还没归过档」才把当前这份收成新组；其余情况从空组开始
    let adopt_current = !is_custom_key(key) && state.active.is_none();
    let adopt = || -> Result<String> {
        // Banks 还没东西（第一次玩）时没得接，退回空组
        match backup(banks, root, label) {
            Ok(name) => Ok(name),
            Err(_) => fresh_group(root, label),
        }
    };
    let name = if is_campaign_key(key) {
        // 原版键：先复用老组（「原版」那个，或者任意还没归属的老组）——
        // 那份内容就是原版进度，别新建一个空组把它晾在一边
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
    switch(root, banks, &name)?;
    Ok(Some(name))
}

/// 战役级键（原版）：就是槽位本身，不带 `/`。
fn is_campaign_key(key: &str) -> bool {
    !is_custom_key(key) && !key.contains('/')
}

/// 切回原版时该用哪个组。
///
/// 优先级：**标签是「原版」的那个**（不管它现在绑着谁 —— 内容就是原版进度），
/// 其次任意一个还没归属的老组（`legacy`）。都没有才让调用方新建。
/// 只读：返回组名，绑定交给调用方。
fn original_group(root: &Path) -> Result<Option<String>> {
    let list = backups(root)?;
    if let Some(found) = list
        .iter()
        .find(|item| !item.auto && item.label == ORIGINAL)
    {
        return Ok(Some(found.name.clone()));
    }
    Ok(list
        .into_iter()
        .find(|item| item.legacy)
        .map(|item| item.name))
}

// ---------------------------------------------------------------------------
// 扫描**真机**的存档目录（Accounts/<账号>/<地域>/Saves 与 Banks）
// ---------------------------------------------------------------------------

/// 一个存档文件（`*.SC2Save`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SaveEntry {
    /// 账号目录名（`Accounts/<账号>` 那一层，通常是数字 id）。
    pub account: String,
    /// 账号下那一层（`<地域>-<账号>`），同一账号可能有多个。
    pub region: String,
    /// 层级：带上 `Saves` 前缀的相对路径（`Saves` / `Saves/Campaign` / `Saves/Unsaved/Campaign` …）。
    pub folder: String,
    /// 文件名，**不含扩展名**。
    pub name: String,
    /// 绝对路径。
    pub path: String,
    pub bytes: u64,
    /// 修改时间（Unix 秒）。
    pub modified: u64,
}

/// 一个 Banks 文件（`*.SC2Bank`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BankEntry {
    pub account: String,
    pub region: String,
    /// 地图 key：**父目录名**（`Banks/<地图 key>/*.SC2Bank`）；直接在 `Banks` 根下就是空串。
    pub map: String,
    /// 文件名，不含扩展名。
    pub name: String,
    pub path: String,
    pub bytes: u64,
    /// 修改时间（Unix 秒）。
    pub modified: u64,
}

/// 扫出来的全部存档。
#[derive(Debug, Clone, Default, Serialize)]
pub struct SaveInventory {
    /// 账号目录名（数字 id），**按最近活跃排序**。
    pub accounts: Vec<String>,
    /// 所有 `*.SC2Save`，按修改时间降序。
    pub saves: Vec<SaveEntry>,
    /// 所有 `*.SC2Bank`，按修改时间降序。
    pub banks: Vec<BankEntry>,
}

/// 递归时跳过的目录名：这些跟存档无关，收进来只会让界面变脏。
const SKIP_DIRS: [&str; 4] = ["Hotkeys", "Cache", "logs", "Logs"];

/// 扫一遍**真机**的存档位置。
///
/// 真机结构（2026-10 核实，scnexus 没做存档管理，没得抄）：
/// ```text
/// <Documents>/StarCraft II/Accounts/<账号>/<地域>/Saves/**/*.SC2Save
/// <Documents>/StarCraft II/Accounts/<账号>/<地域>/Banks/<地图 key>/*.SC2Bank
/// ```
/// 注意：**没有** `<Documents>/StarCraft II/Banks` 这个目录 —— 旧代码读的就是它，
/// 所以真机上永远是 0 个。别再加回去。
///
/// 读不到 / 目录不存在一律当空（不是错误）；`documents` 传 `Installation::documents_root`
/// （即用户文档目录下的 `StarCraft II`）。
pub fn scan(documents: &Path) -> SaveInventory {
    let mut inventory = SaveInventory::default();
    let accounts_root = documents.join("Accounts");
    let Ok(accounts) = std::fs::read_dir(&accounts_root) else {
        return inventory;
    };

    for account in accounts.flatten() {
        if !account.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let account_name = account.file_name().to_string_lossy().into_owned();
        let Ok(regions) = std::fs::read_dir(account.path()) else {
            continue;
        };
        for region in regions.flatten() {
            if !region.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            let region_name = region.file_name().to_string_lossy().into_owned();
            // 只看 Saves 和 Banks：别的目录（Hotkeys 之类）跟存档无关
            let saves_root = region.path().join("Saves");
            let banks_root = region.path().join("Banks");
            if !saves_root.is_dir() && !banks_root.is_dir() {
                continue;
            }
            scan_saves_dir(
                &saves_root,
                &account_name,
                &region_name,
                "Saves",
                &mut inventory.saves,
            );
            scan_banks_dir(
                &banks_root,
                &account_name,
                &region_name,
                "",
                &mut inventory.banks,
            );
            if !inventory.accounts.contains(&account_name) {
                inventory.accounts.push(account_name.clone());
            }
        }
    }

    // 新的在前
    inventory.saves.sort_by(|a, b| b.modified.cmp(&a.modified));
    inventory.banks.sort_by(|a, b| b.modified.cmp(&a.modified));
    // 账号按「它下面最新那份存档」排序：最近玩过的排前面
    let mut scored: Vec<(String, u64)> = inventory
        .accounts
        .iter()
        .map(|account| {
            let newest = inventory
                .saves
                .iter()
                .filter(|item| &item.account == account)
                .map(|item| item.modified)
                .chain(
                    inventory
                        .banks
                        .iter()
                        .filter(|item| &item.account == account)
                        .map(|item| item.modified),
                )
                .max()
                .unwrap_or(0);
            (account.clone(), newest)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    inventory.accounts = scored.into_iter().map(|(account, _)| account).collect();
    inventory
}

/// 递归收 `Saves` 下的 `*.SC2Save`；`folder` 是带前缀的相对路径。
fn scan_saves_dir(dir: &Path, account: &str, region: &str, folder: &str, out: &mut Vec<SaveEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if SKIP_DIRS
                .iter()
                .any(|skip| skip.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            scan_saves_dir(&path, account, region, &format!("{folder}/{name}"), out);
            continue;
        }
        let Some(stem) = save_stem(&path, "SC2Save") else {
            continue;
        };
        out.push(SaveEntry {
            account: account.to_string(),
            region: region.to_string(),
            folder: folder.to_string(),
            name: stem,
            path: path.to_string_lossy().into_owned(),
            bytes: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            modified: modified_secs(&entry),
        });
    }
}

/// 递归收 `Banks` 下的 `*.SC2Bank`；地图 key = **第一层目录名**（父目录）。
fn scan_banks_dir(dir: &Path, account: &str, region: &str, map: &str, out: &mut Vec<BankEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if SKIP_DIRS
                .iter()
                .any(|skip| skip.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            // 第一层子目录就是地图 key；再深也只记这一层
            let next = if map.is_empty() {
                name.clone()
            } else {
                map.to_string()
            };
            scan_banks_dir(&path, account, region, &next, out);
            continue;
        }
        let Some(stem) = save_stem(&path, "SC2Bank") else {
            continue;
        };
        out.push(BankEntry {
            account: account.to_string(),
            region: region.to_string(),
            map: map.to_string(),
            name: stem,
            path: path.to_string_lossy().into_owned(),
            bytes: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            modified: modified_secs(&entry),
        });
    }
}

/// 这个文件是不是 `.<extension>`；是就返回去掉扩展名的文件名。
fn save_stem(path: &Path, extension: &str) -> Option<String> {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
    {
        return None;
    }
    Some(path.file_stem()?.to_string_lossy().into_owned())
}

/// 文件的修改时间（Unix 秒）；取不到就是 0（排序会沉到后面，不影响别的）。
fn modified_secs(entry: &std::fs::DirEntry) -> u64 {
    entry
        .metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn duplicate_names_are_disambiguated_in_targets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        // 两个不同注册 ID（= 两个组）、名字和版本都一样的变体
        let library = library_with_custom(
            &data,
            &[("甲", "一样的名字", None), ("乙", "一样的名字", None)],
        );
        let targets = targets(&library, &[]);
        let labels: Vec<String> = targets
            .iter()
            .filter(|item| item.key.starts_with("custom/"))
            .map(|item| item.label.clone())
            .collect();
        assert_eq!(labels.len(), 2, "两个组各一条：{labels:?}");
        assert_ne!(labels[0], labels[1], "重名必须分得开：{labels:?}");
    }

    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(dir).expect("建目录");
        std::fs::write(dir.join(name), body).expect("写");
    }

    /// 直接把隔离开着写进 state.json（模拟"打开过开关、但还没归过档"）。
    ///
    /// 这几个用例要的是「Banks 里已经有进度、但还没有任何组」的状态；走
    /// `set_isolation(true)` 会先把它收成「原版」，正是这些用例要避开的。
    fn enable_isolation(root: &Path) {
        let state = Isolation {
            enabled: true,
            ..Default::default()
        };
        write_isolation(root, &state).expect("开隔离");
    }

    #[test]
    fn missing_banks_is_not_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let set = snapshot(&tmp.path().join("没有这个目录")).expect("快照");
        assert!(set.missing);
        assert!(set.files.is_empty());
    }

    #[test]
    fn backup_then_restore_round_trips() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");

        write(&banks, "ENS.SC2Bank", "first");
        let name = backup(&banks, &library, "复刻战役").expect("备份");
        assert!(name.starts_with("复刻战役-"), "目录名要带标签：{name}");

        write(&banks, "ENS.SC2Bank", "second");
        write(&banks, "新文件.SC2Bank", "junk");

        let safety = restore(&banks, &library, &name).expect("还原");
        assert_eq!(
            std::fs::read_to_string(banks.join("ENS.SC2Bank")).expect("读"),
            "first"
        );
        assert!(!banks.join("新文件.SC2Bank").exists(), "备份里没有的要清掉");
        assert!(safety.starts_with("还原前-"), "还原前要留后路：{safety}");
        assert_eq!(
            std::fs::read_to_string(root(&library).join(&safety).join("ENS.SC2Bank")).expect("读"),
            "second"
        );
    }

    #[test]
    fn listing_shows_labels() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "a.SC2Bank", "x");
        backup(&banks, &library, "第一部").expect("备份");

        let list = backups(&library).expect("列备份");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label, "第一部");
        assert_eq!(list[0].files, 1);
    }

    #[test]
    fn isolation_defaults_to_disabled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let library = tmp.path().join("data");

        // 没有 state.json 时：默认**关着**，什么归属都没有
        let state = isolation(&library).expect("读");
        assert!(
            !state.enabled,
            "没有 state.json 时默认关着（不替用户动 Banks）"
        );
        assert!(state.active.is_none(), "还没归过档");
        assert!(state.assignments.is_empty(), "也不该有归属");
    }

    #[test]
    fn opening_isolation_stores_the_current_progress_first() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "原版.SC2Bank", "playthrough");

        let state = set_isolation(&library, &banks, true).expect("打开");
        assert!(state.enabled);
        let active = state.active.clone().expect("要归过档");
        assert!(active.starts_with(ORIGINAL), "初始那份算原版：{active}");
        assert_eq!(
            std::fs::read_to_string(root(&library).join(&active).join("原版.SC2Bank")).expect("读"),
            "playthrough",
            "打开隔离不能把用户原有的进度弄丢"
        );
    }

    #[test]
    fn opening_isolation_without_any_saves_still_works() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");

        // 还没玩过（Banks 不存在）：打开开关不能被"没有存档可备份"挡回去
        let state = set_isolation(&library, &banks, true).expect("打开");
        assert!(state.enabled);
        assert!(state.active.is_none(), "没东西可收，先空着");
    }

    #[test]
    fn official_slot_without_record_inherits_the_current_saves() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "玩到第三章");
        enable_isolation(&library);

        // 没有记录时启用官方战役：当前这份收成它的组（不是空组）
        let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
            .expect("切换")
            .expect("开着隔离就该切");
        assert!(profile.contains("自由之翼"), "组名带战役名：{profile}");

        let group = snapshot(&root(&library).join(&profile)).expect("快照");
        assert_eq!(group.files.len(), 1, "新组不该是空的：{group:?}");
        assert_eq!(
            std::fs::read_to_string(root(&library).join(&profile).join("进度.SC2Bank"))
                .expect("读"),
            "玩到第三章",
            "没有记录时默认归当前启用的战役"
        );

        // 「原版」组照旧保留，而且是同一份 —— 刻意的：谁也没被顶掉
        let original = backups(&library)
            .expect("列")
            .into_iter()
            .find(|item| item.label == ORIGINAL)
            .expect("原版组要留着");
        assert_eq!(
            std::fs::read_to_string(root(&library).join(&original.name).join("进度.SC2Bank"))
                .expect("读"),
            "玩到第三章",
            "老用户升级后第一份进度不能被当成空组切走"
        );
    }

    #[test]
    fn custom_slot_without_record_gets_an_empty_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "原版进度");
        enable_isolation(&library);

        // 自制战役各自独立：没有记录时**不继承**当前存档
        let profile = ensure_for_slot(&library, &banks, "custom", "自制战役")
            .expect("切换")
            .expect("开着隔离就该切");
        assert!(
            snapshot(&banks).expect("快照").files.is_empty(),
            "自制战役该从零开始"
        );
        assert!(
            snapshot(&root(&library).join(&profile))
                .expect("快照")
                .files
                .is_empty(),
            "组本身也是空的"
        );

        // 原来那份还在「原版」里
        let original = backups(&library)
            .expect("列")
            .into_iter()
            .find(|item| item.label == ORIGINAL)
            .expect("原版组要留着");
        assert_eq!(
            std::fs::read_to_string(root(&library).join(&original.name).join("进度.SC2Bank"))
                .expect("读"),
            "原版进度"
        );
    }

    #[test]
    fn switching_swaps_the_saves_and_remembers_where_we_were() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "原版第一章");
        enable_isolation(&library);

        // 官方战役第一次启用：接着当前这份进度（见上一个用例）
        let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
            .expect("切换")
            .expect("开了隔离就该切");
        assert!(profile.contains("自由之翼"), "组名带战役名：{profile}");
        assert_eq!(
            std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
            "原版第一章"
        );
        assert_eq!(
            profile_for(&library, "wol").expect("查"),
            Some(profile.clone())
        );

        // 在这个战役里玩一会儿
        write(&banks, "进度.SC2Bank", "自由之翼第一章");

        // 切回原版：原版的档要回来，新战役这份要存好
        let original = isolation(&library)
            .expect("读")
            .assignments
            .get(&profile)
            .map(|_| ())
            .map(|_| ORIGINAL.to_string());
        let name = backups(&library)
            .expect("列")
            .into_iter()
            .find(|item| item.label == ORIGINAL)
            .expect("原版组")
            .name;
        switch(&library, &banks, &name).expect("切回原版");
        assert_eq!(
            std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
            "原版第一章"
        );
        let _ = original;

        // 再切回那个战役：它自己的进度还在
        switch(&library, &banks, &profile).expect("切回战役");
        assert_eq!(
            std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
            "自由之翼第一章"
        );
    }

    #[test]
    fn assigning_a_profile_steals_it_from_the_other_campaign() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let library = tmp.path().join("data");
        std::fs::create_dir_all(root(&library)).expect("建目录");

        assign(&library, "手头这份", Some("wol")).expect("指派");
        assign(&library, "手头这份", Some("hots")).expect("改派");
        assert_eq!(
            profile_for(&library, "hots").expect("查"),
            Some("手头这份".to_string())
        );
        assert_eq!(
            profile_for(&library, "wol").expect("查"),
            None,
            "一个战役只占一个组"
        );

        assign(&library, "手头这份", None).expect("摘掉");
        assert_eq!(profile_for(&library, "hots").expect("查"), None);
    }

    #[test]
    fn saving_into_a_named_group_targets_that_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "原版");
        enable_isolation(&library);
        let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
            .expect("切换")
            .expect("开着隔离就该切");

        // 现在游戏里换成另一份内容，把它存进那个组
        write(&banks, "进度.SC2Bank", "最新进度");
        write(&banks, "新档.SC2Bank", "新");
        let state = save_into(&library, &banks, &profile).expect("存进去");
        assert_eq!(state.active.as_deref(), Some(profile.as_str()));
        assert_eq!(
            std::fs::read_to_string(root(&library).join(&profile).join("进度.SC2Bank"))
                .expect("读"),
            "最新进度"
        );
        assert!(
            root(&library).join(&profile).join("新档.SC2Bank").is_file(),
            "新文件也要进去"
        );

        // 不存在的组要报错，别悄悄建一个
        assert!(save_into(&library, &banks, "没有这个组").is_err());
    }

    #[test]
    fn default_off_leaves_banks_and_state_alone() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "玩到第三章");
        let before = std::fs::read(banks.join("进度.SC2Bank")).expect("读");

        // 用户没打开隔离（没有 state.json）：启用战役一律不碰 Banks
        assert_eq!(
            ensure_for_slot(&library, &banks, "wol", "自由之翼").expect("调"),
            None,
            "默认关着就不该切档"
        );
        assert_eq!(
            std::fs::read(banks.join("进度.SC2Bank")).expect("读"),
            before,
            "Banks 一个字节都不能动"
        );
        assert_eq!(
            snapshot(&banks).expect("快照").files.len(),
            1,
            "文件不多不少"
        );
        assert!(
            !state_file(&library).exists(),
            "默认关着时连 state.json 都不写"
        );
        assert!(backups(&library).expect("列").is_empty(), "也不该凭空建组");
    }

    /// 造一个库，里面只有一个自制槽位、若干版本。
    fn library_with_custom(data: &Path, variants: &[(&str, &str, Option<&str>)]) -> Library {
        let library = Library::new(data);
        let mut index = crate::library::LibraryIndex::default();
        let slot = crate::library::CampaignSlot {
            variants: variants
                .iter()
                .map(|(id, name, registration)| crate::library::Variant {
                    id: id.to_string(),
                    name: name.to_string(),
                    registration_id: registration.map(str::to_string),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        index.slots.insert("custom".to_string(), slot);
        library.save_index(&index).expect("写索引");
        library
    }

    #[test]
    fn vanilla_uses_the_slot_key_and_each_variant_gets_its_own() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let library = library_with_custom(&tmp.path().join("data"), &[]);

        // 原版：键就是槽位（老数据 `lotv` / `wol` 语义不变，不用迁移）
        assert_eq!(isolation_key(&library, "wol", None), "wol");
        assert_eq!(isolation_key(&library, "lotv", None), "lotv");
        // 有版本（这里库是空的，退回版本目录名）：槽位/身份
        assert_eq!(isolation_key(&library, "lotv", Some("肉鸽")), "lotv/肉鸽");
    }

    #[test]
    fn two_custom_campaigns_get_separate_groups() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        let library = library_with_custom(&data, &[("甲", "战役甲", None), ("乙", "战役乙", None)]);
        set_isolation(&data, &banks, true).expect("开隔离");

        let a = ensure_for_slot(
            &data,
            &banks,
            &isolation_key(&library, "custom", Some("甲")),
            "战役甲",
        )
        .expect("切甲")
        .expect("开着");
        let b = ensure_for_slot(
            &data,
            &banks,
            &isolation_key(&library, "custom", Some("乙")),
            "战役乙",
        )
        .expect("切乙")
        .expect("开着");

        assert_ne!(a, b, "两部自制战役必须是两个组：{a} / {b}");
        assert_eq!(
            profile_for(&data, "custom/甲").expect("查"),
            Some(a.clone())
        );
        assert_eq!(
            profile_for(&data, "custom/乙").expect("查"),
            Some(b.clone())
        );

        // 各自的组是空的（自制战役不继承当前存档）
        assert!(
            snapshot(&root(&data).join(&a))
                .expect("快照")
                .files
                .is_empty()
        );
        assert!(
            snapshot(&root(&data).join(&b))
                .expect("快照")
                .files
                .is_empty()
        );

        // 名字不撞（组名带战役名）
        assert!(a.contains("战役甲"), "{a}");
        assert!(b.contains("战役乙"), "{b}");
    }

    #[test]
    fn versions_of_one_custom_campaign_share_a_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        // 同一部战役的两个版本：包内声明的注册 ID 相同（大小写不同也算同一个）
        let library = library_with_custom(
            &data,
            &[
                ("v1", "战役 1.0", Some("Team.Campaign")),
                ("v2", "战役 2.0", Some("team.campaign")),
            ],
        );

        set_isolation(&data, &banks, true).expect("开隔离");
        let key1 = isolation_key(&library, "custom", Some("v1"));
        let key2 = isolation_key(&library, "custom", Some("v2"));
        assert_eq!(key1, key2, "注册 ID 一样就得是同一个键");

        let first = ensure_for_slot(&data, &banks, &key1, "战役 1.0")
            .expect("切 1.0")
            .expect("开着");
        let second = ensure_for_slot(&data, &banks, &key2, "战役 2.0")
            .expect("切 2.0")
            .expect("开着");
        assert_eq!(first, second, "换版本要共用同一个组");
        assert_eq!(backups(&data).expect("列").len(), 1, "不该多出一个组");
    }

    #[test]
    fn legacy_custom_assignment_is_kept_but_never_auto_migrated() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_custom(&data, &[("甲", "战役甲", None)]);

        // 老数据：组存在，归属是光秃秃的 `custom`
        let old = root(&data).join("旧的自制战役-1");
        std::fs::create_dir_all(&old).expect("建组");
        std::fs::write(old.join("旧.SC2Bank"), "老进度").expect("写");
        let mut state = Isolation::default();
        state
            .assignments
            .insert("旧的自制战役-1".to_string(), "custom".to_string());
        write_isolation(&data, &state).expect("写状态");

        // 新键不会匹配到它 —— 不猜版本
        assert_eq!(
            profile_for(&data, &isolation_key(&library, "custom", Some("甲"))).expect("查"),
            None,
            "绝不自动把老组迁到某个自制版本"
        );
        // 组本身还在（不丢）
        assert!(old.is_dir());
        assert_eq!(
            std::fs::read_to_string(old.join("旧.SC2Bank")).expect("读"),
            "老进度"
        );

        // 候选里不会冒出这个老键（界面会单独标成「旧的自制战役（未指定版本）」）
        assert!(
            !targets(&library, &[])
                .iter()
                .any(|item| item.key == "custom")
        );

        // 用户手动指过去：一次 assign 就归到新键上
        assign(&data, "旧的自制战役-1", Some("custom/甲")).expect("手改");
        assert_eq!(
            profile_for(&data, "custom/甲").expect("查"),
            Some("旧的自制战役-1".to_string())
        );
    }

    #[test]
    fn targets_list_official_vanilla_then_each_version_then_custom() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_custom(
            &data,
            &[
                ("v1", "战役 1.0", Some("Team.Campaign")),
                ("v2", "战役 2.0", Some("team.campaign")),
                ("other", "另一部", None),
            ],
        );

        let list = targets(&library, &[]);
        let keys: Vec<&str> = list.iter().map(|item| item.key.as_str()).collect();

        // 官方每个槽位先「原版」（键 = 槽位），没有改版时只有这一条
        assert_eq!(
            &keys[..4],
            &["wol", "hots", "lotv", "nova"],
            "官方按固定菜单顺序，每个槽位先原版"
        );
        assert_eq!(list[0].label, "自由之翼（原版）");
        // 自制：同一部（共享注册 ID）只列一条，另一部单独一条
        assert_eq!(keys[4], "custom/team.campaign", "同一部只列一条");
        assert_eq!(keys[5], "custom/other");
        assert_eq!(list[4].label, "自制战役 · 战役 1.0");
    }

    #[test]
    fn targets_include_official_variants_after_the_vanilla_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = Library::new(&data);
        let mut index = crate::library::LibraryIndex::default();
        index.slots.insert(
            "lotv".to_string(),
            crate::library::CampaignSlot {
                variants: vec![
                    crate::library::Variant {
                        id: "肉鸽".to_string(),
                        name: "肉鸽".to_string(),
                        ..Default::default()
                    },
                    crate::library::Variant {
                        id: "另一个改版".to_string(),
                        name: "另一个改版".to_string(),
                        ..Default::default()
                    },
                ],
                active: None,
            },
        );
        library.save_index(&index).expect("写索引");

        let list = targets(&library, &[]);
        let keys: Vec<&str> = list.iter().map(|item| item.key.as_str()).collect();
        // 官方顺序：wol（原版）、hots（原版）、lotv（原版 + 两个改版）、nova（原版）
        assert_eq!(
            &keys[..6],
            &[
                "wol",
                "hots",
                "lotv",
                "lotv/肉鸽",
                "lotv/另一个改版",
                "nova"
            ],
            "每个槽位先原版、再跟上它的版本：{keys:?}"
        );
        assert_eq!(list[2].label, "虚空之遗（原版）");
        assert_eq!(list[3].label, "虚空之遗 · 肉鸽");
    }

    #[test]
    fn custom_target_label_prefers_the_active_version_then_the_newest() {
        // label 不能随遍历顺序漂：启用的那版优先，都没启用就取导入时间最新的
        fn variant(id: &str, name: &str, imported_at: u64) -> crate::library::Variant {
            crate::library::Variant {
                id: id.to_string(),
                name: name.to_string(),
                registration_id: Some("team.campaign".to_string()),
                imported_at,
                ..Default::default()
            }
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = Library::new(&data);
        let mut index = crate::library::LibraryIndex::default();
        index.slots.insert(
            "custom".to_string(),
            crate::library::CampaignSlot {
                variants: vec![
                    variant("new", "战役 2.0", 20),
                    variant("old", "战役 1.0", 10),
                ],
                active: None,
            },
        );
        library.save_index(&index).expect("写索引");

        // 都没启用 -> 取最新的那版
        let list = targets(&library, &[]);
        assert_eq!(list[4].label, "自制战役 · 战役 2.0");
        assert_eq!(list.len(), 5, "同一部只列一条");

        // 启用了旧那版 -> 取启用的那版（哪怕它不是最新）
        let mut index = library.index();
        index.slots.get_mut("custom").expect("槽位").active = Some("old".to_string());
        library.save_index(&index).expect("写索引");
        let list = targets(&library, &[]);
        assert_eq!(list[4].label, "自制战役 · 战役 1.0");
    }

    #[test]
    fn group_files_lists_one_group_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        write(&root(&data).join("甲"), "a.SC2Bank", "1");
        write(&root(&data).join("甲"), "b.SC2Bank", "22");
        write(&root(&data).join("乙"), "c.SC2Bank", "333");

        let files = group_files(&data, "甲").expect("列甲");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "a.SC2Bank");
        assert_eq!(files[0].bytes, 1);
        assert_eq!(files[1].name, "b.SC2Bank");
        assert_eq!(files[1].bytes, 2);

        // 空组是正常状态
        std::fs::create_dir_all(root(&data).join("空")).expect("建组");
        assert!(group_files(&data, "空").expect("列空组").is_empty());

        // 没有这个组要说清楚是哪一组
        let err = group_files(&data, "查无此组").expect_err("该报错");
        assert!(format!("{err}").contains("查无此组"), "{err}");
    }

    #[test]
    fn moving_a_save_file_moves_it_between_groups_and_never_touches_banks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        // 游戏目录那份 Banks 也放一个**同名**文件：绝不能被碰
        let banks = tmp.path().join("Banks");
        write(&banks, "a.SC2Bank", "游戏里那份");
        write(&root(&data).join("甲"), "a.SC2Bank", "进度");
        write(&root(&data).join("乙"), "b.SC2Bank", "别的");

        move_save_file(&data, "甲", "a.SC2Bank", "乙").expect("挪");

        assert!(
            !root(&data).join("甲").join("a.SC2Bank").exists(),
            "源组里没了"
        );
        assert_eq!(
            std::fs::read_to_string(root(&data).join("乙").join("a.SC2Bank")).expect("读"),
            "进度"
        );
        assert!(root(&data).join("甲").is_dir(), "源组空了也不删组");
        assert_eq!(
            std::fs::read_to_string(banks.join("a.SC2Bank")).expect("读"),
            "游戏里那份",
            "只动 data/saves，绝不碰 Banks"
        );
    }

    #[test]
    fn moving_refuses_to_overwrite_a_same_named_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        write(&root(&data).join("甲"), "a.SC2Bank", "旧的");
        write(&root(&data).join("乙"), "a.SC2Bank", "新的");

        let err = move_save_file(&data, "甲", "a.SC2Bank", "乙").expect_err("必须拒绝");
        assert!(format!("{err}").contains("同名"), "{err}");

        // 两边都没被动过
        assert_eq!(
            std::fs::read_to_string(root(&data).join("甲").join("a.SC2Bank")).expect("读"),
            "旧的"
        );
        assert_eq!(
            std::fs::read_to_string(root(&data).join("乙").join("a.SC2Bank")).expect("读"),
            "新的"
        );
    }

    #[test]
    fn moving_creates_the_target_group_and_rejects_path_like_names() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        write(&root(&data).join("甲"), "b.SC2Bank", "内容");

        // 目标组不存在就建
        move_save_file(&data, "甲", "b.SC2Bank", "新建的组").expect("目标组自动建");
        assert_eq!(
            std::fs::read_to_string(root(&data).join("新建的组").join("b.SC2Bank")).expect("读"),
            "内容"
        );

        // 文件名 / 组名都不许带路径（跳出 data/saves 的一道闸）
        write(&root(&data).join("甲"), "c.SC2Bank", "x");
        assert!(move_save_file(&data, "甲", "../c.SC2Bank", "新建的组").is_err());
        assert!(move_save_file(&data, "..", "c.SC2Bank", "新建的组").is_err());
        assert!(move_save_file(&data, "甲", "c.SC2Bank", "..").is_err());
    }

    #[test]
    fn official_vanilla_and_mod_versions_get_separate_groups() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        write(&banks, "原版进度.SC2Bank", "玩到第三章");
        let library = library_with_custom(&data, &[]);
        set_isolation(&data, &banks, true).expect("开隔离");

        // 原版一份（键 = 槽位），改版一份（键 = 槽位/身份）
        let vanilla = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
            .expect("切原版")
            .expect("开着");
        let modded = ensure_for_slot(&data, &banks, "lotv/肉鸽", "虚空之遗 · 肉鸽")
            .expect("切改版")
            .expect("开着");

        assert_ne!(vanilla, modded, "原版和改版必须是两个组");
        assert_eq!(
            profile_for(&data, "lotv").expect("查"),
            Some(vanilla.clone())
        );
        assert_eq!(
            profile_for(&data, "lotv/肉鸽").expect("查"),
            Some(modded.clone())
        );
        // 原版组有那份进度（第一次启用时收进去的），改版组是空的（不串档）
        assert_eq!(
            std::fs::read_to_string(root(&data).join(&vanilla).join("原版进度.SC2Bank"))
                .expect("读"),
            "玩到第三章"
        );
        assert!(
            snapshot(&root(&data).join(&modded))
                .expect("快照")
                .files
                .is_empty(),
            "已经有组在用的时候，改版从空组开始，别把原版进度抄过来"
        );
        let _ = &library;
    }

    #[test]
    fn official_mod_versions_sharing_registration_share_a_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        let library = Library::new(&data);
        let mut index = crate::library::LibraryIndex::default();
        index.slots.insert(
            "lotv".to_string(),
            crate::library::CampaignSlot {
                variants: vec![
                    crate::library::Variant {
                        id: "肉鸽 v1".to_string(),
                        name: "肉鸽 1.0".to_string(),
                        registration_id: Some("Team.Rogue".to_string()),
                        imported_at: 10,
                        ..Default::default()
                    },
                    crate::library::Variant {
                        id: "肉鸽 v2".to_string(),
                        name: "肉鸽 2.0".to_string(),
                        registration_id: Some("team.rogue".to_string()),
                        imported_at: 20,
                        ..Default::default()
                    },
                ],
                active: None,
            },
        );
        library.save_index(&index).expect("写索引");

        set_isolation(&data, &banks, true).expect("开隔离");
        let key1 = isolation_key(&library, "lotv", Some("肉鸽 v1"));
        let key2 = isolation_key(&library, "lotv", Some("肉鸽 v2"));
        assert_eq!(key1, key2, "同一部改版换版本 -> 同一个键");

        let first = ensure_for_slot(&data, &banks, &key1, "虚空之遗 · 肉鸽 1.0")
            .expect("切 1.0")
            .expect("开着");
        let second = ensure_for_slot(&data, &banks, &key2, "虚空之遗 · 肉鸽 2.0")
            .expect("切 2.0")
            .expect("开着");
        assert_eq!(first, second, "换版本要共用同一个组");
        assert_eq!(backups(&data).expect("列").len(), 1, "不该多出一个组");
        // 原版那个键没被牵连
        assert_eq!(profile_for(&data, "lotv").expect("查"), None);
    }

    #[test]
    fn two_different_official_mods_get_two_groups() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        let library = Library::new(&data);
        let mut index = crate::library::LibraryIndex::default();
        index.slots.insert(
            "lotv".to_string(),
            crate::library::CampaignSlot {
                variants: vec![
                    crate::library::Variant {
                        id: "肉鸽".to_string(),
                        name: "肉鸽".to_string(),
                        ..Default::default()
                    },
                    crate::library::Variant {
                        id: "残卷".to_string(),
                        name: "残卷".to_string(),
                        ..Default::default()
                    },
                ],
                active: None,
            },
        );
        library.save_index(&index).expect("写索引");

        set_isolation(&data, &banks, true).expect("开隔离");
        let a = ensure_for_slot(
            &data,
            &banks,
            &isolation_key(&library, "lotv", Some("肉鸽")),
            "虚空之遗 · 肉鸽",
        )
        .expect("切肉鸽")
        .expect("开着");
        let b = ensure_for_slot(
            &data,
            &banks,
            &isolation_key(&library, "lotv", Some("残卷")),
            "虚空之遗 · 残卷",
        )
        .expect("切残卷")
        .expect("开着");
        assert_ne!(a, b, "两部不同改版必须是两个组：{a} / {b}");
        assert!(a.contains("肉鸽"), "{a}");
        assert!(b.contains("残卷"), "{b}");
    }

    #[test]
    fn restore_safety_backup_is_marked_auto() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        write(&banks, "进度.SC2Bank", "原来的");

        let manual = backup(&banks, &data, "手动备份").expect("备份");
        write(&banks, "进度.SC2Bank", "新的");
        let safety = restore(&banks, &data, &manual).expect("还原");
        assert!(safety.starts_with(AUTO_BACKUP), "还原前留的那份：{safety}");

        let list = backups(&data).expect("列");
        let auto: Vec<&str> = list
            .iter()
            .filter(|item| item.auto)
            .map(|item| item.name.as_str())
            .collect();
        let groups: Vec<&str> = list
            .iter()
            .filter(|item| !item.auto)
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(auto, vec![safety.as_str()], "「还原前」要标成自动备份");
        assert_eq!(groups, vec![manual.as_str()], "组列表里只留用户自己的组");
    }

    /// 造一个只有 lotv 槽位（带一个「肉鸽」改版）的库；`active` 是这个槽位当前启用的版本。
    fn library_with_lotv_mod_active(data: &Path, active: Option<&str>) -> Library {
        let library = Library::new(data);
        let mut index = crate::library::LibraryIndex::default();
        index.slots.insert(
            "lotv".to_string(),
            crate::library::CampaignSlot {
                variants: vec![crate::library::Variant {
                    id: "肉鸽".to_string(),
                    name: "肉鸽".to_string(),
                    ..Default::default()
                }],
                active: active.map(str::to_string),
            },
        );
        library.save_index(&index).expect("写索引");
        library
    }

    fn library_with_lotv_mod(data: &Path) -> Library {
        library_with_lotv_mod_active(data, None)
    }

    /// 造一份隔离状态：`active` 指着 `name` 那个组，那个组绑在 `key` 上。
    fn state_with(active: Option<&str>, name: &str, key: &str) -> Isolation {
        let mut state = Isolation {
            active: active.map(str::to_string),
            ..Default::default()
        };
        state.assignments.insert(name.to_string(), key.to_string());
        state
    }

    #[test]
    fn move_save_to_key_uses_the_existing_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let _library = library_with_lotv_mod(&data);
        write(&root(&data).join("旧组"), "a.SC2Bank", "进度");
        std::fs::create_dir_all(root(&data).join("现有组")).expect("建组");
        assign(&data, "现有组", Some("lotv/肉鸽")).expect("绑定");

        let moved = move_save_to_key(&data, "旧组", "a.SC2Bank", "lotv/肉鸽").expect("挪");
        assert_eq!(moved, "现有组", "已有组就用它，不该再造一个");
        assert_eq!(
            std::fs::read_to_string(root(&data).join("现有组").join("a.SC2Bank")).expect("读"),
            "进度"
        );
        assert_eq!(backups(&data).expect("列").len(), 2, "只有旧组和现有组");
    }

    #[test]
    fn move_save_to_key_creates_and_binds_a_new_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let _library = library_with_lotv_mod(&data);
        write(&root(&data).join("旧组"), "a.SC2Bank", "进度");

        let moved = move_save_to_key(&data, "旧组", "a.SC2Bank", "lotv/肉鸽").expect("挪");
        // 组名和启用时造组一致「战役名 · 版本名」（空格会被折成 _）
        assert!(moved.starts_with("虚空之遗"), "{moved}");
        assert!(moved.contains("肉鸽"), "{moved}");
        assert_eq!(
            profile_for(&data, "lotv/肉鸽").expect("查"),
            Some(moved.clone()),
            "新组要自动绑定到这个键"
        );
        assert_eq!(
            std::fs::read_to_string(root(&data).join(&moved).join("a.SC2Bank")).expect("读"),
            "进度"
        );
        assert!(
            !root(&data).join("旧组").join("a.SC2Bank").exists(),
            "源组里没了"
        );
        assert!(root(&data).join("旧组").is_dir(), "源组空了也不删");
    }

    #[test]
    fn move_save_to_key_refuses_same_name_and_unknown_keys() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let _library = library_with_lotv_mod(&data);
        write(&root(&data).join("甲"), "a.SC2Bank", "旧的");
        std::fs::create_dir_all(root(&data).join("乙")).expect("建组");
        assign(&data, "乙", Some("lotv/肉鸽")).expect("绑定");
        write(&root(&data).join("乙"), "a.SC2Bank", "新的");

        let err = move_save_to_key(&data, "甲", "a.SC2Bank", "lotv/肉鸽").expect_err("必须拒绝");
        assert!(format!("{err}").contains("同名"), "{err}");
        assert_eq!(
            std::fs::read_to_string(root(&data).join("甲").join("a.SC2Bank")).expect("读"),
            "旧的"
        );

        // 不认识的键：不能凭空造一个名字都说不清的组
        let err =
            move_save_to_key(&data, "甲", "a.SC2Bank", "lotv/不存在的版本").expect_err("要拒绝");
        assert!(format!("{err}").contains("未知的归属"), "{err}");
        assert_eq!(backups(&data).expect("列").len(), 2, "只该有甲和乙两个组");
    }

    #[test]
    fn move_save_to_key_never_touches_banks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let _library = library_with_lotv_mod(&data);
        // 游戏目录那份也放一份同名文件：挪动绝不能被它碰到
        let banks = tmp.path().join("Banks");
        write(&banks, "a.SC2Bank", "游戏里那份");
        write(&root(&data).join("旧组"), "a.SC2Bank", "进度");

        move_save_to_key(&data, "旧组", "a.SC2Bank", "lotv/肉鸽").expect("挪");
        assert_eq!(
            std::fs::read_to_string(banks.join("a.SC2Bank")).expect("读"),
            "游戏里那份",
            "只动 data/saves，绝不碰 Banks"
        );
    }

    #[test]
    fn reconcile_rebinds_a_campaign_level_group_to_the_active_version() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        // 槽位当前启用的是改版「肉鸽」，但老数据把组绑在战役级键 `lotv` 上
        let library = library_with_lotv_mod_active(&data, Some("肉鸽"));
        let group = root(&data).join("虚空之遗-1");
        std::fs::create_dir_all(&group).expect("建组");
        std::fs::write(group.join("a.SC2Bank"), "进度").expect("写");
        let state = state_with(Some("虚空之遗-1"), "虚空之遗-1", "lotv");
        write_isolation(&data, &state).expect("写状态");

        assert!(
            reconcile_active_binding(&library, &data).expect("迁移"),
            "该重绑到版本级键"
        );
        let state = isolation(&data).expect("读");
        assert_eq!(
            state.assignments.get("虚空之遗-1").map(String::as_str),
            Some("lotv/肉鸽")
        );
        assert_eq!(state.active.as_deref(), Some("虚空之遗-1"), "active 不动");
        // 组名和文件都不动
        assert!(group.is_dir());
        assert_eq!(
            std::fs::read_to_string(group.join("a.SC2Bank")).expect("读"),
            "进度"
        );
    }

    #[test]
    fn reconcile_leaves_vanilla_alone() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        // 槽位正用着原版：战役级键 `lotv` 本来就是对的
        let library = library_with_lotv_mod(&data);
        let state = state_with(Some("虚空之遗-1"), "虚空之遗-1", "lotv");
        write_isolation(&data, &state).expect("写状态");

        assert!(
            !reconcile_active_binding(&library, &data).expect("迁移"),
            "原版不该动"
        );
        let state = isolation(&data).expect("读");
        assert_eq!(
            state.assignments.get("虚空之遗-1").map(String::as_str),
            Some("lotv")
        );
    }

    #[test]
    fn reconcile_is_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_lotv_mod_active(&data, Some("肉鸽"));

        // 已经绑在版本级键上：不动
        let state = state_with(Some("甲"), "甲", "lotv/肉鸽");
        write_isolation(&data, &state).expect("写状态");
        assert!(!reconcile_active_binding(&library, &data).expect("迁移"));

        // 没有 active / 没有绑定：也不动（老组交给 legacy 标签处理）
        write_isolation(&data, &Isolation::default()).expect("写状态");
        assert!(!reconcile_active_binding(&library, &data).expect("迁移"));

        // 真的重绑一次之后再跑：什么都不做
        let state = state_with(Some("乙"), "乙", "lotv");
        write_isolation(&data, &state).expect("写状态");
        assert!(reconcile_active_binding(&library, &data).expect("迁移"));
        assert!(
            !reconcile_active_binding(&library, &data).expect("迁移"),
            "第二次必须无动作"
        );
    }

    #[test]
    fn unbound_groups_are_marked_legacy() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        write(&root(&data).join("原版-1"), "a.SC2Bank", "老进度");

        let entry = backups(&data).expect("列").pop().expect("有组");
        assert!(entry.legacy, "没有绑定的组要标 legacy（名字不代表内容）");
        assert!(!entry.auto);

        // 绑上某个战役之后就不再是 legacy
        assign(&data, "原版-1", Some("wol")).expect("绑定");
        let entry = backups(&data).expect("列").pop().expect("有组");
        assert!(!entry.legacy);

        // 自动备份不算 legacy（它另有一套 auto 规则）
        let banks = tmp.path().join("Banks");
        write(&banks, "a.SC2Bank", "进度");
        let safety = backup(&banks, &data, AUTO_BACKUP).expect("自动备份");
        let entry = backups(&data)
            .expect("列")
            .into_iter()
            .find(|item| item.name == safety)
            .expect("有自动备份");
        assert!(entry.auto);
        assert!(!entry.legacy);
    }

    #[test]
    fn vanilla_slot_reuses_the_legacy_original_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        // 只有一个 lotv 槽位、正用着原版
        let _library = library_with_lotv_mod_active(&data, None);
        // 当年开隔离时自动收下的老组：有内容、没有任何绑定
        write(&root(&data).join("原版-1"), "a.SC2Bank", "原版进度");
        set_isolation(&data, &banks, true).expect("开隔离");

        let name = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
            .expect("切原版")
            .expect("开着");
        assert_eq!(name, "原版-1", "要复用老组，别新建空组");
        assert_eq!(
            profile_for(&data, "lotv").expect("查"),
            Some("原版-1".to_string()),
            "复用的同时要把它绑到槽位键上"
        );
        assert_eq!(
            std::fs::read_to_string(root(&data).join("原版-1").join("a.SC2Bank")).expect("读"),
            "原版进度"
        );
        // 切组时 restore() 会留一份「还原前」的自动备份，所以这里只数组
        assert_eq!(
            backups(&data)
                .expect("列")
                .iter()
                .filter(|item| !item.auto)
                .count(),
            1,
            "不该多出一个组"
        );

        // 再走一次：还是同一个组（幂等）
        let again = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
            .expect("再切")
            .expect("开着");
        assert_eq!(again, "原版-1");
        assert_eq!(
            backups(&data)
                .expect("列")
                .iter()
                .filter(|item| !item.auto)
                .count(),
            1
        );
    }

    #[test]
    fn vanilla_slot_without_legacy_creates_a_group() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let banks = tmp.path().join("Banks");
        let _library = library_with_lotv_mod_active(&data, None);
        set_isolation(&data, &banks, true).expect("开隔离");

        let name = ensure_for_slot(&data, &banks, "lotv", "虚空之遗")
            .expect("切原版")
            .expect("开着");
        assert!(name.starts_with("虚空之遗"), "{name}");
        assert_eq!(profile_for(&data, "lotv").expect("查"), Some(name));
    }

    #[test]
    fn reconcile_binds_an_unbound_active_group_to_the_only_vanilla_slot() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_lotv_mod_active(&data, None);
        // active 指着当年收下的老组，它没有任何绑定
        write(&root(&data).join("原版-1"), "a.SC2Bank", "原版进度");
        write_isolation(
            &data,
            &Isolation {
                active: Some("原版-1".to_string()),
                ..Default::default()
            },
        )
        .expect("写状态");

        assert!(
            reconcile_active_binding(&library, &data).expect("迁移"),
            "唯一一个原版槽位 + 组没绑定 -> 该绑上"
        );
        assert_eq!(
            profile_for(&data, "lotv").expect("查"),
            Some("原版-1".to_string())
        );
        assert!(
            !reconcile_active_binding(&library, &data).expect("再迁移"),
            "第二次必须无动作"
        );
    }

    #[test]
    fn reconcile_does_not_guess_when_several_slots_are_vanilla() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        // 两个官方槽位都正用着原版：挑哪个都是猜 -> 不动
        let library = Library::new(&data);
        let mut index = crate::library::LibraryIndex::default();
        for slot in ["wol", "lotv"] {
            index
                .slots
                .insert(slot.to_string(), crate::library::CampaignSlot::default());
        }
        library.save_index(&index).expect("写索引");

        write(&root(&data).join("原版-1"), "a.SC2Bank", "原版进度");
        write_isolation(
            &data,
            &Isolation {
                active: Some("原版-1".to_string()),
                ..Default::default()
            },
        )
        .expect("写状态");

        assert!(!reconcile_active_binding(&library, &data).expect("迁移"));
        assert_eq!(profile_for(&data, "wol").expect("查"), None);
        assert_eq!(profile_for(&data, "lotv").expect("查"), None);
    }

    #[test]
    fn two_installed_custom_campaigns_are_both_enabled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_custom(&data, &[("甲", "战役甲", None), ("乙", "战役乙", None)]);
        let installed = vec![
            ("custom".to_string(), "甲".to_string()),
            ("custom".to_string(), "乙".to_string()),
        ];

        let list = targets(&library, &installed);
        let custom: Vec<&SaveTarget> = list
            .iter()
            .filter(|item| item.key.starts_with("custom/"))
            .collect();
        assert_eq!(custom.len(), 2);
        assert!(
            custom.iter().all(|item| item.enabled),
            "两部自制都装着 -> 两个 true"
        );
    }

    #[test]
    fn only_the_installed_custom_campaign_is_enabled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_custom(&data, &[("甲", "战役甲", None), ("乙", "战役乙", None)]);
        let installed = vec![("custom".to_string(), "甲".to_string())];

        let list = targets(&library, &installed);
        let enabled: Vec<&str> = list
            .iter()
            .filter(|item| item.key.starts_with("custom/") && item.enabled)
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(enabled, vec!["自制战役 · 战役甲"]);
    }

    #[test]
    fn installed_official_variant_disables_the_vanilla_target() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_lotv_mod_active(&data, None);
        let key = isolation_key(&library, "lotv", Some("肉鸽"));

        let list = targets(&library, &[("lotv".to_string(), "肉鸽".to_string())]);
        assert!(
            list.iter()
                .find(|item| item.key == key)
                .expect("有改版")
                .enabled,
            "装着的改版要算启用"
        );
        assert!(
            !list
                .iter()
                .find(|item| item.key == "lotv")
                .expect("有原版")
                .enabled,
            "装了改版，原版就不算装着"
        );
    }

    #[test]
    fn uninstalled_targets_are_disabled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("data");
        let library = library_with_lotv_mod_active(&data, None);

        // 什么都没装：改版全 false，原版全 true
        let list = targets(&library, &[]);
        assert!(
            list.iter()
                .filter(|item| item.key.contains('/'))
                .all(|item| !item.enabled)
        );
        assert!(
            list.iter()
                .filter(|item| !item.key.contains('/'))
                .all(|item| item.enabled)
        );
    }

    /// 改文件的修改时间（排序测试用）。
    fn set_mtime(path: &Path, secs: u64) {
        let file = std::fs::File::options()
            .append(true)
            .open(path)
            .expect("打开");
        file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .expect("改时间");
    }

    #[test]
    fn scan_reads_the_real_account_layout() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let docs = tmp.path().join("StarCraft II");
        let account = docs.join("Accounts/12345/美服-12345");

        // Saves：顶层 + Campaign + Unsaved/Campaign 都要收到
        write(&account.join("Saves"), "top.SC2Save", "save");
        write(&account.join("Saves/Campaign"), "campaign.SC2Save", "save");
        write(
            &account.join("Saves/Unsaved/Campaign"),
            "unsaved.SC2Save",
            "save",
        );
        // 干扰项：别的扩展名、别的目录（Hotkeys）
        write(&account.join("Saves"), "notes.txt", "x");
        write(&account.join("Hotkeys"), "hot.SC2Save", "x");
        write(&account.join("Saves/Hotkeys"), "hot2.SC2Save", "x");

        // Banks：按地图 key 分子目录；再深一层仍然算第一层那个 key
        write(&account.join("Banks/地图A"), "a.SC2Bank", "bank");
        write(&account.join("Banks/地图A/备份"), "b.SC2Bank", "bank");
        write(&account.join("Banks"), "root.SC2Bank", "bank");
        write(&account.join("Banks/地图A"), "ignore.txt", "x");

        // 第二个账号：有 Saves 目录但一个存档都没有
        std::fs::create_dir_all(docs.join("Accounts/99999/亚服-99999/Saves")).expect("建");

        // 时间：top 最新，campaign 次之，unsaved 最老
        set_mtime(&account.join("Saves/top.SC2Save"), 1_700_000_300);
        set_mtime(
            &account.join("Saves/Campaign/campaign.SC2Save"),
            1_700_000_200,
        );
        set_mtime(
            &account.join("Saves/Unsaved/Campaign/unsaved.SC2Save"),
            1_700_000_100,
        );

        let inventory = scan(&docs);

        let names: Vec<&str> = inventory
            .saves
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["top", "campaign", "unsaved"],
            "只收 .SC2Save，而且按修改时间降序"
        );
        let folders: Vec<&str> = inventory
            .saves
            .iter()
            .map(|item| item.folder.as_str())
            .collect();
        assert_eq!(
            folders,
            vec!["Saves", "Saves/Campaign", "Saves/Unsaved/Campaign"]
        );
        assert!(inventory.saves.iter().all(|item| item.account == "12345"));
        assert!(
            inventory
                .saves
                .iter()
                .all(|item| item.region == "美服-12345")
        );
        assert!(
            inventory.saves.iter().all(|item| item.bytes > 0),
            "字节数要读出来"
        );

        assert_eq!(inventory.banks.len(), 3, "Hotkeys 和非 .SC2Bank 都不收");
        let map_a = inventory
            .banks
            .iter()
            .filter(|item| item.map == "地图A")
            .count();
        assert_eq!(map_a, 2, "父目录名当地图 key（再深的子目录也算同一张图）");
        assert!(
            inventory.banks.iter().any(|item| item.map.is_empty()),
            "直接在 Banks 根下的 map key 是空串"
        );

        // 两个账号都列出来，最近活跃的在前
        assert_eq!(
            inventory.accounts,
            vec!["12345".to_string(), "99999".to_string()]
        );
    }

    #[test]
    fn scan_without_documents_is_empty_not_a_panic() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let inventory = scan(&tmp.path().join("根本没有这个目录"));
        assert!(inventory.saves.is_empty());
        assert!(inventory.banks.is_empty());
        assert!(inventory.accounts.is_empty());
    }
}
