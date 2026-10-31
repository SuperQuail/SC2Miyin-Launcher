//! 包内路径的语义：条目名校验、内容根剥离，以及「这条路径该落到游戏目录的哪里」。
//!
//! 全是纯函数 —— 只看字符串与路径，不碰压缩包、不写盘。判错的后果都是「游戏扫不到」，
//! 所以每一条规则都带着真实样本的来历。

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

/// `Maps/` 后面是不是游戏会扫描的分类目录。
///
/// 平坦的 `maps/01.SC2Map` 是**作者的分类习惯**（不是镜像）：真当镜像摆到
/// `Maps/01.SC2Map` 游戏根本不扫，地图就白装了 —— 那种要交给拍平逻辑。
fn looks_like_game_category(rest: &[&str]) -> bool {
    rest.first().is_some_and(|first| {
        first.eq_ignore_ascii_case("campaign") || first.eq_ignore_ascii_case("customcampaigns")
    })
}

/// 路径里第一段 `Maps` / `Mods`，从那里往后就是游戏目录内的相对路径。
///
/// **为什么不能只看开头**：包常常多一层「包名」目录 ——
/// 真实样本是 `疯批帝国军械库2.3/Mods/Alenger/1钢铁.SC2Mod`。
/// 只判断 `starts_with("Mods/")` 的话这一层就把整个前缀吃掉了，
/// 掉到下面按 `is_mod` 取「最后一段当名字」，
/// 结果 `Mods/Alenger/1钢铁.SC2Mod` 变成 `Mods/1钢铁.SC2Mod` ——
/// **`Alenger/` 这一层没了**，地图里声明的 `Mods\Alenger\…` 自然找不到。
///
/// **大小写不敏感**：Windows 上 `maps/` 就是 `Maps/`，落盘之后没有区别 ——
/// 用户在 Windows 上放 `maps/Campaign/…`，装完必须长成 `Maps/Campaign/…`。
/// 认出来之后用**规范拼写**拼接，盘上的目录名始终是游戏认的那个。
///
/// `Maps` 还要求后面跟着游戏扫描的分类（`Campaign` / `CustomCampaigns`）：
/// 平坦的 `maps/01.SC2Map` 是作者的分类习惯，交给拍平逻辑才对。
fn game_relative(source: &str) -> Option<String> {
    let normalised = source.replace('\\', "/");
    let parts: Vec<&str> = normalised.split('/').collect();

    for (index, part) in parts.iter().enumerate() {
        let Some(canonical) = canonical_game_dir(part) else {
            continue;
        };
        let rest = &parts[index + 1..];
        // 只有**大小写不对**的写法才需要额外证据：`maps/` 既可能是镜像也可能是分类习惯。
        // 规范拼写 `Maps/` 一律当镜像 —— 复刻战役那种 `Maps/Starcraft Mass Recall/…`
        // 必须原样保留（它的启动器地图按相对 Maps/ 的路径联动关卡，拍平就废了）。
        let non_canonical = *part != canonical;
        if non_canonical && canonical == "Maps" && !looks_like_game_category(rest) {
            continue;
        }
        let mut out: Vec<String> = rest.iter().map(|p| p.to_string()).collect();
        out.insert(0, canonical.to_string());
        return Some(out.join("/"));
    }

    None
}

/// 这个名字是不是官方战役目录（`Maps/Campaign` 下那一层的名字）。
fn is_known_campaign_dir(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "swarm" | "void" | "voidprologue" | "nova" | "campaign"
    )
}

/// 包内相对路径的**顶层目录**是不是官方战役目录名。
///
/// 是的话说明作者已经按游戏的结构摆好了（`voidprologue/…`），落点直接用这条路径；
/// 否则相对路径是相对"这个战役自己的目录"的（`evolution/…` -> swarm/evolution/）。
pub fn known_campaign_prefix(source: &str) -> Option<&'static str> {
    // 长的排前面，避免 `swarm/evolution` 被 `swarm` 抢先匹配
    const KNOWN: [&str; 6] = [
        "swarm/evolution",
        "voidprologue",
        "swarm",
        "void",
        "nova",
        "customcampaigns",
    ];

    let lower = source.to_ascii_lowercase();
    KNOWN
        .into_iter()
        .find(|prefix| lower.starts_with(&format!("{prefix}/")))
}

/// 决定载荷落到游戏目录的哪里。
///
/// 对 crate 内可见是为了能直接测：包名那层怎么剥，只有在这里才说得清。
pub(crate) fn payload_target(
    source: &str,
    is_mod: bool,
    content_root: &str,
    mirrors_game_root: bool,
) -> PayloadTarget {
    // 包内已经摆好了游戏目录那一层（`Maps/…` 或 `Mods/…`）-> 按原路径落盘
    if let Some(relative) = game_relative(source) {
        return PayloadTarget::Mirror { path: relative };
    }

    // 包是**按游戏根目录摆的**（根上有 `Mods/`）：那其余顶层目录就是 `Maps/`
    // 底下的，**结构和名字都要原样保留**。
    //
    // 判据来自作者自己：他既然把 `Mods/` 摆在根上，兄弟目录就是 `Maps/` 的内容。
    // 这里**不能**拍平 —— 复刻战役的启动器地图按 `Starcraft Mass Recall/…`
    // 这个相对 Maps/ 的路径联动下一关，拍平了它就找不到任何关卡。
    if mirrors_game_root {
        let path = source.replace('\\', "/");
        return PayloadTarget::Mirror {
            path: format!("Maps/{path}"),
        };
    }

    let path = source.replace('\\', "/");

    if is_mod {
        // 走到这里说明路径里**没有** `Mods/` 这一段（比如包根光秃秃一个
        // `X.SC2Mod`），那就当单文件模组，落到 `Mods/` 下。
        // 有 `Mods/` 的情况上面已经按原路径处理掉了。
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        return PayloadTarget::Mod { name };
    }

    // 地图该不该保留包内的相对目录，取决于**内容根站在哪里**：
    //
    // - 内容根已经落在官方战役目录里（`X/swarm/metadata.txt`）-> 包内相对路径是
    //   该战役目录**之内**的结构，要原样保留（`evolution/…` -> swarm/evolution/…）
    // - 否则相对路径是**并列**于战役目录的：顶层是官方目录名（`voidprologue/…`）
    //   就说明作者按游戏结构摆好了；其余（`maps/…`）只是作者的分类习惯，只取文件名
    // **HotS 的进化任务**：官方的目录是 `Maps/Campaign/swarm/evolution/`。
    // 作者经常把包内顶层写成 `evolution/…` —— 这是**相对战役目录**的结构
    // （见 known_campaign_prefix 的说明），要保留这一层交给 placement 拼到 `swarm/` 下。
    // 拍平到 swarm 根的话，游戏在「进化」里根本找不到这些地图（真实样本：AI盟友虫群之心）。
    // 另一种写法是 metadata.txt 就放在 evolution/ 里（内容根自己就是 evolution），同理。
    let evolution_root = content_root
        .rsplit('/')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("evolution"));
    let leading_evolution = path.contains('/')
        && path
            .split('/')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("evolution"));
    if evolution_root || leading_evolution {
        let name = if evolution_root && !leading_evolution {
            format!("evolution/{path}")
        } else {
            path.clone()
        };
        return PayloadTarget::Map { name };
    }

    let leading_is_campaign_dir = known_campaign_prefix(&path).is_some();
    let root_is_campaign_dir = content_root
        .rsplit('/')
        .next()
        .is_some_and(is_known_campaign_dir);

    if path.contains('/') && (root_is_campaign_dir || leading_is_campaign_dir) {
        return PayloadTarget::Map { name: path };
    }

    let name = path.rsplit('/').next().unwrap_or(&path).to_string();
    PayloadTarget::Map { name }
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

    /// HotS 进化任务：包内顶层 `evolution/` 是**相对战役目录**的结构，
    /// 不能被拍平到 swarm 根（真实样本：AI盟友虫群之心）。
    #[test]
    fn evolution_maps_keep_their_campaign_relative_prefix() {
        let target = payload_target("evolution/zevolutionzergling.SC2Map", false, "", false);
        assert!(
            matches!(&target, PayloadTarget::Map { name } if name == "evolution/zevolutionzergling.SC2Map"),
            "{target:?}"
        );

        // metadata.txt 放在 evolution/ 里：内容根自己就是 evolution
        let nested = payload_target("zevolutionzergling.SC2Map", false, "evolution", false);
        assert!(
            matches!(&nested, PayloadTarget::Map { name } if name == "evolution/zevolutionzergling.SC2Map"),
            "{nested:?}"
        );

        // 普通分类目录仍然拍平（maps/a.SC2Map -> a.SC2Map），别把所有目录都保留
        let flat = payload_target("maps/a.SC2Map", false, "", false);
        assert!(
            matches!(&flat, PayloadTarget::Map { name } if name == "a.SC2Map"),
            "{flat:?}"
        );
    }

    /// 界面 Mod：`Interfaces/` 下的东西要原样落到 `<游戏>/Interfaces/`。
    #[test]
    fn interface_payloads_keep_their_path() {
        let target = payload_target("Interfaces/Pro_2020/UI.SC2Interface", false, "", false);
        assert_eq!(
            crate::library::compose::payload_target_path(
                &target,
                &crate::library::compose::Placement::Campaign { sub: None }
            )
            .as_deref(),
            Some("Interfaces/Pro_2020/UI.SC2Interface"),
            "界面 Mod 必须原样落，不能拍进 Maps"
        );
    }
}
