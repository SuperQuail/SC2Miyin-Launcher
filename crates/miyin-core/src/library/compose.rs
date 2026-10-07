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

/// 把一个载荷的落点展开成游戏目录内的相对路径。
///
/// `sub` 是战役的子目录（自由之翼没有、虫群之心是 `swarm`、进化是 `swarm/evolution`）。
pub fn payload_target_path(target: &PayloadTarget, sub: Option<&str>) -> String {
    match target {
        PayloadTarget::Mirror { path } => path.replace('\\', "/"),
        PayloadTarget::Mod { name } => format!("Mods/{name}"),
        PayloadTarget::Map { name } => {
            // 包内已经按游戏结构摆了（voidprologue/…、swarm/evolution/…）-> 直接用
            if known_campaign_prefix(name).is_some() {
                return format!("Maps/Campaign/{name}");
            }
            match sub {
                Some(sub) if !sub.is_empty() => format!("Maps/Campaign/{sub}/{name}"),
                _ => format!("Maps/Campaign/{name}"),
            }
        }
    }
}

/// 把一层内容叠进清单；同目标路径由后叠的（优先级高的）胜出。
fn stack_layer(
    root: &Path,
    payloads: &[Payload],
    sub: Option<&str>,
    layer: &Layer,
    placed: &mut BTreeMap<String, ComposedFile>,
    overridden: &mut Vec<ComposedFile>,
) {
    for payload in payloads {
        let target = payload_target_path(&payload.target, sub);
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

    // 第 0 层：战役本体
    let variant_dir = library.slot_dir(slot_slug).join(&variant.id);
    stack_layer(
        &variant_dir,
        &variant.payloads,
        sub,
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
            sub,
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
            payload_target_path(&map, Some("void")),
            "Maps/Campaign/void/paiur01.SC2Map"
        );
        assert_eq!(
            payload_target_path(&map, Some("swarm/evolution")),
            "Maps/Campaign/swarm/evolution/paiur01.SC2Map"
        );
        // 自由之翼没有子目录
        assert_eq!(
            payload_target_path(&map, None),
            "Maps/Campaign/paiur01.SC2Map"
        );
    }

    #[test]
    fn mod_and_mirror_targets_are_stable() {
        let mode = PayloadTarget::Mod {
            name: "X.SC2Mod".to_string(),
        };
        assert_eq!(payload_target_path(&mode, Some("void")), "Mods/X.SC2Mod");

        // 镜像路径不受子目录影响 —— 包作者已经写死了落点
        let mirror = PayloadTarget::Mirror {
            path: "Maps/Campaign/voidprologue/x.SC2Map".to_string(),
        };
        assert_eq!(
            payload_target_path(&mirror, Some("void")),
            "Maps/Campaign/voidprologue/x.SC2Map"
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
            None,
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
        stack_layer(root, &patch, None, &layer, &mut placed, &mut overridden);

        assert_eq!(placed.len(), 1, "同一目标路径只能留一项");
        let winner = placed.get("Maps/Campaign/a.SC2Map").expect("应当有这一项");
        assert_eq!(winner.layer, layer, "后叠的补丁应当胜出");
        assert!(winner.expanded);
        assert_eq!(overridden.len(), 1, "被盖掉的那项要留痕");
        assert_eq!(overridden[0].layer, Layer::Campaign);
    }
}
