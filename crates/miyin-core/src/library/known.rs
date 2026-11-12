//! **知名复刻战役的入口地图**：数据在 `crates/miyin-core/assets/known_campaigns.json`，
//! 这里只负责匹配。
//!
//! # 优先级
//!
//! 表是**有序**的，靠前的先命中（加新预设 = 在 JSON 里插到正确的顺序）：
//!
//! 1. **群雄纪元**（`SCMR RPG Campaign Launcher`）—— 复刻战役的社区改版，优先于原版复刻；
//! 2. **星际争霸：复刻战役**（`Starcraft Mass Recall/` + `SCMR Campaign Launcher`）；
//! 3. **通用兜底**（`Launcher`）—— 前面都不中才用它。
//!
//! 没有 `signatures` 的条目对**任何包**都成立，只看包里有没有这张入口地图。
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
//!   "signatures": ["特征目录/"],
//!   "launcher": "某某 Campaign Launcher",
//!   "note": "实证：在哪见过这个包、入口脚本怎么写"
//! }
//! ```
//!
//! - `signatures`：地图路径里出现**任意一条**就算（用 `/` 分隔，跟内部表示一致）；
//!   没有这个字段 = 对任何包都成立。
//! - `launcher`：入口地图的**文件名**（不带路径）。**不带 `.SC2Map` 后缀**，比较时忽略大小写。
//! - `root_only`：只认**包根**那张入口地图。复刻战役的入口在子目录里，所以默认任意层级；
//!   通用兜底才收窄到包根（章节里叫 `Launcher` 的地图不是整部战役的入口）。
//! - 只认 `.SC2Map` 文件；`.SC2Mod` 之类不参与打表。
//! - `note`：给维护者留的实证说明，代码不读。**每条都要有实证**，不确定的不要填 ——
//!   填错了比不填更糟，用户会以为启动器认对了。
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
    /// 认包的依据：地图路径里出现其中**任意一条**就算；为空表示对任何包都成立。
    #[serde(default)]
    pub signatures: Vec<String>,
    /// 入口地图的文件名（不带路径、**不带 `.SC2Map` 后缀**）。
    pub launcher: String,
    /// 是不是只认**包根**那张入口地图。
    ///
    /// 复刻战役的入口就在 `Starcraft Mass Recall/` 里，所以默认任意层级；
    /// 通用兜底必须收窄到包根 —— 章节里叫 `Launcher` 的地图不该被当成整部战役的入口。
    #[serde(default)]
    pub root_only: bool,
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

/// 路径最后一段是 `.SC2Map` 时返回去掉扩展名的文件名；其它扩展名一律不参与打表。
fn sc2map_stem(path: &str) -> Option<&str> {
    let last = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let (stem, extension) = last.rsplit_once('.')?;
    extension.eq_ignore_ascii_case("sc2map").then_some(stem)
}

/// 从地图列表里认出这是哪部知名复刻的入口；认不出返回 `None`。
///
/// 返回的是**地图路径**（相对版本目录），跟 `MapEntry::path` 一致。
pub fn find_launcher(maps: &[MapEntry]) -> Option<(String, &'static KnownRemake)> {
    for remake in PRESETS.iter() {
        // 这个包是不是它：地图路径里出现任意一条签名；没有签名 = 任何包都算。
        let hit = remake.signatures.is_empty()
            || maps.iter().any(|map| {
                let path = map.path.replace('\\', "/");
                sc2map_stem(&map.path).is_some()
                    && remake
                        .signatures
                        .iter()
                        .any(|signature| path.contains(signature))
            });
        if !hit {
            continue;
        }

        // 是它 —— 入口地图按**文件名**找（数据里不带后缀，比的就是去掉扩展名的文件名）。
        //
        // 比的是**路径的最后一段**，不是 `MapEntry::name` ——
        // 那个字段是去掉扩展名的 stem（`SCMR Campaign Launcher`），
        // 拿它跟完整文件名比永远不中。这个坑踩过一次：单元测试的假数据
        // 恰好把 name 写成了完整文件名，于是测试过了、真包没过。
        let found = maps
            .iter()
            .filter(|map| {
                if remake.root_only && map.path.contains(['/', '\\']) {
                    return false;
                }
                sc2map_stem(&map.path)
                    .is_some_and(|stem| stem.eq_ignore_ascii_case(&remake.launcher))
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
            assert!(
                !remake.launcher.to_ascii_lowercase().ends_with(".sc2map"),
                "{} 的 launcher 不需要 .SC2Map 后缀：{}",
                remake.name,
                remake.launcher
            );
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
    fn the_rpg_launcher_wins_over_the_base_one() {
        // 群雄纪元是复刻战役的社区改版：同一个包里两张启动器都可能躺着，
        // 打表顺序保证先认群雄纪元。
        let maps = vec![
            map("Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map"),
            map("Starcraft Mass Recall/SCMR RPG Campaign Launcher.SC2Map"),
            map("Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map"),
        ];

        let (path, remake) = find_launcher(&maps).expect("该认出群雄纪元");
        assert_eq!(
            path,
            "Starcraft Mass Recall/SCMR RPG Campaign Launcher.SC2Map"
        );
        assert_eq!(remake.name, "群雄纪元");
    }

    /// 通配兜底：不认识任何预设时，包里那张 `Launcher` 就是入口（不区分大小写）。
    #[test]
    fn the_generic_launcher_is_the_last_resort_and_ignores_case() {
        let maps = vec![map("1. First/a.SC2Map"), map("Launcher.SC2Map")];
        let (path, remake) = find_launcher(&maps).expect("最兜底应认出 Launcher");
        assert_eq!(path, "Launcher.SC2Map");
        assert_eq!(remake.name, "通用入口（兜底）");

        // 大小写不同也认
        let lower = vec![map("LAUNCHER.SC2Map")];
        assert_eq!(
            find_launcher(&lower).map(|(path, _)| path).as_deref(),
            Some("LAUNCHER.SC2Map")
        );

        // 兜底只认包根：章节里叫 Launcher 的地图不是整部战役的入口
        let chapter = vec![map("1. Rebel Yell/Launcher.SC2Map")];
        assert!(
            find_launcher(&chapter).is_none(),
            "章节里的 Launcher 不该被当成入口"
        );

        // 已知预设比它优先（同一条链上更靠前）
        let known_wins = vec![
            map("Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map"),
            map("Starcraft Mass Recall/Launcher.SC2Map"),
        ];
        let (path, remake) = find_launcher(&known_wins).expect("该认出复刻战役");
        assert_eq!(path, "Starcraft Mass Recall/SCMR Campaign Launcher.SC2Map");
        assert_eq!(remake.name, "星际争霸：复刻战役");
    }

    /// 只有 `.SC2Map` 参与打表 —— 旧表里的 `SCMRmod.SC2Mod` 签名就是这么变成死数据的。
    #[test]
    fn only_sc2map_files_participate() {
        let maps = vec![
            map("Starcraft Mass Recall/SCMRmod.SC2Mod"),
            map("Launcher.SC2Mod"),
        ];
        assert!(
            find_launcher(&maps).is_none(),
            "模组 / 非地图文件不该被当成入口"
        );
    }

    /// 每条预设都要能被自己的入口认出来；以后往 JSON 里加条目，这一条自动覆盖。
    #[test]
    fn every_preset_can_be_recognized_from_its_own_launcher() {
        for remake in PRESETS.iter() {
            let mut maps: Vec<MapEntry> = remake
                .signatures
                .iter()
                .filter(|signature| signature.ends_with('/'))
                .map(|signature| map(&format!("{signature}Chapter/01.SC2Map")))
                .collect();
            let launcher_path = if remake.root_only {
                format!("{}.SC2Map", remake.launcher)
            } else {
                format!("Pack/{}.SC2Map", remake.launcher)
            };
            maps.push(map(&launcher_path));

            let found = find_launcher(&maps)
                .unwrap_or_else(|| panic!("{} 没被自己的入口认出来", remake.name));
            assert_eq!(
                found.1.name, remake.name,
                "{} 被更靠前的预设抢走了",
                remake.name
            );
            assert_eq!(found.0, launcher_path);
        }
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
        // 不认识的包、也没有兜底的 Launcher —— 不许硬套
        let maps = vec![
            map("SomeCampaign/1. First/a.SC2Map"),
            map("SomeCampaign/2. Second/b.SC2Map"),
        ];
        assert!(find_launcher(&maps).is_none());
    }
}
