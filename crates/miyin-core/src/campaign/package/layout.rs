//! 包内**布局**：内容根在哪、包根是不是游戏目录镜像。
//!
//! 这两个结论以前散着算：内容根在 inspect 里推，镜像根在载荷收集里按
//! 「包根第一段是不是 `Mods`」判，落点解析又靠「路径里找 `Maps`/`Mods` 段」给包装目录兜底。
//! 三处各猜一半，遇到「机翻版（中文启动）」那种包就拼不出正确落点。
//!
//! 现在统一成一次分析、两个事实：
//!
//! 1. **内容根**：解压时剥离的那一层。
//!    - 有元数据：元数据目录，或它上面**第一个直接装着游戏目录**的祖先
//!      （真实样本「叛逃皇卫完结版」：`叛逃皇卫/{Maps,Mods}`，metadata 埋在
//!      `叛逃皇卫/Maps/Campaign/` 里 —— 取元数据目录会把同包的 `Mods/` 切掉）。
//!    - 没有元数据：包内最浅的那个直接装着游戏目录的目录
//!      （真实样本「疯批帝国军械库2.4」：`疯批帝国军械库2.3/{Maps,Mods}`，没有元数据。
//!      以前靠落点解析里的全路径搜索歪打正着，现在布局这一步就剥掉）。
//! 2. **游戏目录镜像**：内容根下直接有 `Mods/`。
//!    这是复刻战役 SCMR 那种「兄弟目录就是 `Maps/` 里的内容」写法的唯一信号：
//!    ```text
//!    Mods/SCMRmod.SC2Mod
//!    Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map   <- 相对游戏 Maps/ 的路径
//!    ```
//!
//! 落点解析只认**内容根相对路径的开头**（见 `package::paths::payload_target`），
//! 不再在整条路径里找游戏目录段 —— 那样一个叫 `Maps` 的普通子目录也能把落点带偏。

use crate::campaign::HealthIssue;

use super::paths::{canonical_game_dir, strip_prefix};
use super::payloads::is_our_metadata_dir;
use super::types::Entry;

/// 包内布局（inspect 阶段算一次，后面都用它）。
#[derive(Debug, Clone)]
pub(crate) struct Layout {
    /// 解压时剥离的内容根（相对包根；空串 = 包根）。
    pub(crate) content_root: String,
    /// 内容根下直接有 `Mods/`：作者的兄弟目录相对 `Maps/`。
    pub(crate) game_root_mirror: bool,
    /// 布局层面的提示（多个候选内容根、元数据与内容不在一层……）。
    pub(crate) issues: Vec<HealthIssue>,
}

/// 分析包内布局。`metadata_dir`：元数据所在目录（相对包根）。
///
/// `None` = **包里没有元数据**（这时布局说了算）；`Some("")` = 元数据就在包根 ——
/// 内容根不能因为「元数据目录是空串」就跳过去另找一个游戏根。
pub(crate) fn analyze(entries: &[Entry], metadata_dir: Option<&str>) -> Layout {
    let roots = game_roots(entries);
    let mut issues: Vec<HealthIssue> = Vec::new();

    let content_root = match metadata_dir {
        // 没有元数据：布局说了算。取最浅的游戏根 —— 外面套一层包名目录也能剥掉。
        None => shallowest(&roots, &mut issues),
        Some(dir) => {
            // 元数据在镜像包里：取元数据目录**上面**那层装了游戏目录的祖先
            //（真实样本「叛逃皇卫完结版」）。都没有就用元数据目录自己。
            let found = ancestors(dir)
                .into_iter()
                .find(|prefix| roots.iter().any(|root| root == prefix));
            match found {
                Some(content_root) => content_root,
                None => {
                    // 元数据目录自己就是内容根；但如果包内别处还有游戏目录，
                    // 说明作者把内容摆到了元数据之外 —— 这种布局算不出来，得如实说一句。
                    if !roots.is_empty() {
                        issues.push(HealthIssue::warning(
                            "CONTENT_OUTSIDE_METADATA",
                            format!(
                                "包内的游戏目录（{}）不在元数据所在目录「{dir}」下面，已按元数据目录当内容根；包里的地图/模组可能找不全",
                                describe_roots(&roots)
                            ),
                        ));
                    }
                    dir.to_string()
                }
            }
        }
    };

    let game_root_mirror = entries.iter().any(|entry| {
        strip_prefix(&entry.relative, &content_root)
            .map(|relative| relative_parts(&relative))
            .is_some_and(|parts| {
                parts
                    .first()
                    .is_some_and(|first| canonical_game_dir(first) == Some("Mods"))
            })
    });

    Layout {
        content_root,
        game_root_mirror,
        issues,
    }
}

/// 一条路径切成组件名（`/` 分隔由调用方自己拼）。
fn relative_parts(relative: &std::path::Path) -> Vec<String> {
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// 包内所有「游戏根」：每个直接装着 `Maps` / `Mods` / `Interfaces` 的目录。
///
/// 只认每条路径里**第一个**游戏目录段 —— `Maps/Campaign/…` 的游戏根是包根，
/// 不会因为 `Campaign` 里还有东西就多出候选。
///
/// `Miyin/` 是我们自己的附加数据目录（导出包会把旧的补丁内容原样塞进去，
/// 里面可能有 `Mods/…`），它不是包的内容，不能算游戏根。
fn game_roots(entries: &[Entry]) -> Vec<String> {
    let mut roots: Vec<String> = Vec::new();

    for entry in entries {
        if is_our_metadata_dir(&entry.relative) {
            continue;
        }
        let parts = relative_parts(&entry.relative);
        let Some(index) = parts
            .iter()
            .position(|part| canonical_game_dir(part).is_some())
        else {
            continue;
        };
        let prefix = parts[..index].join("/");
        if !roots.iter().any(|found| found == &prefix) {
            roots.push(prefix);
        }
    }

    roots
}

/// `path` 自己与它上面每一层（从包根开始）。
fn ancestors(path: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut current = String::new();
    for part in path.split('/').filter(|part| !part.is_empty()) {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        out.push(current.clone());
    }
    out
}

/// 最浅的游戏根。
///
/// **并列时返回包根**（什么都不剥）—— 这种包是多份内容的合集，剥掉任何一层都会让
/// 另一份内容整段丢失（解压只会铺内容根之下的条目）。宁可整套文件都进库、报一条
/// `MULTI_ROOT` 让人自己判断，也不能悄悄扔东西。
fn shallowest(roots: &[String], issues: &mut Vec<HealthIssue>) -> String {
    let depth = |root: &str| root.split('/').filter(|part| !part.is_empty()).count();

    let mut sorted: Vec<&String> = roots.iter().collect();
    sorted.sort_by(|left, right| depth(left).cmp(&depth(right)).then_with(|| left.cmp(right)));

    let Some(first) = sorted.first() else {
        return String::new();
    };

    if sorted.len() > 1 && depth(first) == depth(sorted[1]) {
        issues.push(HealthIssue::warning(
            "MULTI_ROOT",
            format!(
                "包内有多个并列的游戏目录根（{}），已把包根当内容根（不剥任何一层）—— 包里是一份合集，请确认要装的是哪一份",
                describe_roots(roots)
            ),
        ));
        return String::new();
    }

    (*first).clone()
}

/// 给用户看的目录名：空串写成「包根」。
fn describe_root(root: &str) -> String {
    if root.is_empty() {
        "包根".to_string()
    } else {
        root.to_string()
    }
}

fn describe_roots(roots: &[String]) -> String {
    roots
        .iter()
        .map(|root| describe_root(root))
        .collect::<Vec<String>>()
        .join("、")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(path: &str) -> Entry {
        Entry {
            index: 0,
            relative: PathBuf::from(path),
            size: 0,
        }
    }

    fn layout(metadata_dir: Option<&str>, paths: &[&str]) -> Layout {
        let entries: Vec<Entry> = paths.iter().map(|path| entry(path)).collect();
        analyze(&entries, metadata_dir)
    }

    #[test]
    fn flat_ccm_package_has_no_content_root() {
        let found = layout(
            Some(""),
            &[
                "metadata.txt",
                "paiur01.SC2Map",
                "HTXL.SC2Mod",
                "voidprologue/voidprologue01.SC2Map",
            ],
        );
        assert_eq!(found.content_root, "");
        assert!(!found.game_root_mirror, "没有 Mods/ 就不是游戏目录镜像");
        assert!(found.issues.is_empty(), "{:?}", found.issues);
    }

    #[test]
    fn metadata_less_map_pack_has_no_content_root() {
        let found = layout(None, &["paiur01.SC2Map", "HTXL.SC2Mod"]);
        assert_eq!(found.content_root, "");
        assert!(!found.game_root_mirror);
        assert!(found.issues.is_empty(), "{:?}", found.issues);
    }

    #[test]
    fn game_mirror_at_package_root_keeps_package_root() {
        let found = layout(
            Some(""),
            &[
                "Maps/Campaign/void/metadata.txt",
                "Maps/Campaign/void/paiur01.SC2Map",
                "Mods/X.SC2Mod",
            ],
        );
        assert_eq!(found.content_root, "");
        assert!(found.game_root_mirror);
        assert!(found.issues.is_empty(), "{:?}", found.issues);
    }

    #[test]
    fn metadata_inside_a_wrapped_mirror_keeps_the_wrapper() {
        // 真实样本「叛逃皇卫完结版」：包名那层里才是 Maps/ 与 Mods/
        let found = layout(
            Some("叛逃皇卫/Maps/Campaign"),
            &[
                "叛逃皇卫/Maps/Campaign/metadata.txt",
                "叛逃皇卫/Maps/Campaign/tarcade.SC2Map",
                "叛逃皇卫/Mods/叛逃者.SC2Mod",
            ],
        );
        assert_eq!(found.content_root, "叛逃皇卫");
        assert!(found.game_root_mirror);
        assert!(found.issues.is_empty(), "{:?}", found.issues);
    }

    #[test]
    fn metadata_less_wrapped_mirror_strips_the_wrapper() {
        // 真实样本「疯批帝国军械库2.4」：没有元数据，包名那层里是 Maps/ 与 Mods/
        let found = layout(
            None,
            &[
                "疯批帝国军械库2.3/Maps/Campaign/tarcade.SC2Map",
                "疯批帝国军械库2.3/Mods/Alenger/1钢铁.SC2Mod",
            ],
        );
        assert_eq!(found.content_root, "疯批帝国军械库2.3");
        assert!(found.game_root_mirror);
    }

    #[test]
    fn nested_ccm_metadata_is_the_content_root() {
        let found = layout(
            Some("Reborn"),
            &[
                "Reborn/metadata.txt",
                "Reborn/maps/01.SC2Map",
                "Reborn/Mods/extra.SC2Mod",
            ],
        );
        assert_eq!(found.content_root, "Reborn");
        assert!(found.game_root_mirror);
    }

    #[test]
    fn campaign_subdirectory_content_root_is_kept() {
        // 真实样本「酒馆合作虫心」：内容根是 X/swarm，包内没有游戏目录
        let found = layout(
            Some("酒馆合作虫心/swarm"),
            &[
                "酒馆合作虫心/swarm/metadata.txt",
                "酒馆合作虫心/swarm/zchar01.SC2Map",
                "酒馆合作虫心/swarm/evolution/zevolutionbaneling.SC2Map",
            ],
        );
        assert_eq!(found.content_root, "酒馆合作虫心/swarm");
        assert!(!found.game_root_mirror);
        assert!(found.issues.is_empty(), "{:?}", found.issues);
    }

    #[test]
    fn multiple_roots_at_the_same_depth_fall_back_to_the_package_root() {
        let found = layout(
            None,
            &[
                "A/Maps/Campaign/a.SC2Map",
                "B/Maps/Campaign/b.SC2Map",
                "B/Mods/x.SC2Mod",
            ],
        );
        assert_eq!(
            found.content_root, "",
            "并列时不能挑一个剥 —— 另一份内容会整段丢"
        );
        assert!(!found.game_root_mirror);
        assert!(
            found.issues.iter().any(|issue| issue.code == "MULTI_ROOT"),
            "{:?}",
            found.issues
        );
    }

    #[test]
    fn content_outside_the_metadata_dir_is_reported() {
        // metadata.txt 在包根，内容却在包装目录里：内容根按元数据目录算，
        // 这种布局算不出来 —— 必须让人知道，而不是悄悄装错。
        let found = layout(
            Some(""),
            &[
                "metadata.txt",
                "Pack/Maps/Campaign/a.SC2Map",
                "Pack/Mods/x.SC2Mod",
            ],
        );
        assert_eq!(found.content_root, "");
        assert!(!found.game_root_mirror);
        assert!(
            found
                .issues
                .iter()
                .any(|issue| issue.code == "CONTENT_OUTSIDE_METADATA"),
            "{:?}",
            found.issues
        );
    }

    #[test]
    fn our_own_miyin_namespace_is_not_a_game_root() {
        // 导出包会把补丁原文塞进 Miyin/patches/…（里面可能有 Mods/…）——
        // 那不是包内容，不能当成游戏根
        let found = layout(
            Some(""),
            &[
                "metadata.json",
                "paiur01.SC2Map",
                "Miyin/patches/p/Mods/Extra.SC2Mod",
            ],
        );
        assert_eq!(found.content_root, "");
        assert!(!found.game_root_mirror);
        assert!(found.issues.is_empty(), "{:?}", found.issues);
    }
}
