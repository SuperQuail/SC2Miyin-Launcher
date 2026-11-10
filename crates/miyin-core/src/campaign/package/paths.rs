//! 包内路径的语义：条目名校验、内容根剥离，以及「这条路径该落到游戏目录的哪里」。
//!
//! 全是纯函数 —— 只看字符串与路径，不碰压缩包、不写盘。判错的后果都是「游戏扫不到」，
//! 所以每一条规则都带着真实样本的来历。
//!
//! # 落点只有三种语义
//!
//! | 类型 | 含义 | 最终落点 |
//! | --- | --- | --- |
//! | `GamePath` | 包里**写死**了游戏目录内的相对路径 | 原路径（见判定表） |
//! | `Mod` | 模组，相对 `Mods/` 的路径（目录结构保留） | `Mods/…` |
//! | `CampaignMap` | 战役地图，相对**这个战役自己的目录**的路径 | 由导入槽位决定 |
//!
//! `CampaignMap` 的名字可以带目录层（`evolution/…`、`voidprologue/…`），
//! 展开成游戏路径是 `library::compose::payload_target_path` 的事。
//!
//! # 判定表
//!
//! 输入是**内容根相对路径**（`layout::analyze` 已经剥掉包装层）与两个布局事实
//! （内容根、包根是不是游戏目录镜像）。顺序即优先级：
//!
//! | # | 形态 | 例子 | 结果 |
//! | --- | --- | --- | --- |
//! | 1 | 模组 | `Mods/Alenger/x.SC2Mod` / `X.SC2Mod` | `Mod { name: 相对 Mods 的路径 }` |
//! | 2 | 裸地图（没有目录层） | `x.SC2Map` | `CampaignMap { x }` |
//! | 3 | 游戏目录开头，`Maps/` 下还有目录 | `Maps/Campaign/void/a.SC2Map` | `GamePath` 原样 |
//! | 4 | `Maps/` 根上直接躺着的裸地图 | `Maps/x.SC2Map` | `CampaignMap { x }`（游戏不扫 `Maps` 根） |
//! | 5 | 省了 `Maps/` 的官方分类目录 | `Campaign/void/a.SC2Map` | `GamePath { Maps/Campaign/… }` |
//! | 6 | 战役内容形态（官方战役目录名开头 / 内容根已在战役目录里） | `voidprologue/a.SC2Map`、`evolution/za.SC2Map` | `CampaignMap` |
//! | 7 | 包根是游戏目录镜像时的兄弟路径 | `Starcraft Mass Recall/1. Rebel Yell/a.SC2Map` | `GamePath { Maps/… }` |
//! | 8 | 其余（作者的分类目录） | `maps/01.SC2Map` | `CampaignMap { 01.SC2Map }` |
//!
//! 几条**为什么**：
//!
//! - 裸地图（2）没有目录层，写不出有意义的游戏相对路径 —— 游戏扫地图按目录找，
//!   裸文件只能是战役地图。
//! - `Maps` 根（4）不是游戏会扫描的位置：官方战役在 `Maps/Campaign`、自制战役在
//!   `Maps/CustomCampaigns`，其余目录要作者在启动器地图里**按路径引用**。
//!   `Maps/xxx.SC2Map` 谁都不引用它，实际含义就是「这个包的战役地图」。
//!   （真实样本「机翻版（中文启动）」：`Maps/metadata.txt` + `Maps/*.SC2Map` + `Mods/…`
//!   的 CCM 包被套进了游戏目录镜像。）
//! - 大小写的差别在这里收掉（`maps/`、`MAPS/` 都是 `Maps/`）：Windows 上落盘后
//!   本来就是同一个目录，按原文拼只会拼出游戏扫不到的 `Maps/maps/…`。

use std::path::{Path, PathBuf};

use crate::campaign::sanitize::sanitize_dir_name;

use super::types::PayloadTarget;

/// 校验 zip 条目名，并返回清理后的相对路径。
///
/// 返回 `None` 表示该条目**越界或不可用**，必须整体拒绝这个包：
/// 绝对路径、盘符、UNC、以及任何 `..` 组件。
pub fn safe_entry_path(name: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");

    if normalized.starts_with('/') {
        return None;
    }
    // 盘符形式：C:\\foo
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        return None;
    }

    let mut out = PathBuf::new();
    for component in normalized.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            return None;
        }
        // 复用目录名安全化：非法字符替换、保留名拒绝、长度截断
        out.push(sanitize_dir_name(component)?);
    }

    (!out.as_os_str().is_empty()).then_some(out)
}

/// 取相对路径的父目录字符串（统一用 `/` 分隔）。
pub(crate) fn parent_prefix(path: &Path) -> String {
    let Some(parent) = path.parent() else {
        return String::new();
    };
    parent
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 去掉内容根前缀，得到包内相对路径。
pub(crate) fn strip_prefix(path: &Path, prefix: &str) -> Option<PathBuf> {
    if prefix.is_empty() {
        return Some(path.to_path_buf());
    }
    path.strip_prefix(prefix).ok().map(Path::to_path_buf)
}

/// 游戏根下的三个规范目录名 —— **大小写不敏感**，返回规范拼写。
///
/// Windows 的文件系统不分大小写：包里的 `maps/x.SC2Map` 和 `Maps/x.SC2Map` 落盘之后
/// 是同一个地方。以前按区分大小写比较，小写 `maps/` 认不出来，会被当成「普通分类目录」
/// 原样塞进 `Maps/` 底下，变成 `Maps/maps/x.SC2Map` —— 游戏就找不到这些地图了。
pub(crate) fn canonical_game_dir(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("maps") {
        return Some("Maps");
    }
    if name.eq_ignore_ascii_case("mods") {
        return Some("Mods");
    }
    if name.eq_ignore_ascii_case("interfaces") {
        return Some("Interfaces");
    }
    None
}

/// 这个名字是不是官方战役目录（`Maps/Campaign` 下那一层的名字）。
fn is_known_campaign_dir(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "swarm" | "void" | "voidprologue" | "nova" | "campaign" | "evolution"
    )
}

/// 包内相对路径的**顶层目录**是不是官方战役目录名。
///
/// 是的话说明作者已经按游戏的结构摆好了（`voidprologue/…`），落点直接用这条路径；
/// 否则相对路径是相对「这个战役自己的目录」的（`evolution/…` -> swarm/evolution/）。
pub fn known_campaign_prefix(source: &str) -> Option<&'static str> {
    // 长的排前面，避免 `swarm/evolution` 被 `swarm` 抢先匹配
    const KNOWN: [&str; 5] = ["swarm/evolution", "voidprologue", "swarm", "void", "nova"];

    let lower = source.to_ascii_lowercase();
    KNOWN
        .into_iter()
        .find(|prefix| lower.starts_with(&format!("{prefix}/")))
}

/// 决定载荷落到游戏目录的哪里。判定表见模块文档。
///
/// `content_root` 只用来判断「内容根是不是已经落在某个战役目录里」；
/// `game_root_mirror` 由 `layout::analyze` 给出。
pub(crate) fn payload_target(
    source: &str,
    is_mod: bool,
    content_root: &str,
    game_root_mirror: bool,
) -> PayloadTarget {
    let path = normalise(source);
    let segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();

    // 载荷根一定带文件名；真遇到空路径也别 panic，当无名战役内容交给后面。
    let Some(first) = segments.first().copied() else {
        return PayloadTarget::CampaignMap {
            name: String::new(),
        };
    };

    // 1) 模组：唯一归宿是 `Mods/` 下 —— 地图依赖里写的是 `Mods\…` 这种相对游戏根的
    //    路径，所以 `Mods/` 之后的目录结构必须原样保留（`Alenger/1钢铁.SC2Mod` 是
    //    一个模组的一部分，不是两个）。没写 `Mods/` 的按「模组自己的目录」摆，同样保留结构。
    if is_mod {
        let name = strip_leading_dir(&segments, "Mods").unwrap_or_else(|| segments.join("/"));
        return PayloadTarget::Mod { name };
    }

    // 2) 裸地图：没有目录层，写不出有意义的游戏相对路径 —— 只能是战役内容。
    if segments.len() == 1 {
        return campaign_map(&path, content_root);
    }

    // 3/4) 开头就是游戏目录（大小写不敏感，落点用规范拼写）
    if let Some(canonical) = canonical_game_dir(first) {
        let rest = &segments[1..];
        return match canonical {
            // `Maps/Campaign`、`maps/campaign` 也归一成规范拼写（Windows 上本来就是一回事）
            "Maps" if rest.len() >= 2 => PayloadTarget::GamePath {
                path: under_maps(rest),
            },
            // `Maps/` 根上直接躺着的地图不是游戏会扫的位置（官方战役在
            // `Maps/Campaign`、自制在 `Maps/CustomCampaigns`、其余目录要按路径引用）——
            // 它实际含义就是「这个包的战役地图」。
            "Maps" => campaign_map(&rest.join("/"), content_root),
            _ => PayloadTarget::GamePath {
                path: join_parts(&[canonical], rest),
            },
        };
    }

    // 5) 省了 `Maps/` 的官方分类目录：`Campaign/…`、`CustomCampaigns/…`
    if first.eq_ignore_ascii_case("campaign") || first.eq_ignore_ascii_case("customcampaigns") {
        return PayloadTarget::GamePath {
            path: under_maps(&segments),
        };
    }

    // 6) 战役内容形态：内容根已经在战役目录里，或者作者按游戏结构写了官方目录名。
    let root_last = content_root.rsplit('/').next().unwrap_or_default();
    let in_campaign_dir = is_known_campaign_dir(root_last);
    let leading_campaign_shape =
        known_campaign_prefix(&path).is_some() || first_is_evolution(&path);
    if in_campaign_dir || leading_campaign_shape {
        return campaign_map(&path, content_root);
    }

    // 7) 包根是游戏目录镜像：兄弟目录相对 `Maps/`（复刻战役 SCMR 那种按路径联动关卡）。
    if game_root_mirror {
        return PayloadTarget::GamePath {
            path: format!("Maps/{path}"),
        };
    }

    // 8) 其余都是战役内容（作者的分类目录会拍平）。
    campaign_map(&path, content_root)
}

/// 战役内容（`CampaignMap`）的相对路径怎么取。
///
/// - 内容根已经在战役目录里（`X/swarm/metadata.txt`）：包内相对路径是**该战役目录之内**
///   的结构，原样保留；进化任务的内容根（`evolution`）要补回 `evolution/` 这一层。
/// - 作者写了官方战役目录名（`voidprologue/…`、`swarm/…`、`nova/…`）：原样保留，
///   由 placement 决定从哪一层拼。
/// - 其余（作者的分类目录、被拒的 `Maps` 根裸地图）：只取文件名 —— 游戏不扫作者自拟的目录名。
fn campaign_map(relative: &str, content_root: &str) -> PayloadTarget {
    let path = relative.trim_matches('/');
    let root_last = content_root.rsplit('/').next().unwrap_or_default();

    if root_last.eq_ignore_ascii_case("evolution") {
        return PayloadTarget::CampaignMap {
            name: if first_is_evolution(path) {
                path.to_string()
            } else {
                format!("evolution/{path}")
            },
        };
    }

    if is_known_campaign_dir(root_last) {
        return PayloadTarget::CampaignMap {
            name: path.to_string(),
        };
    }

    if known_campaign_prefix(path).is_some() || first_is_evolution(path) {
        return PayloadTarget::CampaignMap {
            name: path.to_string(),
        };
    }

    let name = path.rsplit('/').next().unwrap_or(path);
    PayloadTarget::CampaignMap {
        name: name.to_string(),
    }
}

/// 统一分隔符、去掉首尾与重复的 `/`。
fn normalise(source: &str) -> String {
    source
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// `segments` 以 `dir` 开头（大小写不敏感）时返回其余部分。
fn strip_leading_dir(segments: &[&str], dir: &str) -> Option<String> {
    let first = segments.first()?;
    first
        .eq_ignore_ascii_case(dir)
        .then(|| segments[1..].join("/"))
        .filter(|rest| !rest.is_empty())
}

/// `Maps/` 下的路径：`Campaign` / `CustomCampaigns` 这一层用规范拼写。
fn under_maps(rest: &[&str]) -> String {
    match rest.first() {
        Some(first) if first.eq_ignore_ascii_case("campaign") => {
            join_parts(&["Maps", "Campaign"], &rest[1..])
        }
        Some(first) if first.eq_ignore_ascii_case("customcampaigns") => {
            join_parts(&["Maps", "CustomCampaigns"], &rest[1..])
        }
        _ => join_parts(&["Maps"], rest),
    }
}

fn join_parts(prefix: &[&str], rest: &[&str]) -> String {
    prefix
        .iter()
        .copied()
        .chain(rest.iter().copied())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// 路径第一段是不是 `evolution`（HotS 进化任务，相对战役目录的那一层）。
fn first_is_evolution(path: &str) -> bool {
    path.split('/')
        .next()
        .is_some_and(|first| first.eq_ignore_ascii_case("evolution"))
}

/// `candidate` 是否位于 `root` 之下（用来判断载荷是不是解开的目录树）。
pub(crate) fn is_under(candidate: &Path, root: &str) -> bool {
    candidate
        .strip_prefix(root)
        .map(|rest| rest.components().next().is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::compose::{Placement, payload_target_path};

    /// 判定表的一行。
    fn check(
        source: &str,
        is_mod: bool,
        content_root: &str,
        mirror: bool,
        expected: PayloadTarget,
    ) {
        let found = payload_target(source, is_mod, content_root, mirror);
        assert_eq!(found, expected, "输入 {source:?}");
    }

    fn card(name: &str) -> PayloadTarget {
        PayloadTarget::CampaignMap {
            name: name.to_string(),
        }
    }

    fn game(path: &str) -> PayloadTarget {
        PayloadTarget::GamePath {
            path: path.to_string(),
        }
    }

    fn module(name: &str) -> PayloadTarget {
        PayloadTarget::Mod {
            name: name.to_string(),
        }
    }

    #[test]
    fn campaign_content_is_recognised_in_every_written_form() {
        // 平铺 CCM：内容根旁边的地图
        check("paiur01.SC2Map", false, "", false, card("paiur01.SC2Map"));
        // 作者的分类目录：拍平（游戏不扫作者自拟的目录名）
        check("maps/01.SC2Map", false, "", false, card("01.SC2Map"));
        check("files/扩展/02.SC2Map", false, "", false, card("02.SC2Map"));
        // 官方战役目录名开头：原样保留
        check(
            "voidprologue/voidprologue01.SC2Map",
            false,
            "",
            false,
            card("voidprologue/voidprologue01.SC2Map"),
        );
        check(
            "swarm/evolution/zevo.SC2Map",
            false,
            "",
            false,
            card("swarm/evolution/zevo.SC2Map"),
        );
        check(
            "nova/nova01.SC2Map",
            false,
            "",
            false,
            card("nova/nova01.SC2Map"),
        );
        // 进化任务：相对 swarm 战役目录的那一层
        check(
            "evolution/zevolutionzergling.SC2Map",
            false,
            "",
            false,
            card("evolution/zevolutionzergling.SC2Map"),
        );
        // 内容根已经落在战役目录里：包内路径是该战役目录之内的结构
        check(
            "evolution/zevo.SC2Map",
            false,
            "酒馆合作虫心/swarm",
            false,
            card("evolution/zevo.SC2Map"),
        );
        check(
            "zchar01.SC2Map",
            false,
            "酒馆合作虫心/swarm",
            false,
            card("zchar01.SC2Map"),
        );
        // 内容根自己就是 evolution：补回这一层
        check(
            "zevo.SC2Map",
            false,
            "某包/evolution",
            false,
            card("evolution/zevo.SC2Map"),
        );
    }

    #[test]
    fn explicit_game_paths_are_kept_exactly_as_written() {
        // 官方战役镜像
        check(
            "Maps/Campaign/void/paiur01.SC2Map",
            false,
            "",
            false,
            game("Maps/Campaign/void/paiur01.SC2Map"),
        );
        // 复刻战役：Maps 下的作者目录（启动器地图按相对 Maps/ 的路径联动关卡）
        check(
            "Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
            false,
            "",
            true,
            game("Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map"),
        );
        // 自制战役目录
        check(
            "Maps/CustomCampaigns/精英虫心1.55 汉化/zchar01.SC2Map",
            false,
            "",
            false,
            game("Maps/CustomCampaigns/精英虫心1.55 汉化/zchar01.SC2Map"),
        );
        // 大小写与反斜杠都归一成规范拼写
        check(
            "maps/campaign/void/a.SC2Map",
            false,
            "",
            false,
            game("Maps/Campaign/void/a.SC2Map"),
        );
        check(
            "MAPS\\CUSTOMCAMPAIGNS\\X\\a.SC2Map",
            false,
            "",
            false,
            game("Maps/CustomCampaigns/X/a.SC2Map"),
        );
        // 省了 Maps/ 的官方分类目录
        check(
            "Campaign/void/a.SC2Map",
            false,
            "",
            false,
            game("Maps/Campaign/void/a.SC2Map"),
        );
        check(
            "CustomCampaigns/X/a.SC2Map",
            false,
            "",
            false,
            game("Maps/CustomCampaigns/X/a.SC2Map"),
        );
        // 界面 Mod：原样落
        check(
            "Interfaces/Pro_2020/UI.SC2Interface",
            false,
            "",
            false,
            game("Interfaces/Pro_2020/UI.SC2Interface"),
        );
    }

    #[test]
    fn maps_flat_at_the_maps_root_are_campaign_maps() {
        // 真实样本「机翻版（中文启动）」：Maps/metadata.txt + Maps/*.SC2Map + Mods/…
        check(
            "Maps/thanson01.SC2Map",
            false,
            "",
            true,
            card("thanson01.SC2Map"),
        );
        check(
            "Maps/thanson01.SC2Map",
            false,
            "",
            false,
            card("thanson01.SC2Map"),
        );
        check(
            "maps/thanson01.SC2Map",
            false,
            "",
            true,
            card("thanson01.SC2Map"),
        );
        check(
            "MAPS/thanson01.SC2Map",
            false,
            "",
            true,
            card("thanson01.SC2Map"),
        );
    }

    #[test]
    fn bare_files_are_never_game_paths() {
        // 裸文件写不出游戏相对路径：镜像根上也不行（以前会变成 Maps/x.SC2Map —— 死路）
        check("launcher.SC2Map", false, "", true, card("launcher.SC2Map"));
        check("launcher.SC2Map", false, "", false, card("launcher.SC2Map"));
    }

    #[test]
    fn mirror_root_makes_sibling_directories_maps_relative() {
        // 复刻战役 SCMR：Mods/ 在包根，兄弟目录是 Maps/ 里的内容
        check(
            "Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
            false,
            "",
            true,
            game("Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map"),
        );
        // 不在镜像根里就仍按战役内容处理
        check(
            "Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map",
            false,
            "",
            false,
            card("Terran01.SC2Map"),
        );
    }

    #[test]
    fn mods_always_land_under_mods_with_their_directories_kept() {
        check("Mods/X.SC2Mod", true, "", true, module("X.SC2Mod"));
        check("mods/X.SC2Mod", true, "", false, module("X.SC2Mod"));
        check("MODS/X.SC2Mod", true, "", false, module("X.SC2Mod"));
        // 目录结构保留：地图依赖里写的就是带目录的名字
        check(
            "Mods/Alenger/1钢铁.SC2Mod",
            true,
            "",
            true,
            module("Alenger/1钢铁.SC2Mod"),
        );
        check(
            "Alenger/1钢铁.SC2Mod",
            true,
            "",
            false,
            module("Alenger/1钢铁.SC2Mod"),
        );
        // 裸模组 + 镜像根：以前会被塞到 Maps/ 下（死路），现在一律 Mods/
        check("SCMRmod.SC2Mod", true, "", true, module("SCMRmod.SC2Mod"));
    }

    #[test]
    fn landings_are_what_the_game_reads() {
        // 战役地图：wol 直接进 Maps/Campaign，hots 进 swarm 子目录
        let wol = Placement::Campaign { sub: None };
        let hots = Placement::Campaign {
            sub: Some("swarm".into()),
        };
        let custom = Placement::of("custom", "某包", None);

        assert_eq!(
            payload_target_path(&card("a.SC2Map"), &wol).as_deref(),
            Some("Maps/Campaign/a.SC2Map")
        );
        assert_eq!(
            payload_target_path(&card("evolution/zevo.SC2Map"), &hots).as_deref(),
            Some("Maps/Campaign/swarm/evolution/zevo.SC2Map")
        );
        assert_eq!(
            payload_target_path(&card("voidprologue/a.SC2Map"), &wol).as_deref(),
            Some("Maps/Campaign/voidprologue/a.SC2Map")
        );
        assert_eq!(
            payload_target_path(&card("a.SC2Map"), &custom).as_deref(),
            Some("Maps/CustomCampaigns/某包/a.SC2Map")
        );
        // 模组与游戏路径不受槽位影响
        assert_eq!(
            payload_target_path(&module("Alenger/x.SC2Mod"), &custom).as_deref(),
            Some("Mods/Alenger/x.SC2Mod")
        );
        assert_eq!(
            payload_target_path(&game("Maps/Starcraft Mass Recall/x.SC2Map"), &wol).as_deref(),
            Some("Maps/Starcraft Mass Recall/x.SC2Map")
        );
    }

    #[test]
    fn rejects_traversal_and_absolute_entry_names() {
        assert_eq!(safe_entry_path("../evil.txt"), None);
        assert_eq!(safe_entry_path("a/../../evil.txt"), None);
        assert_eq!(safe_entry_path("/etc/passwd"), None);
        assert_eq!(safe_entry_path("C:\\Windows\\system32"), None);
        assert_eq!(safe_entry_path("\\\\server\\share\\x"), None);
    }

    #[test]
    fn accepts_and_normalizes_normal_entry_names() {
        assert_eq!(
            safe_entry_path("MyCampaign/maps/a.SC2Map"),
            Some(PathBuf::from("MyCampaign/maps/a.SC2Map"))
        );
        assert_eq!(
            safe_entry_path("./MyCampaign//b.txt"),
            Some(PathBuf::from("MyCampaign/b.txt"))
        );
    }

    #[test]
    fn derives_content_root_from_nested_metadata() {
        assert_eq!(
            parent_prefix(Path::new("MyCampaign/metadata.txt")),
            "MyCampaign"
        );
        assert_eq!(parent_prefix(Path::new("metadata.txt")), "");
        assert_eq!(parent_prefix(Path::new("a/b/metadata.txt")), "a/b");
    }

    #[test]
    fn strip_prefix_drops_content_root() {
        let stripped = strip_prefix(Path::new("MyCampaign/maps/a.SC2Map"), "MyCampaign");
        assert_eq!(stripped, Some(PathBuf::from("maps/a.SC2Map")));
    }
}
