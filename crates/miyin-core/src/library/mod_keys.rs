//! 模组身份：按文件夹归并、把包里的路径归一化成挂载键。

use crate::campaign::package::{Payload, PayloadTarget};

use super::model::{ModEntry, Variant};

/// 这个版本**实际**会铺哪些模组。
///
/// 没配过（`None`）时按「全挂」算 —— 保证老记录和刚导入的包都能正常跑。
pub fn effective_mounted_mods(variant: &Variant) -> Vec<String> {
    if let Some(list) = &variant.mounted_mods {
        return list.clone();
    }

    variant
        .payloads
        .iter()
        .filter_map(|payload| mod_identity(payload).map(|found| found.key))
        .collect()
}

/// 把一堆载荷按模组归并 —— **按文件夹去重**。
///
/// `Alenger/1钢铁.SC2Mod` 和 `Alenger/2贝希摩斯虫群.SC2Mod` 归成一行「Alenger」，
/// 并记下它由几个文件组成。地图载荷会被忽略。
pub fn group_mods(payloads: &[Payload], mounted: &[String], in_game: &[String]) -> Vec<ModEntry> {
    let mut rows: Vec<ModEntry> = Vec::new();

    for payload in payloads {
        let Some(found) = mod_identity(payload) else {
            continue;
        };

        match rows.iter_mut().find(|row| row.path == found.key) {
            Some(row) => row.parts += 1,
            None => rows.push(ModEntry {
                mounted: mounted.contains(&found.key),
                in_game: in_game.contains(&found.key),
                path: found.key,
                name: found.name,
                parts: 1,
            }),
        }
    }

    rows
}

/// 一个模组的身份。
///
/// **模组按文件夹分，不按单个文件分** —— 这是从真实样本学到的：
///
/// `@text
/// Mods/
/// ├── 3疯批帝国之翼.SC2Mod        单文件         -> 一个模组
/// ├── Alenger/                    普通文件夹     -> 一个模组
/// │   ├── 1钢铁.SC2Mod                            （里面 18 个 .SC2Mod）
/// │   └── …
/// └── kit_liberty_story.SC2Mod/   解开目录树     -> 一个模组
/// `@
///
/// 判定办法：取落点里 `Mods/` 之后**第一段**。
/// `Alenger/1钢铁.SC2Mod` 与 `Alenger/2贝希摩斯虫群.SC2Mod`
/// 是同一个模组「Alenger」的两部分，不是两个模组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModKey {
    /// 挂载键：`Mods/` 之后的第一段（文件夹名，或 .SC2Mod 文件名）。
    pub key: String,
    /// 显示名：去掉 .SC2Mod 后缀。
    pub name: String,
}

/// 认出一个载荷属于哪个模组；地图返回 `None`。
///
/// 两种写法都算模组：
/// - `PayloadTarget::Mod` —— 包内是裸的 `X.SC2Mod`
/// - `PayloadTarget::Mirror` 且落在 `Mods/` 下 —— 包内是游戏目录镜像
///   （真实样本几乎都是这种）
pub fn mod_identity(payload: &Payload) -> Option<ModKey> {
    match &payload.target {
        // 裸的 .SC2Mod：落点是 `Mods/{name}`，名字本身就是键
        // （名字里可能还带子目录，所以拼上前缀交给统一的解析）
        PayloadTarget::Mod { name } => mod_key_of(&format!("Mods/{name}")),
        // 游戏目录镜像：路径已经带 Mods/ 前缀了
        PayloadTarget::Mirror { path } => mod_key_of(path),
        PayloadTarget::Map { .. } => None,
    }
}

/// 把作者写的依赖模组名字归一化成键（**不认识 `Mods/` 前缀也认**）。
///
/// `@text
/// Mods/Alenger/          -> Alenger
/// Mods/Alenger           -> Alenger
/// Alenger                -> Alenger
/// Alenger.SC2Mod         -> Alenger
/// 3疯批帝国之翼.SC2Mod    -> 3疯批帝国之翼
/// `@
///
/// 包作者写依赖时偷懒不写前缀是很常见的，不能因此就当没声明。
pub fn normalize_mod_key(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/').replace('\\', "/");
    let without_prefix = trimmed
        .strip_prefix("Mods/")
        .or_else(|| trimmed.strip_prefix("mods/"))
        .unwrap_or(&trimmed);
    let first = without_prefix.split('/').next().unwrap_or(without_prefix);

    if first.to_ascii_lowercase().ends_with(".sc2mod") {
        first[..first.len() - ".SC2Mod".len()].to_string()
    } else {
        first.to_string()
    }
}

/// 从一条 `Mods/` 下的相对路径推出模组身份。
pub fn mod_key_of(relative: &str) -> Option<ModKey> {
    let normalised = relative.replace('\\', "/");
    let rest = normalised
        .strip_prefix("Mods/")
        .or_else(|| normalised.strip_prefix("mods/"))?;
    let first = rest.split('/').next().filter(|part| !part.is_empty())?;

    // 显示名去掉 .SC2Mod 后缀（大小写不敏感）
    let lower = first.to_ascii_lowercase();
    let name = if lower.ends_with(".sc2mod") {
        first[..first.len() - ".SC2Mod".len()].to_string()
    } else {
        first.to_string()
    };

    Some(ModKey {
        key: first.to_string(),
        name,
    })
}
