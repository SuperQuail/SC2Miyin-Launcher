//! 存档管理：`Documents\StarCraft II\Banks` 的备份与还原（issue #20）。
//!
//! **存档是用户数据**，不是游戏本体 —— 所以它不走游戏目录那套写盘闸门
//! （那个闸门只管安装目录里的文件）。但同样守两条：**动之前先备份**、
//! **先给人看**（[`SaveSet`] 就是给人看的那一份）。
//!
//! **存档隔离**：打开之后，每个战役用自己的那份 Banks，切换战役时自动换档。
//! 默认**关着** —— 不开的时候启动器一个字节都不碰存档。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

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

/// 一份已备份的存档。
#[derive(Debug, Clone, Serialize)]
pub struct BackupEntry {
    /// 备份目录名，还原时拿它指认
    pub name: String,
    /// 备份时带的标签（战役 / 用户自己写的备注）
    pub label: String,
    pub bytes: u64,
    pub files: usize,
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
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
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
    let safety = backup(banks, library_root, "还原前").ok();

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
// 存档隔离（按安装的战役分开）
// ---------------------------------------------------------------------------

/// 隔离状态。存在 `<data>/saves/state.json`。
///
/// **默认关**：不开的时候启动器一个字节都不碰 Banks，切换战役也不动存档。
/// 打开的那一刻，当前这份 Banks 会被收成名叫「原版」的组 —— 用户原有的进度
/// 不会被谁顶掉。
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
/// "初始存档按照原版存"。关掉只是不再自动切，已经存下的组一个都不删。
pub fn set_isolation(root: &Path, banks: &Path, enabled: bool) -> Result<Isolation> {
    let mut state = isolation(root)?;
    if enabled && state.active.is_none() {
        let name = backup(banks, root, ORIGINAL)?;
        state.active = Some(name);
    }
    state.enabled = enabled;
    write_isolation(root, &state)?;
    Ok(state)
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

    // 覆盖式存回：先清掉旧内容再铺，否则被删掉的档会留在组里
    let dir = self::root(root).join(&name);
    std::fs::create_dir_all(&dir)?;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }
    let set = snapshot(banks)?;
    for file in &set.files {
        std::fs::copy(banks.join(&file.name), dir.join(&file.name))?;
    }

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

    // 现在这份先回自己的组（没有 active 就是第一次，直接归档）
    let mut state = isolation(root)?;
    if state.active.is_some() || snapshot(banks)?.files.is_empty() {
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

/// 启用某个战役之前调它：隔离开着就切到该战役的存档组。
///
/// 没有组就**新建一个空的并指派给它** —— 这个战役从零开始，原版进度不受影响。
/// 返回切到了哪个组（没开隔离返回 `None`）。
pub fn ensure_for_slot(
    root: &Path,
    banks: &Path,
    slot: &str,
    label: &str,
) -> Result<Option<String>> {
    let state = isolation(root)?;
    if !state.enabled {
        return Ok(None);
    }

    if let Some(name) = profile_for(root, slot)? {
        switch(root, banks, &name)?;
        return Ok(Some(name));
    }

    // 新建一个空组，名字带上战役名，然后指派
    let name = dir_name(label);
    std::fs::create_dir_all(self::root(root).join(&name))?;
    assign(root, &name, Some(slot))?;
    switch(root, banks, &name)?;
    Ok(Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(dir).expect("建目录");
        std::fs::write(dir.join(name), body).expect("写");
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
    fn isolation_starts_off_and_stores_the_current_progress_first() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "原版.SC2Bank", "playthrough");

        assert!(!isolation(&library).expect("读").enabled, "默认必须是关的");

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
    fn switching_swaps_the_saves_and_remembers_where_we_were() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "原版第一章");
        set_isolation(&library, &banks, true).expect("打开");

        // 给某个战役新建一个空组
        let profile = ensure_for_slot(&library, &banks, "wol", "自由之翼")
            .expect("切换")
            .expect("开了隔离就该切");
        assert!(profile.contains("自由之翼"), "组名带战役名：{profile}");
        assert!(
            snapshot(&banks).expect("快照").files.is_empty(),
            "新战役从零开始"
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
    fn nothing_happens_while_isolation_is_off() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let banks = tmp.path().join("Banks");
        let library = tmp.path().join("data");
        write(&banks, "进度.SC2Bank", "原版");

        assert_eq!(
            ensure_for_slot(&library, &banks, "wol", "自由之翼").expect("调"),
            None,
            "没开隔离就不该碰存档"
        );
        assert_eq!(
            std::fs::read_to_string(banks.join("进度.SC2Bank")).expect("读"),
            "原版"
        );
    }
}
