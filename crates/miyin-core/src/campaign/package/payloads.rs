//! 从条目清单里认出载荷（地图 / 模组）：哪些条目算一张地图，它该落到哪里。
//!
//! - **载荷根**：一条路径里第一个以 `.SC2Map` / `.SC2Mod` 结尾的组件。
//!   单文件 MPQ 与解开的目录树都靠这一条认出来（`expanded` 区分）。
//! - **按文件夹去重**：`Mods/Alenger/` 下的 15 个 `.SC2Mod` 是**一个**模组。
//! - **落点**：不在这里判 —— 交给 `paths::payload_target` 的判定表，
//!   这里只把布局事实（内容根、是否游戏目录镜像）传下去。

use std::path::Path;

use super::paths::{payload_target, strip_prefix};
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

/// 从条目清单里找出所有载荷，并算出各自的落点。
pub(crate) fn collect_payloads(
    entries: &[Entry],
    content_root: &str,
    game_root_mirror: bool,
) -> Vec<Payload> {
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
                    .is_some_and(|relative| super::paths::is_under(&relative, &source))
            });
            let target = payload_target(&source, is_mod, content_root, game_root_mirror);
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
    use crate::campaign::package::PayloadTarget;

    fn entry(path: &str) -> Entry {
        Entry {
            index: 0,
            relative: std::path::PathBuf::from(path),
            size: 0,
        }
    }

    #[test]
    fn detects_single_file_payloads() {
        let entries = vec![
            entry("metadata.txt"),
            entry("paiur01.SC2Map"),
            entry("HTXL.SC2Mod"),
        ];
        let payloads = collect_payloads(&entries, "", false);
        assert_eq!(payloads.len(), 2);

        let map = payloads
            .iter()
            .find(|payload| payload.source == "paiur01.SC2Map")
            .expect("地图");
        assert!(!map.expanded, "单文件 MPQ 不应标记为解开的目录");
        assert!(matches!(
            &map.target,
            PayloadTarget::CampaignMap { name } if name == "paiur01.SC2Map"
        ));

        let module = payloads
            .iter()
            .find(|payload| payload.source == "HTXL.SC2Mod")
            .expect("模组");
        assert!(matches!(
            &module.target,
            PayloadTarget::Mod { name } if name == "HTXL.SC2Mod"
        ));
    }

    #[test]
    fn detects_expanded_directory_payloads() {
        // 真实数据里 .SC2Map 经常是一棵解开的目录树
        let entries = vec![
            entry("Maps/Campaign/void/metadata.txt"),
            entry("Maps/Campaign/void/epiloguestory01.SC2Map/"),
            entry("Maps/Campaign/void/epiloguestory01.SC2Map/DocumentHeader"),
            entry("Maps/Campaign/void/epiloguestory01.SC2Map/Base.SC2Data/x.xml"),
        ];
        // 内容根已经落在官方战役目录里：包内相对路径是该战役目录之内的结构
        let payloads = collect_payloads(&entries, "Maps/Campaign/void", false);
        assert_eq!(payloads.len(), 1);

        let payload = &payloads[0];
        assert_eq!(payload.source, "epiloguestory01.SC2Map");
        assert!(payload.expanded, "目录树应标记为 expanded");
        assert!(matches!(
            &payload.target,
            PayloadTarget::CampaignMap { name } if name == "epiloguestory01.SC2Map"
        ));
        assert_eq!(
            crate::library::compose::payload_target_path(
                &payload.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/epiloguestory01.SC2Map")
        );
    }

    #[test]
    fn mirror_game_paths_stay_put_and_mods_go_to_mods() {
        // 游戏目录镜像：Maps/ 与 Mods/ 都已摆好
        let entries = vec![
            entry("Maps/Campaign/void/paiur01.SC2Map"),
            entry("Mods/X.SC2Mod"),
        ];
        let payloads = collect_payloads(&entries, "", true);
        let targets: Vec<String> = payloads.iter().map(Payload::target_path).collect();
        assert_eq!(
            targets,
            vec![
                "Maps/Campaign/void/paiur01.SC2Map".to_string(),
                "Mods/X.SC2Mod".to_string(),
            ]
        );
        assert!(matches!(
            &payloads[0].target,
            PayloadTarget::GamePath { .. }
        ));
        assert!(matches!(&payloads[1].target, PayloadTarget::Mod { .. }));
    }

    /// 真实样本「机翻版（中文启动）」：包根是游戏目录镜像（`Maps/` + `Mods/`），
    /// metadata.txt 在 `Maps/` 里，地图直接躺在 `Maps/` 根上。
    ///
    /// 地图必须被认成战役地图而不是游戏路径 —— 否则会落到游戏不扫的 `Maps/` 根，
    /// 导入成功、战役却一动不动。
    #[test]
    fn flat_maps_beside_metadata_inside_a_game_mirror_are_campaign_maps() {
        let entries = vec![
            entry("Maps/metadata.txt"),
            entry("Maps/thanson01.SC2Map"),
            entry("Mods/SCEvo_Core.SC2Mod"),
        ];
        let payloads = collect_payloads(&entries, "", true);

        let map = payloads
            .iter()
            .find(|payload| !payload.is_mod)
            .expect("地图");
        assert_eq!(map.source, "Maps/thanson01.SC2Map");
        assert!(matches!(
            &map.target,
            PayloadTarget::CampaignMap { name } if name == "thanson01.SC2Map"
        ));
        assert_eq!(
            crate::library::compose::payload_target_path(
                &map.target,
                &crate::library::compose::Placement::Campaign { sub: None },
            )
            .as_deref(),
            Some("Maps/Campaign/thanson01.SC2Map")
        );

        let module = payloads
            .iter()
            .find(|payload| payload.is_mod)
            .expect("模组");
        assert_eq!(module.target_path(), "Mods/SCEvo_Core.SC2Mod");
    }

    #[test]
    fn nested_mod_directories_are_kept() {
        // Alenger/ 下 15 个 .SC2Mod 是一个模组；结构要保留（依赖里写的就是它）
        let entries = vec![
            entry("Mods/Alenger/1钢铁.SC2Mod"),
            entry("Mods/Alenger/2贝希摩斯.SC2Mod"),
            entry("Mods/单文件.SC2Mod"),
        ];
        let payloads = collect_payloads(&entries, "", true);
        let targets: Vec<String> = payloads.iter().map(Payload::target_path).collect();
        assert!(targets.contains(&"Mods/Alenger/1钢铁.SC2Mod".to_string()));
        assert!(targets.contains(&"Mods/单文件.SC2Mod".to_string()));
    }

    #[test]
    fn loads_of_kinds_are_counted_by_payload_not_by_extension() {
        let entries = vec![
            entry("Maps/Campaign/void/a.SC2Map/DocumentHeader"),
            entry("Maps/Campaign/void/b.SC2Map/DocumentHeader"),
            entry("Mods/Alenger/1钢铁.SC2Mod"),
            entry("Mods/Alenger/2贝希摩斯.SC2Mod"),
        ];
        let payloads = collect_payloads(&entries, "", true);
        let (maps, mods) = count_payloads(&payloads);
        assert_eq!(maps, 2, "散成目录树的地图也要算");
        assert_eq!(mods, 1, "Alenger/ 下的两个文件是一个模组");
    }
}
