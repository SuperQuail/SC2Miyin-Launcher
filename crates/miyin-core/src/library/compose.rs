//! 分层合成：**战役本体 + 若干补丁 → 一份"落到游戏目录的清单"**。
//!
//! 这是补丁机制的地基。补丁不复制战役内容，而是在**铺盘那一刻**按优先级叠加：
//!
//! ```text
//! 第 0 层   data/campaigns/<战役>/<版本>/     战役本体
//! 第 1 层   data/patches/<补丁id>/            priority 100
//! 第 2 层   data/patches/<补丁id>/            priority 200
//! ──────────────────────────────────────────────
//! 合成 = 每个目标路径取"最高优先级那一层"的文件
//! ```
//!
//! 好处正是需求里的三条：
//!
//! - **省空间**：补丁内容全程只存一份，不复制战役
//! - **可恢复**：合成不改动任何原始层，铺盘的文件全部记账，撤下即还原
//! - **开关快**：勾掉一个补丁 = 重算一次清单

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::campaign::metadata::CampaignType;
use crate::campaign::package::{Payload, PayloadTarget, known_campaign_prefix};
use crate::library::{Binding, Library, Patch, Variant};

/// 一层内容的出处。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Layer {
    /// 战役本体。
    Campaign,
    /// 某个补丁。
    Patch {
        id: String,
        name: String,
        priority: i64,
    },
}

impl Layer {
    /// 面向用户的名字。
    pub fn label(&self) -> String {
        match self {
            Self::Campaign => "战役本体".to_string(),
            Self::Patch { name, .. } => format!("补丁「{name}」"),
        }
    }
}

/// 合成清单里的一项。
#[derive(Debug, Clone, Serialize)]
pub struct ComposedFile {
    /// 游戏目录内的相对路径（一律正斜杠）。
    pub target: String,
    /// 来源文件或目录的绝对路径。
    pub source: PathBuf,
    /// 来源是不是一棵解开的目录树。
    pub expanded: bool,
    /// 来自哪一层。
    pub layer: Layer,
}

/// 一份合成清单。
#[derive(Debug, Clone, Serialize)]
pub struct Composition {
    /// 最终要铺进游戏目录的东西，按目标路径排好序。
    pub files: Vec<ComposedFile>,
    /// 被更高优先级层盖掉的项（用来向用户解释"谁盖了谁"）。
    pub overridden: Vec<ComposedFile>,
}

impl Composition {
    /// 有多少项是被补丁改写过的。
    pub fn patched_count(&self) -> usize {
        self.files
            .iter()
            .filter(|file| matches!(file.layer, Layer::Patch { .. }))
            .count()
    }

    /// 参与合成的补丁（按优先级从低到高）。
    pub fn patch_layers(&self) -> Vec<Layer> {
        let mut layers: Vec<Layer> = Vec::new();
        for file in &self.files {
            if let layer @ Layer::Patch { .. } = &file.layer
                && !layers.contains(layer)
            {
                layers.push(layer.clone());
            }
        }
        layers
    }
}

/// 载荷在游戏目录里的落点基址。
///
/// 两类战役的**安装方式根本不同**：
///
/// | 类型 | 地图 | 模组 |
/// | --- | --- | --- |
/// | 原版战役（含改版） | 进 `Maps/Campaign[/子目录]/` | 进 `Mods/` |
/// | 自制战役 | 进 `Maps/CustomCampaigns/<战役目录>/` | 进 `Mods/` |
///
/// **自制战役的地图要真的装到游戏目录里**（`Maps/CustomCampaigns`）——
/// 那是 CCM 和同类工具约定俗成的位置，玩家和别的工具都去那里找。
/// （早先这里判成「留在库里不进游戏目录」，结果自制战役根本装不上，
/// 用户看到的就是「导入成功了但游戏里没有」。）
///
/// 模组两类战役都要进 `Mods/`：**地图里写死了依赖路径**。
/// 实测 SCMR 的 `Terran01.SC2Map` 里声明的是 `Mods\SCMRmod.SC2Mod` ——
/// 这是相对游戏根目录的路径，游戏与编辑器只会去 `<游戏>/Mods/` 下找。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// 原版战役：`Maps/Campaign[/子目录]`。
    Campaign { sub: Option<String> },
    /// 自制战役：`Maps/CustomCampaigns/<战役目录>/`。
    Custom { folder: String },
}

impl Placement {
    /// 由槽位、版本目录名与版本声明的子目录推出来。
    ///
    /// `folder` 只在自制战役时用到 —— 它就是 `Maps/CustomCampaigns/` 下面的那层目录名。
    pub fn of(slot_slug: &str, folder: &str, target_sub: Option<&str>) -> Self {
        let is_custom = CampaignType::from_slug(slot_slug).is_some_and(|kind| kind.is_custom());
        if is_custom {
            Self::Custom {
                folder: folder.to_string(),
            }
        } else {
            Self::Campaign {
                sub: target_sub.map(str::to_string),
            }
        }
    }

    /// 这一层内容落在游戏目录里的基址。
    pub fn base(&self) -> String {
        match self {
            Self::Campaign { .. } => "Maps/Campaign".to_string(),
            Self::Custom { folder } => format!("Maps/CustomCampaigns/{folder}"),
        }
    }
}

/// 把一个载荷的落点展开成游戏目录内的相对路径。
///
/// 返回 `None` 表示**这个载荷不进游戏目录**（自制战役的地图就是这样，
/// 它留在库里等着被编辑器打开）。
pub fn payload_target_path(target: &PayloadTarget, placement: &Placement) -> Option<String> {
    match target {
        PayloadTarget::Mirror { path } => Some(path.replace('\\', "/")),
        // 模组两类战役都要进 Mods/ —— 地图依赖它
        PayloadTarget::Mod { name } => Some(format!("Mods/{name}")),
        PayloadTarget::Map { name } => {
            // 自制战役：整包进 Maps/CustomCampaigns/<战役目录>/
            if let Placement::Custom { folder } = placement {
                return Some(format!("Maps/CustomCampaigns/{folder}/{name}"));
            }

            // 包内已经按官方结构摆了（voidprologue/…、swarm/evolution/…）-> 直接用
            if known_campaign_prefix(name).is_some() {
                return Some(format!("Maps/Campaign/{name}"));
            }
            Some(match placement {
                Placement::Campaign { sub: Some(sub) } if !sub.is_empty() => {
                    format!("Maps/Campaign/{sub}/{name}")
                }
                _ => format!("Maps/Campaign/{name}"),
            })
        }
    }
}

/// 把一层内容叠进清单；同目标路径由后叠的（优先级高的）胜出。
fn stack_layer(
    root: &Path,
    payloads: &[Payload],
    placement: &Placement,
    layer: &Layer,
    placed: &mut BTreeMap<String, ComposedFile>,
    overridden: &mut Vec<ComposedFile>,
) {
    for payload in payloads {
        // 自制战役的地图不进游戏目录 —— 它们留在库里等着被编辑器打开
        let Some(target) = payload_target_path(&payload.target, placement) else {
            continue;
        };

        let source = root.join(payload.source.replace('/', std::path::MAIN_SEPARATOR_STR));
        let item = ComposedFile {
            target: target.clone(),
            source,
            expanded: payload.expanded,
            layer: layer.clone(),
        };

        if let Some(previous) = placed.insert(target, item) {
            overridden.push(previous);
        }
    }
}

/// 合成某个版本（含它身上启用的补丁）。
///
/// `sub` 由调用方给出（版本自己声明的 `target_sub`，退化到槽位）。
pub fn compose(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    sub: Option<&str>,
) -> Composition {
    let mut placed: BTreeMap<String, ComposedFile> = BTreeMap::new();
    let mut overridden: Vec<ComposedFile> = Vec::new();

    // 官方战役还是自制战役 —— 落点不一样
    // 自制战役要用版本目录名当 CustomCampaigns 下面那层
    let placement = Placement::of(slot_slug, &variant.id, sub);

    // 模组**只铺挂载了的那些** —— 官方战役的改版包一样可能带模组，
    // 而不同战役的模组之间会互相打架，所以一律让用户自己选挂哪几个
    // （见 Variant::mounted_mods 与 ModsView）。地图不受影响。
    let mounted = crate::library::effective_mounted_mods(variant);
    let payloads: Vec<Payload> = variant
        .payloads
        .iter()
        .filter(|payload| match crate::library::mod_identity(payload) {
            Some(found) => mounted.contains(&found.key),
            None => true,
        })
        .cloned()
        .collect();

    // 第 0 层：战役本体
    let variant_dir = library.slot_dir(slot_slug).join(&variant.id);
    stack_layer(
        &variant_dir,
        &payloads,
        &placement,
        &Layer::Campaign,
        &mut placed,
        &mut overridden,
    );

    // 第 1..N 层：这个战役上启用的补丁，按优先级从低到高
    let index = library.index();
    let mut bindings: Vec<Binding> = index
        .bindings
        .get(slot_slug)
        .map(|list| {
            list.iter()
                .filter(|binding| binding.enabled)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    // 优先级相同时按补丁 id 排，保证结果稳定可复现
    bindings.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.patch_id.cmp(&right.patch_id))
    });

    for binding in bindings {
        let Some(patch) = index.patches.get(&binding.patch_id) else {
            continue;
        };
        let layer = Layer::Patch {
            id: patch.id.clone(),
            name: patch.name.clone(),
            priority: binding.priority,
        };
        let patch_dir = library.patch_dir(&patch.id);
        stack_layer(
            &patch_dir,
            &patch.payloads,
            &placement,
            &layer,
            &mut placed,
            &mut overridden,
        );
    }

    Composition {
        files: placed.into_values().collect(),
        overridden,
    }
}

/// 某个战役当前挂着的补丁（含未启用的），按优先级排好。
pub fn bindings_of(library: &Library, slot_slug: &str) -> Vec<(Binding, Patch)> {
    let index = library.index();
    let mut list: Vec<(Binding, Patch)> = index
        .bindings
        .get(slot_slug)
        .map(|bindings| {
            bindings
                .iter()
                .filter_map(|binding| {
                    index
                        .patches
                        .get(&binding.patch_id)
                        .map(|patch| (binding.clone(), patch.clone()))
                })
                .collect()
        })
        .unwrap_or_default();

    list.sort_by(|left, right| {
        left.0
            .priority
            .cmp(&right.0.priority)
            .then_with(|| left.0.patch_id.cmp(&right.0.patch_id))
    });
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(source: &str, target: PayloadTarget, expanded: bool) -> Payload {
        Payload {
            source: source.to_string(),
            target,
            expanded,
            is_mod: false,
        }
    }

    #[test]
    fn map_targets_follow_the_sub_directory() {
        let map = PayloadTarget::Map {
            name: "paiur01.SC2Map".to_string(),
        };
        assert_eq!(
            payload_target_path(
                &map,
                &Placement::Campaign {
                    sub: Some("void".into())
                }
            )
            .as_deref(),
            Some("Maps/Campaign/void/paiur01.SC2Map")
        );
        assert_eq!(
            payload_target_path(
                &map,
                &Placement::Campaign {
                    sub: Some("swarm/evolution".into())
                }
            )
            .as_deref(),
            Some("Maps/Campaign/swarm/evolution/paiur01.SC2Map")
        );
        // 自由之翼没有子目录
        assert_eq!(
            payload_target_path(&map, &Placement::Campaign { sub: None }).as_deref(),
            Some("Maps/Campaign/paiur01.SC2Map")
        );
    }

    #[test]
    fn mod_and_mirror_targets_are_stable() {
        let mode = PayloadTarget::Mod {
            name: "X.SC2Mod".to_string(),
        };
        assert_eq!(
            payload_target_path(
                &mode,
                &Placement::Campaign {
                    sub: Some("void".into())
                }
            )
            .as_deref(),
            Some("Mods/X.SC2Mod")
        );

        // 镜像路径不受子目录影响 —— 包作者已经写死了落点
        let mirror = PayloadTarget::Mirror {
            path: "Maps/Campaign/voidprologue/x.SC2Map".to_string(),
        };
        assert_eq!(
            payload_target_path(
                &mirror,
                &Placement::Campaign {
                    sub: Some("void".into())
                }
            )
            .as_deref(),
            Some("Maps/Campaign/voidprologue/x.SC2Map")
        );
    }

    #[test]
    fn higher_priority_layer_wins() {
        let mut placed = BTreeMap::new();
        let mut overridden = Vec::new();
        let root = Path::new("root");

        let campaign = vec![payload(
            "a.SC2Map",
            PayloadTarget::Map {
                name: "a.SC2Map".to_string(),
            },
            false,
        )];
        stack_layer(
            root,
            &campaign,
            &Placement::Campaign { sub: None },
            &Layer::Campaign,
            &mut placed,
            &mut overridden,
        );

        let patch = vec![payload(
            "a.SC2Map",
            PayloadTarget::Map {
                name: "a.SC2Map".to_string(),
            },
            true,
        )];
        let layer = Layer::Patch {
            id: "p".to_string(),
            name: "补丁".to_string(),
            priority: 100,
        };
        stack_layer(
            root,
            &patch,
            &Placement::Campaign { sub: None },
            &layer,
            &mut placed,
            &mut overridden,
        );

        assert_eq!(placed.len(), 1, "同一目标路径只能留一项");
        let winner = placed.get("Maps/Campaign/a.SC2Map").expect("应当有这一项");
        assert_eq!(winner.layer, layer, "后叠的补丁应当胜出");
        assert!(winner.expanded);
        assert_eq!(overridden.len(), 1, "被盖掉的那项要留痕");
        assert_eq!(overridden[0].layer, Layer::Campaign);
    }

    #[test]
    fn custom_campaigns_install_into_custom_campaigns() {
        let map = PayloadTarget::Map {
            name: "1. Rebel Yell/Terran01.SC2Map".to_string(),
        };

        // 自制战役：整包进 `Maps/CustomCampaigns/<战役目录>/`。
        //
        // **这里改过一次**：早先判成「地图留在库里不进游戏目录」，理由是
        // 「SC2 不支持自制战役」。结论是错的 —— CustomCampaigns 正是 CCM
        // 和同类工具约定俗成的位置，玩家和别的工具都去那里找。
        // 照旧写法，用户看到的是「导入成功了但游戏里没有」。
        let custom = Placement::of("custom", "qunyou", None);
        assert_eq!(
            custom,
            Placement::Custom {
                folder: "qunyou".to_string()
            }
        );
        assert_eq!(
            custom.base(),
            "Maps/CustomCampaigns/qunyou",
            "自制战役进 CustomCampaigns —— 那是 CCM 认的位置"
        );
        assert_eq!(
            payload_target_path(&map, &custom).as_deref(),
            Some("Maps/CustomCampaigns/qunyou/1. Rebel Yell/Terran01.SC2Map"),
            "地图要真的装进去"
        );

        // 但模组**必须**进 Mods/ —— 实测 SCMR 的地图里写死了
        // Mods\SCMRmod.SC2Mod 这个依赖路径，游戏与编辑器只去那里找
        let mod_target = PayloadTarget::Mod {
            name: "SCMRmod.SC2Mod".to_string(),
        };
        assert_eq!(
            payload_target_path(&mod_target, &custom).as_deref(),
            Some("Mods/SCMRmod.SC2Mod")
        );

        // 官方战役（含其改版）还是老地方
        let official = Placement::of("lotv", "ignored", Some("void"));
        assert_eq!(official.base(), "Maps/Campaign");
        assert_eq!(
            payload_target_path(&map, &official).as_deref(),
            Some("Maps/Campaign/void/1. Rebel Yell/Terran01.SC2Map")
        );
        assert_eq!(
            payload_target_path(&mod_target, &official).as_deref(),
            Some("Mods/SCMRmod.SC2Mod")
        );
    }
}
