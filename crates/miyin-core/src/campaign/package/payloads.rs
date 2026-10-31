//! 从条目清单里认出载荷（地图 / 模组）：哪些条目算一张地图，它该落到哪里。
//!
//! 这里也是「按文件夹去重」的地方 —— Mods/Alenger/ 下的 15 个 .SC2Mod 是**一个**模组。

use std::path::Path;

use super::paths::{canonical_game_dir, is_under, payload_target, strip_prefix};
use super::types::{Entry, Payload};

/// 这条路径是不是落在我们自己的附加数据目录里。
pub fn is_our_metadata_dir(relative: &Path) -> bool {
    relative
        .components()
        .next()
        .map(|first| {
            first
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(crate::library::export::MIYIN_DIR)
        })
        .unwrap_or(false)
}

/// 取相对路径里的载荷根：第一个以 `.SC2Map` / `.SC2Mod` 结尾的组件（含它自己）。
fn payload_root(relative: &Path) -> Option<(String, bool)> {
    let mut parts: Vec<String> = Vec::new();

    for component in relative.components() {
        let name = component.as_os_str().to_string_lossy().into_owned();
        parts.push(name.clone());
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".sc2mod") {
            return Some((parts.join("/"), true));
        }
        if lower.ends_with(".sc2map") {
            return Some((parts.join("/"), false));
        }
    }

    None
}

/// 包内是否直接就是游戏目录镜像（顶层出现 `Maps` 或 `Mods`）。
pub(crate) fn has_mirror_root(entries: &[Entry]) -> bool {
    entries.iter().any(|entry| {
        entry
            .relative
            .components()
            .next()
            .map(|first| {
                // 与 `payload_target` 一致，但**不区分大小写**：`maps/` 在 Windows 上就是 `Maps/`。
                // Interfaces 也算 —— 白名单早就放行它，但这里一直不认，
                // 结果是界面 Mod 装不进去（v3 要补的第一个缺口）。
                let name = first.as_os_str().to_string_lossy();
                canonical_game_dir(&name).is_some()
            })
            .unwrap_or(false)
    })
}

/// 这个包是不是**按游戏根目录摆的**。
///
/// 判据：包根直接有个 `Mods/`（不是包名目录下面的）。
///
/// 真实样本教我们的 —— 复刻战役 SCMR 长这样：
///
/// `@text
/// Mods/SCMRassets.SC2Mod          <- 已经在游戏根的位置上了
/// Mods/SCMRmod.SC2Mod
/// Starcraft Mass Recall/          <- 那这个兄弟目录就是 Maps/ 底下的
/// ├── 1. Rebel Yell/Terran01.SC2Map
/// └── SCMR Campaign Launcher.SC2Map
/// `@
///
/// 它的启动器地图里写的是 `GameSetNextMap("Starcraft Mass Recall/1. Rebel Yell/Terran01")`
/// —— 这个路径**相对 Maps/**。所以装完必须长成
/// `Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map`。
///
/// 踩过：这些地图被当成「作者的分类目录」拍平成 `Maps/Campaign/Terran01.SC2Map`，
/// 层级一没，启动器地图就联动不了任何关卡 —— 用户看到的就是
/// 「能打开启动器，但点哪一关都进不去」。
///
/// `Mods/` 是作者给出来的**信号**，不是我们在猜。
fn mirrors_game_root(entries: &[Entry]) -> bool {
    entries.iter().any(|entry| {
        let path = entry.relative.to_string_lossy().replace('\\', "/");
        let first = path.split('/').next().unwrap_or_default();
        path.len() > 4 && canonical_game_dir(first) == Some("Mods")
    })
}

/// 从条目清单里找出所有载荷，并算出各自的落点。
///
/// 取"根"的办法：一条路径里**第一个**以 `.SC2Map` / `.SC2Mod` 结尾的组件就是载荷根，
/// 这样无论它是单文件还是解开的目录树都能正确识别。
pub(crate) fn collect_payloads(entries: &[Entry], content_root: &str) -> Vec<Payload> {
    // 这个包是不是按游戏根目录摆的（根上有 Mods/）—— 决定地图要不要保留目录结构
    let mirrors = mirrors_game_root(entries);
    let mut roots: Vec<(String, bool)> = Vec::new();

    for entry in entries {
        let Some(relative) = strip_prefix(&entry.relative, content_root) else {
            continue;
        };
        // `Miyin/` 是我们自己的附加数据目录（导出时会写上）——
        // 它不是战役内容，重新导入时不能被当成载荷。
        if is_our_metadata_dir(&relative) {
            continue;
        }
        let Some(root) = payload_root(&relative) else {
            continue;
        };
        if !roots.iter().any(|(existing, _)| existing == &root.0) {
            roots.push(root);
        }
    }

    roots
        .into_iter()
        .map(|(source, is_mod)| {
            let expanded = entries.iter().any(|entry| {
                strip_prefix(&entry.relative, content_root)
                    .is_some_and(|relative| is_under(&relative, &source))
            });
            let target = payload_target(&source, is_mod, content_root, mirrors);
            Payload {
                source,
                target,
                expanded,
                is_mod,
            }
        })
        .collect()
}

/// 按**载荷**统计地图与模组数量。
///
/// 不能按文件扩展名数：真实样本里的地图是**解开的目录树**
/// （`tarcade.SC2Map/Base.SC2Data/…`），里面的文件是 `.xml` / `.galaxy`，
/// 按扩展名数会得到「0 张地图」。载荷是按「第一个 .SC2Map/.SC2Mod 组件」认出来的，
/// 单文件和目录树都算一个。
///
/// 模组还要**按文件夹去重**：`Mods/Alenger/` 下面是 15 个 `.SC2Mod`，
/// 那是**一个**模组，不是 15 个。
pub fn count_payloads(payloads: &[Payload]) -> (usize, usize) {
    let maps = payloads.iter().filter(|payload| !payload.is_mod).count();

    let mut keys: Vec<String> = Vec::new();
    for payload in payloads.iter().filter(|payload| payload.is_mod) {
        if let Some(found) = crate::library::mod_identity(payload)
            && !keys.contains(&found.key)
        {
            keys.push(found.key);
        }
    }

    (maps, keys.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::identify::{self, CampaignEvidence};
    use crate::campaign::metadata::CampaignType;
    use crate::campaign::package::PayloadTarget;
    use std::path::PathBuf;

    fn entry(path: &str, _is_dir: bool) -> Entry {
        Entry {
            index: 0,
            relative: PathBuf::from(path),
            size: 0,
        }
    }

    #[test]
    fn detects_single_file_payloads() {
        let entries = vec![
            entry("metadata.txt", false),
            entry("paiur01.SC2Map", false),
            entry("HTXL.SC2Mod", false),
        ];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 2);

        let map = payloads
            .iter()
            .find(|p| p.source == "paiur01.SC2Map")
            .expect("map");
        assert!(!map.expanded, "单文件 MPQ 不应标记为解开的目录");
        assert!(matches!(&map.target, PayloadTarget::Map { name } if name == "paiur01.SC2Map"));

        let mode = payloads
            .iter()
            .find(|p| p.source == "HTXL.SC2Mod")
            .expect("mod");
        assert!(matches!(&mode.target, PayloadTarget::Mod { name } if name == "HTXL.SC2Mod"));
    }

    #[test]
    fn detects_expanded_directory_payloads() {
        // 真实数据里 .SC2Map 经常是一棵解开的目录树
        let entries = vec![
            entry("Maps/Campaign/void/metadata.txt", false),
            entry("Maps/Campaign/void/epiloguestory01.SC2Map/", true),
            entry(
                "Maps/Campaign/void/epiloguestory01.SC2Map/DocumentHeader",
                false,
            ),
            entry(
                "Maps/Campaign/void/epiloguestory01.SC2Map/Base.SC2Data/x.xml",
                false,
            ),
        ];
        let payloads = collect_payloads(&entries, "Maps/Campaign/void");
        assert_eq!(payloads.len(), 1);

        let payload = &payloads[0];
        assert_eq!(payload.source, "epiloguestory01.SC2Map");
        assert!(payload.expanded, "目录树应标记为 expanded");
        // 包内已经是游戏目录镜像，落点保持原路径
        assert!(payload.target_name().starts_with("epiloguestory01.SC2Map"));
    }

    #[test]
    fn mirror_paths_are_preserved() {
        let entries = vec![entry("Mods/Foo.SC2Mod/Base.SC2Data/a.xml", false)];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0].target_path(),
            "Mods/Foo.SC2Mod",
            "镜像路径应当原样保留"
        );
    }

    #[test]
    fn identifies_packages_by_mirror_path_before_falling_back() {
        // 镜像路径是"精确"证据，应当优先于文件名启发式
        let entries = vec![
            entry("Maps/Campaign/void/paiur01.SC2Map", false),
            entry("Maps/Campaign/void/pkorhal01.SC2Map", false),
        ];
        let payloads = collect_payloads(&entries, "");
        let mirror: Vec<String> = payloads.iter().map(|p| p.target_path()).collect();
        let found = identify::from_mirror_paths(&mirror).expect("应当按镜像路径判定");
        assert_eq!(found.campaign_type, CampaignType::Lotv);
        assert_eq!(found.evidence, CampaignEvidence::MirrorPath);
        assert!(found.evidence.is_exact());
    }

    #[test]
    fn falls_back_to_map_name_prefixes() {
        let types = collect_payloads(
            &[
                entry("paiur01.SC2Map", false),
                entry("pkorhal01.SC2Map", false),
                entry("ppurifier02.SC2Map", false),
            ],
            "",
        );
        let names: Vec<String> = types.iter().map(|p| p.target_name()).collect();
        let found = identify::from_map_names(&names).expect("应当按文件名判定");
        assert_eq!(found.campaign_type, CampaignType::Lotv);
        assert_eq!(found.evidence, CampaignEvidence::MapNamePrefix);
        assert!(!found.evidence.is_exact(), "文件名只是启发式，不算精确证据");

        let hots = collect_payloads(
            &[
                entry("zchar01.SC2Map", false),
                entry("zlab02.SC2Map", false),
            ],
            "",
        );
        let names: Vec<String> = hots.iter().map(|p| p.target_name()).collect();
        assert_eq!(
            identify::from_map_names(&names).map(|found| found.campaign_type),
            Some(CampaignType::Hots)
        );

        // 分不清时必须返回 None，交给界面问用户
        let mixed = collect_payloads(
            &[
                entry("paiur01.SC2Map", false),
                entry("zchar01.SC2Map", false),
            ],
            "",
        );
        let names: Vec<String> = mixed.iter().map(|p| p.target_name()).collect();
        assert!(identify::from_map_names(&names).is_none());

        // 自定义命名认不出来，但也不算错
        let unknown = collect_payloads(&[entry("mymap.SC2Map", false)], "");
        let names: Vec<String> = unknown.iter().map(|p| p.target_name()).collect();
        assert!(identify::from_map_names(&names).is_none());
    }

    #[test]
    fn keeps_relative_directories_inside_the_campaign_folder() {
        // 真实样本「酒馆合作虫心」：内容根就是 X/swarm，进化地图在 evolution/ 下。
        // 这一层必须保留，否则进化地图会被摆到虫心主目录，游戏里找不到。
        let entries = vec![
            entry("酒馆合作虫心/swarm/metadata.txt", false),
            entry("酒馆合作虫心/swarm/zchar01.SC2Map", false),
            entry(
                "酒馆合作虫心/swarm/evolution/zevolutionbaneling.SC2Map",
                false,
            ),
        ];
        let payloads = collect_payloads(&entries, "酒馆合作虫心/swarm");

        let plain = payloads
            .iter()
            .find(|p| p.source == "zchar01.SC2Map")
            .expect("普通地图");
        assert_eq!(plain.target_name(), "zchar01.SC2Map");

        let evolution = payloads
            .iter()
            .find(|p| p.source.contains("evolution"))
            .expect("进化地图");
        assert_eq!(
            evolution.target_name(),
            "evolution/zevolutionbaneling.SC2Map",
            "战役目录之内的相对结构要原样保留"
        );
        assert_eq!(
            crate::library::compose::payload_target_path(
                &evolution.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("swarm".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/swarm/evolution/zevolutionbaneling.SC2Map")
        );
    }

    #[test]
    fn recognises_official_campaign_directories_inside_the_package() {
        // 真实样本「净化者纪元幼儿园」：内容根是包根，序章地图放在 voidprologue/。
        // voidprologue 是官方目录名，说明作者按游戏结构摆好了，落点直接用。
        let entries = vec![
            entry("metadata.txt", false),
            entry("paiur01.SC2Map", false),
            entry("voidprologue/voidprologue01.SC2Map", false),
        ];
        let payloads = collect_payloads(&entries, "");

        let main = payloads
            .iter()
            .find(|p| p.source == "paiur01.SC2Map")
            .expect("主线地图");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &main.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/paiur01.SC2Map")
        );

        let prologue = payloads
            .iter()
            .find(|p| p.source.contains("voidprologue"))
            .expect("序章地图");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &prologue.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/voidprologue/voidprologue01.SC2Map"),
            "官方目录名要当绝对路径用，而不是塞进 void/ 下面"
        );
    }

    #[test]
    fn arbitrary_container_folders_are_flattened() {
        // 作者的分类习惯（maps/、files/…）没有游戏语义，只取文件名 ——
        // 否则地图会落到游戏根本不扫描的目录里
        let entries = vec![entry("metadata.txt", false), entry("maps/01.SC2Map", false)];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].target_name(), "01.SC2Map");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &payloads[0].target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/01.SC2Map")
        );
    }
    #[test]
    fn a_lowercase_maps_folder_is_the_same_as_maps() {
        // 用户报的那个：包里的文件夹叫 maps，Windows 上它就是 Maps。
        // 以前按大小写敏感比，认不出来，整包被塞成 Maps/maps/…，游戏找不到地图。
        let entries = vec![
            entry("metadata.txt", false),
            entry("maps/Campaign/void/01.SC2Map", false),
        ];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            crate::library::compose::payload_target_path(
                &payloads[0].target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/01.SC2Map"),
            "小写 maps/ 要当 Maps/ 用，而且不许再套一层 maps"
        );

        // 但平坦的 maps/01 仍然是作者的分类习惯 —— 照旧拍平到能扫到的位置，
        // 别为了修大小写把这类包送进 Maps 根目录。
        let flat = vec![entry("metadata.txt", false), entry("maps/01.SC2Map", false)];
        let flat_payloads = collect_payloads(&flat, "");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &flat_payloads[0].target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/01.SC2Map")
        );
    }
}
