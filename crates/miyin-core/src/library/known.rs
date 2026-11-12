//! **知名复刻战役的入口地图**：数据在 `crates/miyin-core/assets/known_campaigns.json`，
//! 这里只负责匹配。
//!
//! # 为什么打表
//!
//! 这些战役是固定的、作者是固定的、入口地图的文件名也是固定的 ——
//! 表比启发式可靠得多：不会把某一版的分组目录误判，也不会因为作者改了个
//! 不起眼的名字就失效。
//!
//! # 数据文件
//!
//! 预设数据是 `crates/miyin-core/assets/known_campaigns.json`，构建时 `include_str!`
//! 进 exe —— 绿色版只有一个 exe，数据跟着版本走，更新器不用碰 `data/`。
//! **加一条预设 = 往 JSON 的 `campaigns` 数组追加一段，不要改这个文件里的逻辑**：
//!
//! ```json
//! {
//!   "name": "给用户看的名字",
//!   "signatures": ["特征目录/", "特征文件名.SC2Map"],
//!   "launcher": "某某 Campaign Launcher.SC2Map",
//!   "note": "实证：在哪见过这个包、入口脚本怎么写"
//! }
//! ```
//!
//! - `signatures`：地图路径里出现**任意一条**就算（用 `/` 分隔，跟内部表示一致）。
//! - `launcher`：入口地图的**文件名**（不带路径 —— 同一个名字可能出现在多处）。
//! - `note`：给维护者留的实证说明，代码不读。**每条都要有实证**，不确定的不要填 ——
//!   填错了比不填更糟，用户会以为启动器认对了。
//! - 数组顺序 = 优先级；签名不要跨条目重复。
//!
//! 内嵌文件解析失败时退化成空表（认不出入口 = 让用户自己挑，不 panic），
//! 由 `the_embedded_preset_file_is_well_formed` 测试兜底。

use std::sync::LazyLock;

use serde::Deserialize;

use super::MapEntry;

/// 内嵌的预设数据；构建期进 exe，运行期只解析一次。
const PRESETS_JSON: &str = include_str!("../../assets/known_campaigns.json");

/// 一条已知的复刻战役。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownRemake {
    /// 给用户看的名字。
    pub name: String,
    /// 认包的依据：地图路径里出现其中**任意一条**就算。
    pub signatures: Vec<String>,
    /// 入口地图的文件名（不带路径）。
    pub launcher: String,
    /// 维护者留的实证说明；代码不读，存在数据文件里跟着条目走。
    #[serde(default)]
    pub note: String,
}

/// 数据文件的外层结构。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    campaigns: Vec<KnownRemake>,
}

/// 解析好的表。
///
/// 内嵌数据坏了就退化成空表 —— 认不出入口只是让用户自己挑一张，比 panic 好；
/// 数据本身的合法性由测试保证。
static PRESETS: LazyLock<Vec<KnownRemake>> = LazyLock::new(|| {
    serde_json::from_str::<PresetFile>(PRESETS_JSON)
        .map(|file| file.campaigns)
        .unwrap_or_default()
});

/// 从地图列表里认出这是哪部知名复刻的入口；认不出返回 `None`。
///
/// 返回的是**地图路径**（相对版本目录），跟 `MapEntry::path` 一致。
pub fn find_launcher(maps: &[MapEntry]) -> Option<(String, &'static KnownRemake)> {
    for remake in PRESETS.iter() {
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
                last.eq_ignore_ascii_case(&remake.launcher)
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

    /// 内嵌的 JSON 至少要能解析、每条都填齐 —— 预设只改数据文件，这份测试就是
    /// 数据文件的校验；加条目时不用再写一条功能测试。
    #[test]
    fn the_embedded_preset_file_is_well_formed() {
        assert!(!PRESETS.is_empty(), "预设文件不能为空");

        // 签名重复会让「谁先命中」变成隐式的数组顺序问题
        let mut signatures = std::collections::HashSet::new();
        for remake in PRESETS.iter() {
            assert!(!remake.name.trim().is_empty(), "预设缺少名字");
            assert!(
                !remake.launcher.trim().is_empty(),
                "{} 缺少入口地图",
                remake.name
            );
            assert!(
                !remake.launcher.contains('/') && !remake.launcher.contains('\\'),
                "{} 的 launcher 只能写文件名，不能带路径：{}",
                remake.name,
                remake.launcher
            );
            assert!(!remake.signatures.is_empty(), "{} 没有签名", remake.name);
            for signature in &remake.signatures {
                assert!(!signature.trim().is_empty(), "{} 有空签名", remake.name);
                assert!(
                    signatures.insert(signature.clone()),
                    "签名重复，命中谁会变成隐式顺序问题：{signature}"
                );
            }
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

        let (path, remake) = find_launcher(&maps).expect("该认出复刻战役");
        assert_eq!(
            path, "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
            "认出的必须是**它自己的**入口，不是章节里那个 Enslavers 的"
        );
        assert_eq!(remake.name, "星际争霸：复刻战役");
    }

    #[test]
    fn the_shallowest_copy_wins() {
        // 入口地图同名出现在多处时，真正的入口在**浅层**，副本在 Extras/ 那种地方。
        // 复刻战役的包里就有 Extras/ 一份。
        let maps = vec![
            map("Starcraft Mass Recall/Extras/SCMR Campaign Launcher.SC2Map"),
            map("Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map"),
        ];

        let (path, _) = find_launcher(&maps).expect("该认出复刻战役");
        assert_eq!(
            path, "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map",
            "取路径最短的那个"
        );
    }

    /// 章节里的启动器**不是**另一个要打表的战役。
    #[test]
    fn a_chapter_launcher_does_not_become_its_own_entry() {
        let maps = vec![map(
            "Starcraft Mass Recall/8. Enslavers Redux/Enslavers Redux Campaign Launcher.SC2Map",
        )];

        // 这个包里要是没有复刻战役自己的入口，就不该硬认
        assert!(
            find_launcher(&maps).is_none(),
            "章节启动器不该被当成一个独立的知名战役"
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
