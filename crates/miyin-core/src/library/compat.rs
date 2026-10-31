//! **两套安装约定的兼容性对比**：同一个包，按我们装 vs 按枢纽（CCM）装，
//! 各自有哪些文件会落进**游戏会扫描的目录**。
//!
//! 为什么需要它：纸面上两边都"兼容 CCM 格式"，但**落点约定并不一样** ——
//! 我们按包内声明的 `campaign` 把官方改版放进 `Maps/Campaign/…`，
//! 而枢纽那一套**不看声明**，一律当自制战役塞进 `Maps/CustomCampaigns/<名字>/`。
//! 同一个包两边算出来的路径不同，能不能被游戏认出来也就可能不同。
//! 这个模块把那笔账逐文件算清楚，而不是让人凭印象猜"应该兼容吧"。
//!
//! # 两套约定
//!
//! | | ours（弥音） | nexus（枢纽 / CCM） |
//! | --- | --- | --- |
//! | 官方改版（`campaign=wol`…） | `Maps/Campaign[/子目录]/` | `Maps/CustomCampaigns/<名字>/` |
//! | 自制战役（`campaign=custom`） | `Maps/CustomCampaigns/<名字>/` | `Maps/CustomCampaigns/<名字>/` |
//! | 模组 | `Mods/<名字>` | `Mods/<名字>` |
//! | 镜像载荷（`PayloadTarget::Mirror`） | 按包内写死的路径 | 同样按包内写死的路径 |
//!
//! 两套都走同一份落点规则（[crate::library::compose]），这里只是**喂不同的 placement** ——
//! 不另发明一套规则（见 `docs/package-format.md` §1.4 与 AGENTS.md §14.5）。
//!
//! # `recognized` 是什么
//!
//! **落点是不是在游戏会扫描的目录里**：`Maps/Campaign/**`、`Maps/CustomCampaigns/**`、`Mods/**`。
//! 不在（被摆进 `Maps/` 下别的目录、`SC2Data/` 之类）就是 `false`。
//! 这个判据**只看目录**，不看文件是不是真的能跑 —— 那是另一个问题。
//!
//! 注意与写盘闸门区分开：闸门放行的是"安装目录里任意位置"（AGENTS.md §10.5），
//! 那是**允许写**的边界；这里是**游戏认不认**的边界，两者不是一回事。

use serde::Serialize;

use crate::campaign::metadata::CampaignType;
use crate::campaign::package::{PackageInspection, PayloadTarget};
use crate::library::compose::{Placement, payload_target_path};

/// 游戏会扫描的目录根（判断"认不认"的唯一依据）。
const SCANNED_ROOTS: [&str; 3] = ["Maps/Campaign", "Maps/CustomCampaigns", "Mods"];

/// 某个载荷在**一套**约定下的落点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConventionPlacement {
    /// 游戏目录内的相对路径；`None` 表示这套约定下它压根不进游戏目录。
    pub target: Option<String>,
    /// 落点是否在游戏会扫描的目录里。
    pub recognized: bool,
    /// 为什么认 / 不认；两套结果不同时也会给一句。
    pub why: String,
}

/// 一个载荷在两套约定下的落点对照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompatFile {
    /// 包内相对内容根的路径。
    pub source: String,
    /// 载荷落点类型：`mirror` / `mod` / `map`。
    pub kind: String,
    /// 弥音约定下的落点。
    pub ours: ConventionPlacement,
    /// 枢纽（CCM）约定下的落点。
    pub nexus: ConventionPlacement,
}

/// 一套约定的汇总。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ConventionSummary {
    pub recognized: usize,
    pub missed: usize,
}

/// 兼容性报告的汇总。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CompatSummary {
    pub total: usize,
    pub ours: ConventionSummary,
    pub nexus: ConventionSummary,
    /// 值得提一句的地方（没认出归属、两套结论相反……）。
    pub warnings: Vec<String>,
}

/// 一份兼容性报告。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompatReport {
    /// 我们认出来的归属槽位；`None` = 认不出来（照样给两套落点，不猜）。
    pub slot: Option<String>,
    pub files: Vec<CompatFile>,
    pub summary: CompatSummary,
}

/// 算这个包在两套约定下各自的落点与可识别性。
///
/// `folder` 是自制战役在 `Maps/CustomCampaigns/` 下面那层目录名：调用方传包建议的
/// 安装目录名（`PackageInspection::suggested_dir_name`），取不到时退回压缩包文件名。
pub fn analyze(inspection: &PackageInspection, folder: &str) -> CompatReport {
    // 我们这一套：按包内声明的归属算（认不出来就按官方战役的默认落点 —— 反正也装不上）
    let ours = Placement::of(
        inspection.campaign_type.slug(),
        folder,
        inspection.campaign_type.sub_directory(),
    );
    // 枢纽这一套：**不看 campaign**，整包当自制战役
    let nexus = Placement::of(CampaignType::Custom.slug(), folder, None);

    let files: Vec<CompatFile> = inspection
        .payloads
        .iter()
        .map(|payload| {
            let mut ours = place(&payload.target, &ours);
            let mut nexus = place(&payload.target, &nexus);
            ours.why = why(
                ours.recognized,
                nexus.recognized,
                &ours.target,
                &nexus.target,
            );
            nexus.why = why(
                nexus.recognized,
                ours.recognized,
                &nexus.target,
                &ours.target,
            );
            CompatFile {
                source: payload.source.clone(),
                kind: kind_of(&payload.target).to_string(),
                ours,
                nexus,
            }
        })
        .collect();

    let slot = inspection.campaign_type.is_actionable().then(|| {
        inspection
            .campaign_type
            .main_slot()
            .unwrap_or(inspection.campaign_type.slug())
            .to_string()
    });

    CompatReport {
        slot: slot.clone(),
        summary: summarize(&files, slot.as_deref(), &inspection.campaign_type),
        files,
    }
}

/// 这个载荷在一套约定下落哪、认不认。
fn place(target: &PayloadTarget, placement: &Placement) -> ConventionPlacement {
    let target = payload_target_path(target, placement);
    match &target {
        Some(path) => ConventionPlacement {
            recognized: recognized(path),
            why: String::new(),
            target,
        },
        None => ConventionPlacement {
            target: None,
            recognized: false,
            why: "这套约定下它不进游戏目录".to_string(),
        },
    }
}

/// 汇总两套结果，并挑出值得提一句的话。
fn summarize(files: &[CompatFile], slot: Option<&str>, campaign: &CampaignType) -> CompatSummary {
    let count = |pick: fn(&CompatFile) -> bool| files.iter().filter(|file| pick(file)).count();

    let mut warnings = Vec::new();
    match slot {
        None => {
            warnings.push("认不出这个包属于哪部战役：两套落点都只是按默认规则算出来的".to_string())
        }
        // 这正是两套结果的差别所在：官方改版被枢纽当成自制战役
        Some(found) if !campaign.is_custom() => warnings.push(format!(
            "这个包声明成 {found}：枢纽那套不看声明，会把它当自制战役装进 Maps/CustomCampaigns/"
        )),
        Some(_) => {}
    }
    if count(|file| !file.ours.recognized) > 0 {
        warnings.push("按我们的约定有文件落在游戏扫描不到的目录里".to_string());
    }
    if count(|file| !file.nexus.recognized) > 0 {
        warnings.push("按枢纽的约定有文件落在游戏扫描不到的目录里".to_string());
    }

    CompatSummary {
        total: files.len(),
        ours: ConventionSummary {
            recognized: count(|file| file.ours.recognized),
            missed: count(|file| !file.ours.recognized),
        },
        nexus: ConventionSummary {
            recognized: count(|file| file.nexus.recognized),
            missed: count(|file| !file.nexus.recognized),
        },
        warnings,
    }
}

/// 落点是不是落在游戏会扫描的目录里。
///
/// 按**路径组件**比，不用字符串前缀 —— 否则 `Maps/CampaignX/…` 会被当成
/// `Maps/Campaign` 的子目录。分隔符两种都收（包内路径理论上统一用正斜杠，
/// 但这条判据是给别人当契约用的，别在标点上栽跟头）。
pub fn recognized(target: &str) -> bool {
    let normalized = target.replace('\\', "/");
    let trimmed = normalized.trim_matches('/');
    SCANNED_ROOTS.iter().any(|root| {
        trimmed == *root
            || trimmed
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// 这条路径没被认出来的话，话说清楚；两套结果不同时也说一句。
///
/// `mirror` 载荷的落点是**包作者写死的**，两套约定都不改它 —— 那种情况下
/// "两套不同"是假象，所以这里只在真的不一样时才提。
fn why(
    recognized: bool,
    other_recognized: bool,
    target: &Option<String>,
    other: &Option<String>,
) -> String {
    if !recognized {
        return match target {
            Some(path) => {
                format!("{path} 不在游戏扫描的目录里（Maps/Campaign、Maps/CustomCampaigns、Mods）")
            }
            None => "这套约定下它不进游戏目录".to_string(),
        };
    }
    if recognized != other_recognized {
        return if other_recognized {
            "我们的约定认它，枢纽那套不认".to_string()
        } else {
            "枢纽那套认它，我们的约定不认".to_string()
        };
    }
    if target != other {
        return "两套约定的落点不同，但都在游戏扫描的目录里".to_string();
    }
    String::new()
}

fn kind_of(target: &PayloadTarget) -> &'static str {
    match target {
        PayloadTarget::Mirror { .. } => "mirror",
        PayloadTarget::Mod { .. } => "mod",
        PayloadTarget::Map { .. } => "map",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::package::{PackageInspection, Payload};
    use std::path::PathBuf;

    /// 一个最小可用的预检结果：只填这个模块会读的字段。
    fn inspection(campaign: CampaignType, payloads: Vec<Payload>) -> PackageInspection {
        use crate::campaign::CampaignFormat;
        use crate::campaign::metadata::PackageKind;

        PackageInspection {
            path: PathBuf::from("x.zip"),
            installable: true,
            needs_password: false,
            format: CampaignFormat::Ccm,
            name: Some("示例".to_string()),
            author: None,
            version: None,
            description: None,
            campaign_type: campaign,
            cover: None,
            tags: Vec::new(),
            kind: PackageKind::Campaign,
            id: None,
            requires: Vec::new(),
            priority: None,
            payloads,
            overrides: Vec::new(),
            main_map: None,
            doc: None,
            declared_mods: Vec::new(),
            modid: None,
            identification: None,
            suggested_slot: None,
            content_root: String::new(),
            suggested_dir_name: Some("示例".to_string()),
            map_count: 0,
            mod_count: 0,
            entry_count: 0,
            unpacked_bytes: 0,
            issues: Vec::new(),
        }
    }

    fn map(name: &str) -> Payload {
        Payload {
            source: name.to_string(),
            target: PayloadTarget::Map {
                name: name.to_string(),
            },
            expanded: false,
            is_mod: false,
        }
    }

    fn module(name: &str) -> Payload {
        Payload {
            source: name.to_string(),
            target: PayloadTarget::Mod {
                name: name.to_string(),
            },
            expanded: false,
            is_mod: true,
        }
    }

    fn mirror(path: &str) -> Payload {
        Payload {
            source: path.to_string(),
            target: PayloadTarget::Mirror {
                path: path.to_string(),
            },
            expanded: false,
            is_mod: false,
        }
    }

    /// 官方改版：两套落点不同、但都认得出来；模组两边都进 Mods/。
    #[test]
    fn official_revision_lands_in_different_places_but_both_recognized() {
        let report = analyze(
            &inspection(
                CampaignType::Lotv,
                vec![map("paiur01.SC2Map"), module("X.SC2Mod")],
            ),
            "示例",
        );

        assert_eq!(report.slot.as_deref(), Some("lotv"));
        assert_eq!(report.summary.ours.missed, 0);
        assert_eq!(report.summary.nexus.missed, 0);

        let map = &report.files[0];
        assert_eq!(
            map.ours.target.as_deref(),
            Some("Maps/Campaign/void/paiur01.SC2Map")
        );
        assert_eq!(
            map.nexus.target.as_deref(),
            Some("Maps/CustomCampaigns/示例/paiur01.SC2Map")
        );
        assert!(map.ours.recognized && map.nexus.recognized);
        assert_eq!(map.ours.why, "两套约定的落点不同，但都在游戏扫描的目录里");

        let module = &report.files[1];
        assert_eq!(module.ours.target.as_deref(), Some("Mods/X.SC2Mod"));
        assert_eq!(module.nexus.target.as_deref(), Some("Mods/X.SC2Mod"));
        assert_eq!(module.ours.why, "", "落点一样就不啰嗦");

        assert!(
            report
                .summary
                .warnings
                .iter()
                .any(|warning| warning.contains("枢纽那套不看声明")),
            "官方改版被枢纽当自制战役，必须提一句：{:?}",
            report.summary.warnings
        );
    }

    /// 自制战役：两套本来就一样，不该报"不同"。
    #[test]
    fn custom_campaign_is_identical_under_both() {
        let report = analyze(
            &inspection(CampaignType::Custom, vec![map("a.SC2Map")]),
            "自制",
        );

        let file = &report.files[0];
        assert_eq!(file.ours.target, file.nexus.target);
        assert_eq!(file.ours.why, "");
        assert!(
            report.summary.warnings.is_empty(),
            "{:?}",
            report.summary.warnings
        );
    }

    /// 镜像载荷的落点是包作者写死的：两套都不改它，掉出扫描目录就两边都不认。
    #[test]
    fn stray_mirror_path_is_missed_by_both() {
        let report = analyze(
            &inspection(
                CampaignType::Wol,
                vec![mirror("Maps/Starcraft Mass Recall/Terran01.SC2Map")],
            ),
            "示例",
        );

        let file = &report.files[0];
        assert_eq!(file.ours.target, file.nexus.target, "镜像载荷两套落点相同");
        assert!(!file.ours.recognized && !file.nexus.recognized);
        assert!(
            file.ours.why.contains("不在游戏扫描的目录里"),
            "{}",
            file.ours.why
        );
        assert_eq!(report.summary.ours.missed, 1);
        assert_eq!(report.summary.nexus.missed, 1);
    }

    /// 认不出归属时不许崩：slot 为 None，照样给两套落点并提醒。
    #[test]
    fn unknown_campaign_does_not_panic() {
        let report = analyze(
            &inspection(CampaignType::Other(String::new()), vec![map("abc.SC2Map")]),
            "mystery",
        );

        assert_eq!(report.slot, None);
        assert_eq!(report.files.len(), 1);
        assert!(
            report.summary.warnings[0].contains("认不出"),
            "{:?}",
            report.summary.warnings
        );
    }

    /// 判据本身：只看扫描目录，别被相似前缀骗了。
    #[test]
    fn recognition_rule_is_component_wise() {
        assert!(recognized("Maps/Campaign/a.SC2Map"));
        assert!(recognized("Maps/Campaign"));
        assert!(recognized("Maps/Campaign/swarm/x.SC2Map"));
        assert!(recognized("Maps/CustomCampaigns/名字/x.SC2Map"));
        assert!(recognized("Mods/X.SC2Mod"));
        assert!(recognized("Mods/Alenger/1钢铁.SC2Mod"));
        // 反斜杠也认
        assert!(recognized("Mods\\X.SC2Mod"));

        assert!(!recognized("Maps/Starcraft Mass Recall/x.SC2Map"));
        assert!(!recognized("Maps/maps/x.SC2Map"));
        assert!(!recognized("Maps/CampaignX/x.SC2Map"), "相似前缀不算");
        assert!(!recognized("SC2Data/base.SC2Data/x"));
        assert!(!recognized("Interfaces/x.SC2Interface"));
        assert!(!recognized(""));
    }

    /// "为什么"要能区分三种情况：不认、两边相反、只是落点不同。
    #[test]
    fn why_explains_each_case() {
        let stray = analyze(
            &inspection(CampaignType::Wol, vec![mirror("Maps/maps/x.SC2Map")]),
            "示例",
        );
        assert!(stray.files[0].ours.why.contains("不在游戏扫描的目录里"));

        let same = analyze(
            &inspection(CampaignType::Custom, vec![map("a.SC2Map")]),
            "自制",
        );
        assert_eq!(same.files[0].ours.why, "");

        let different = analyze(
            &inspection(CampaignType::Wol, vec![map("a.SC2Map")]),
            "示例",
        );
        assert!(different.files[0].ours.why.contains("落点不同"));
    }
}
