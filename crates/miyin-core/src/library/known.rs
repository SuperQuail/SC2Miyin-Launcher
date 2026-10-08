//! **知名复刻战役的入口地图**，打表。
//!
//! # 为什么打表
//!
//! 这些战役是固定的、作者是固定的、入口地图的文件名也是固定的 ——
//! 表比启发式可靠得多：不会把某一版的分组目录误判，也不会因为作者改了个
//! 不起眼的名字就失效。新增一个只要加一行。
//!
//! 表里每一条都要有**实证**：真的见过这个包、或者真的读过它的启动器脚本。
//! 不确定的名字不要往里填 —— 填错了比不填更糟，用户会以为启动器认对了。
//!
//! # 加一条要什么
//!
//! ```text
//! KnownRemake {
//!     // 给用户看的名字
//!     name: "某某复刻",
//!     // 认包的依据：地图路径里出现**任意一条**就算（用 / 分隔，跟内部表示一致）
//!     signatures: &["特征目录/", "特征文件名.SC2Map"],
//!     // 入口地图的**文件名**（不带路径 —— 同一个名字可能出现在多处）
//!     launcher: "某某 Campaign Launcher.SC2Map",
//! }
//! ```

use super::MapEntry;

/// 一条已知的复刻战役。
pub struct KnownRemake {
    /// 给用户看的名字。
    pub name: &'static str,
    /// 认包的依据：地图路径里出现其中**任意一条**就算。
    pub signatures: &'static [&'static str],
    /// 入口地图的文件名（不带路径）。
    pub launcher: &'static str,
}

/// 表。
///
/// ## 星际争霸：大规模召回（Starcraft Mass Recall / SCMR）
///
/// 星际争霸 1 的完整复刻。实证：包根有 `Starcraft Mass Recall/` 目录，
/// 里面分章节放地图，入口是同目录下的 `SCMR Campaign Launcher.SC2Map`。
/// 它的启动器脚本里写的是 `GameSetNextMap("Starcraft Mass Recall/…")`。
///
/// ## Enslavers Redux
///
/// SCMR 团队做的星际 1 隐藏战役复刻，随 SCMR 一起分发。
/// 实证：包内有 `8. Enslavers Redux/` 章节目录，
/// 入口是 `Enslavers Redux Campaign Launcher.SC2Map`。
///
/// **注意**：它和 SCMR 在同一个包里 —— 表按签名匹配，两个都会命中。
/// 取**最先匹配**的那条，所以顺序有意义：更具体的放前面。
pub const KNOWN_REMAKES: &[KnownRemake] = &[
    KnownRemake {
        name: "星际争霸：大规模召回",
        // 用**目录前缀**而不是完整路径 —— 作者可能换个文件名，但目录名不会变
        signatures: &["Starcraft Mass Recall/", "SCMRmod.SC2Mod"],
        launcher: "SCMR Campaign Launcher.SC2Map",
    },
    KnownRemake {
        name: "Enslavers Redux",
        signatures: &["Enslavers Redux Campaign Launcher.SC2Map"],
        launcher: "Enslavers Redux Campaign Launcher.SC2Map",
    },
];

/// 从地图列表里认出这是哪部知名复刻的入口；认不出返回 `None`。
///
/// 返回的是**地图路径**（相对版本目录），跟 `MapEntry::path` 一致。
pub fn find_launcher(maps: &[MapEntry]) -> Option<(String, &'static KnownRemake)> {
    for remake in KNOWN_REMAKES {
        // 这个包是不是它：地图路径里出现任意一条签名
        let hit = maps.iter().any(|map| {
            let path = map.path.replace('\\', "/");
            remake
                .signatures
                .iter()
                .any(|signature| path.contains(signature))
        });
        if !hit {
            continue;
        }

        // 是它 —— 入口地图按**文件名**找。
        //
        // 比的是**路径的最后一段**，不是 `MapEntry::name` ——
        // 那个字段是去掉扩展名的 stem（`SCMR Campaign Launcher`），
        // 拿它跟完整文件名比永远不中。这个坑踩过一次：单元测试的假数据
        // 恰好把 name 写成了完整文件名，于是测试过了、真包没过。
        let found = maps
            .iter()
            .filter(|map| {
                let last = map.path.rsplit('/').next().unwrap_or(&map.path);
                last.eq_ignore_ascii_case(remake.launcher)
            })
            .min_by_key(|map| map.path.matches('/').count())
            .map(|map| map.path.clone());

        if found.is_some() {
            return found.map(|path| (path, remake));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个地图条目。
    ///
    /// **name 用去掉扩展名的 stem** —— 跟真实的 `variant_maps` 一致。
    /// 早先这里图省事写成完整文件名，结果测试过了、真包不匹配。
    fn map(path: &str) -> MapEntry {
        let file = path.rsplit('/').next().unwrap_or(path);
        let name = file
            .strip_suffix(".SC2Map")
            .or_else(|| file.strip_suffix(".sc2map"))
            .unwrap_or(file)
            .to_string();

        MapEntry {
            path: path.to_string(),
            name,
            chapter: None,
            size: 0,
            is_main: false,
        }
    }

    #[test]
    fn scmr_launcher_is_found_by_table() {
        // 真实路径（相对包根）：地图都在 Starcraft Mass Recall/ 底下，
        // 而同包里还躺着**另一个战役**的启动器 —— 用户很容易挑错那个。
        let maps = vec![
            map(
                "Starcraft Mass Recall/8. Enslavers Redux/Enslavers Redux Campaign Launcher.SC2Map",
            ),
            map("Starcraft Mass Recall/Extras/Enslavers Redux Campaign Launcher.SC2Map"),
            map("Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map"),
            map("Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map"),
        ];

        let (path, remake) = find_launcher(&maps).expect("该认出 SCMR");
        assert_eq!(
            path, "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
            "认出的必须是**它自己的**入口，不是包里那个第三方战役的"
        );
        assert_eq!(remake.name, "星际争霸：大规模召回");
    }

    #[test]
    fn the_shallowest_copy_wins() {
        // 入口地图同名出现在多处时，真正的入口在**浅层**，副本在 Extras/ 那种地方
        let maps = vec![
            map("8. Enslavers Redux/Extras/Enslavers Redux Campaign Launcher.SC2Map"),
            map("8. Enslavers Redux/Enslavers Redux Campaign Launcher.SC2Map"),
        ];

        let (path, _) = find_launcher(&maps).expect("该认出 Enslavers Redux");
        assert_eq!(
            path, "8. Enslavers Redux/Enslavers Redux Campaign Launcher.SC2Map",
            "取路径最短的那个"
        );
    }

    #[test]
    fn a_plain_package_matches_nothing() {
        // 不认识的包**不许硬套** —— 表里没有就当没这回事
        let maps = vec![
            map("SomeCampaign/1. First/a.SC2Map"),
            map("SomeCampaign/launcher.SC2Map"),
        ];
        assert!(find_launcher(&maps).is_none());
    }
}
