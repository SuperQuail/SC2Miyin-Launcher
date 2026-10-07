//! 战役元数据解析。
//!
//! 支持两种格式：
//!
//! | 格式 | 元数据文件 | 位置 | 语法 |
//! | --- | --- | --- | --- |
//! | CCM | `metadata.txt` | 包内**任意层级** | `键=值` 文本 |
//! | 弥音标准 | `metadata.json` | **包根** | JSON |
//!
//! # 相比参考实现修正了什么
//!
//! 1. **键名大小写不敏感**。上游 CCM 用 `ToLower()` 比较，而枢纽用字面量比较，
//!    于是 `Title=` 的包解析出空名字 —— 进而触发目录穿越删除（见 `sanitize` 模块注释）。
//! 2. **没有 `=` 的行被跳过**，而不是取 `undefined` 再抛 `TypeError`。
//! 3. **值按第一个 `=` 切分**，值里的 `=` 不会被截断。
//! 4. **编码探测**：优先 UTF-8，失败退回 GBK，中文包的标题不再乱码。
//! 5. **`campaign` 字段容错匹配**：`wings` / `liberty` / `swarm` 等写法都能识别。

use serde::{Deserialize, Serialize};

/// 把元数据字节解码为文本。
///
/// 先按 UTF-8 解码；失败则按 GBK 解码（国服中文包的常见编码）。
pub fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.trim_start_matches('\u{feff}').to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
            decoded.trim_start_matches('\u{feff}').to_string()
        }
    }
}

/// 按常见分隔符切分一个列表值（逗号 / 顿号 / 分号 / 空格）。
fn split_list(value: &str) -> Vec<String> {
    value
        .split([',', '，', '、', ';', '；', ' '])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// CCM 包元数据（来自 `metadata.txt`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CcmMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub campaign: Option<String>,
    pub version: Option<String>,
    /// 包内自带的封面图（相对内容根的路径）。
    ///
    /// 包作者可以用 cover / image / icon / banner 指定；没写就由导入逻辑按文件名特征查找。
    pub cover: Option<String>,
    /// 标签（逗号 / 顿号 / 空格分隔）。这是弥音扩展键，CCM 本身没有。
    pub tags: Vec<String>,
    /// **注册 ID**：补丁靠它引用战役，更新靠它认出同一个战役。
    pub id: Option<String>,
    /// 包类型：`campaign`（默认）或 `patch`。
    pub kind: Option<String>,
    /// 补丁依赖的战役（注册 ID 或战役名）。为空表示只能手动指定。
    pub requires: Vec<String>,
    /// 补丁默认优先级；大的后覆盖。
    pub priority: Option<i64>,
}

impl CcmMetadata {
    /// 解析 `metadata.txt` 文本。
    ///
    /// 该函数**不会失败**：畸形行一律跳过，最坏情况返回全空的元数据。
    pub fn parse(text: &str) -> Self {
        // 直接调用本函数时文本可能带 BOM。U+FEFF 并非空白字符，
        // trim() 不会去掉它，必须显式剥离，否则第一行的键名永远匹配不上。
        let text = text.trim_start_matches('\u{feff}');
        let mut meta = Self::default();

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // 没有等号的行直接跳过（参考实现在这里会抛 TypeError）
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            match key.trim().to_ascii_lowercase().as_str() {
                "title" | "name" => meta.title = Some(value.to_string()),
                "desc" | "description" => meta.description = Some(value.to_string()),
                "author" => meta.author = Some(value.to_string()),
                "campaign" => meta.campaign = Some(value.to_string()),
                "version" => meta.version = Some(value.to_string()),
                "cover" | "image" | "icon" | "banner" => meta.cover = Some(value.to_string()),
                "id" => meta.id = Some(value.to_string()),
                "type" | "kind" => meta.kind = Some(value.to_string()),
                "requires" | "require" | "dependencies" => {
                    meta.requires = split_list(value);
                }
                "priority" => meta.priority = value.parse().ok(),
                "tags" | "tag" => meta.tags = split_list(value),
                _ => {}
            }
        }

        meta
    }

    /// 是否一个字段都没解析到。
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.author.is_none()
            && self.campaign.is_none()
            && self.version.is_none()
            && self.cover.is_none()
            && self.tags.is_empty()
            && self.id.is_none()
            && self.kind.is_none()
            && self.requires.is_empty()
            && self.priority.is_none()
    }
}

/// 弥音/枢纽标准包元数据（来自包根的 `metadata.json`）。
///
/// 所有字段都是可选的：标准里标了「必填」的字段在现实中经常缺失，
/// 解析层不该因为缺字段就整体失败（参考实现把可选字段当必填，导致激活崩溃）。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StandardMetadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// 包类型，标准包中 Campaign 才是战役。
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub campaign: Option<String>,
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub maps_directory: Option<String>,
    #[serde(default)]
    pub mods_directory: Option<String>,
    /// 包内自带的封面图（相对包根的路径）。
    #[serde(default)]
    pub cover: Option<String>,
    /// 注册 ID（顶层写法；`miyin.id` 优先）。
    #[serde(default)]
    pub id: Option<String>,
    /// 弥音专属扩展（命名空间字段，其它工具会直接忽略）。
    #[serde(default)]
    pub miyin: Option<MiyinExtensions>,
}

impl StandardMetadata {
    /// 取封面路径：命名空间写法 `miyin.cover` 优先，其次顶层的 `cover`。
    pub fn cover_path(&self) -> Option<&str> {
        self.miyin
            .as_ref()
            .and_then(|extensions| extensions.cover.as_deref())
            .or(self.cover.as_deref())
    }

    /// 注册 ID：命名空间写法优先，其次顶层。
    pub fn id(&self) -> Option<&str> {
        self.miyin
            .as_ref()
            .and_then(|extensions| extensions.id.as_deref())
            .or(self.id.as_deref())
            .map(str::trim)
            .filter(|id| !id.is_empty())
    }

    /// 包类型：`miyin.kind` 优先，其次顶层的 `type`。
    pub fn package_kind(&self) -> PackageKind {
        if let Some(kind) = self
            .miyin
            .as_ref()
            .and_then(|extensions| extensions.kind.as_deref())
        {
            return PackageKind::parse(kind);
        }
        match self.kind.as_deref() {
            Some(kind) => PackageKind::parse(kind),
            None => PackageKind::Campaign,
        }
    }

    /// 补丁依赖的战役。
    pub fn patch_requires(&self) -> Vec<String> {
        self.miyin
            .as_ref()
            .and_then(|extensions| extensions.patch.as_ref())
            .map(|patch| patch.requires.clone())
            .unwrap_or_default()
    }

    /// 补丁默认优先级。
    pub fn patch_priority(&self) -> Option<i64> {
        self.miyin
            .as_ref()
            .and_then(|extensions| extensions.patch.as_ref())
            .and_then(|patch| patch.priority)
    }

    /// 扩展字段里的标签。
    pub fn tags(&self) -> Vec<String> {
        self.miyin
            .as_ref()
            .map(|extensions| extensions.tags.clone())
            .unwrap_or_default()
    }
}

/// 弥音专属扩展字段（`metadata.json` 里的 `miyin` 对象）。
///
/// 放在独立命名空间里，是为了让「星际枢纽」等工具直接忽略它们，
/// 同时给我们自己留出**不与枢纽未来字段撞名**的扩展空间。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MiyinExtensions {
    /// 扩展格式版本。启动器只接受不高于自己支持版本的包。
    #[serde(default)]
    pub format: Option<u32>,
    /// 封面图（相对包根的路径）。
    #[serde(default)]
    pub cover: Option<String>,
    /// 标签，显示在版本卡片上。
    #[serde(default)]
    pub tags: Vec<String>,
    /// 注册 ID（优先级高于顶层 `id`）。
    #[serde(default)]
    pub id: Option<String>,
    /// 包类型：`campaign`（默认）或 `patch`。
    #[serde(default)]
    pub kind: Option<String>,
    /// 补丁专属字段。
    #[serde(default)]
    pub patch: Option<PatchExtensions>,
}

/// 补丁专属扩展。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PatchExtensions {
    /// 依赖的战役（注册 ID 或战役名）；为空表示只能手动指定。
    #[serde(default)]
    pub requires: Vec<String>,
    /// 默认优先级；数值大的后覆盖。
    #[serde(default)]
    pub priority: Option<i64>,
}

impl StandardMetadata {
    /// 解析 `metadata.json` 文本。
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// 包的类型：战役本体，还是覆盖在战役之上的补丁。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PackageKind {
    /// 战役包。
    #[default]
    Campaign,
    /// 补丁包（覆盖层）。
    Patch,
}

impl PackageKind {
    /// 容错解析；认不出来一律当战役。
    pub fn parse(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("patch") {
            Self::Patch
        } else {
            Self::Campaign
        }
    }
}

/// 战役归属的资料片。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CampaignType {
    Wol,
    Hots,
    HotsEvolution,
    Lotv,
    LotvPrologue,
    Nova,
    /// 未识别（或缺失）的取值，原样保留便于排查。
    Other(String),
}

impl CampaignType {
    /// 容错解析 `campaign` 字段。
    ///
    /// 参考实现要求严格等于 `WOL` / `HOTS` / `LOTV` / `NCO` 之一，
    /// 于是 `campaign=wings` 的包「装上了却扫不到」。
    pub fn parse(raw: &str) -> Self {
        let value = raw.trim().to_ascii_lowercase();
        if value.is_empty() {
            return Self::Other(String::new());
        }

        // 顺序要紧：先匹配更具体的取值
        if value.contains("evolut") || value.contains("hotsevo") {
            Self::HotsEvolution
        } else if value.contains("prolog") {
            Self::LotvPrologue
        } else if value.contains("wol") || value.contains("wings") || value.contains("liberty") {
            Self::Wol
        } else if value.contains("hots") || value.contains("swarm") || value.contains("kerrigan") {
            Self::Hots
        } else if value.contains("lotv") || value.contains("void") || value.contains("legacy") {
            Self::Lotv
        } else if value.contains("nco") || value.contains("nova") {
            Self::Nova
        } else {
            Self::Other(raw.trim().to_string())
        }
    }

    /// 官方资料片的**固定展示顺序**：自由之翼 → 虫群之心 → 进化 → 虚空之遗 → 序章 → 诺娃。
    ///
    /// 界面必须按这个顺序排，而不是按名字排序 —— 否则「虚空之遗」会排到「诺娃」后面。
    pub fn order(&self) -> u8 {
        match self {
            Self::Wol => 0,
            Self::Hots => 1,
            Self::HotsEvolution => 2,
            Self::Lotv => 3,
            Self::LotvPrologue => 4,
            Self::Nova => 5,
            Self::Other(_) => u8::MAX,
        }
    }

    /// 稳定标识：用作战役库的槽位键与数据目录名。
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Wol => "wol",
            Self::Hots => "hots",
            Self::HotsEvolution => "hotsevolution",
            Self::Lotv => "lotv",
            Self::LotvPrologue => "lotvprologue",
            Self::Nova => "nova",
            Self::Other(_) => "other",
        }
    }

    /// 由稳定标识还原（只认官方槽位）。
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "wol" => Some(Self::Wol),
            "hots" => Some(Self::Hots),
            "hotsevolution" => Some(Self::HotsEvolution),
            "lotv" => Some(Self::Lotv),
            "lotvprologue" => Some(Self::LotvPrologue),
            "nova" => Some(Self::Nova),
            _ => None,
        }
    }

    /// 全部官方槽位，按发布顺序。
    /// 所属的**主战役**：进化归虫群之心、序章归虚空之遗。
    ///
    /// 主菜单只列四大战役，归并关系放在这里，而不是让界面去特判。
    pub fn parent(&self) -> Self {
        match self {
            Self::HotsEvolution => Self::Hots,
            Self::LotvPrologue => Self::Lotv,
            other => other.clone(),
        }
    }

    /// 该资料片**应当归入的主战役槽位**；认不出来时返回 `None`。
    ///
    /// 这是「自动判断是哪个战役」的唯一规则来源：
    /// 进化包 -> `hots`、序章包 -> `lotv`，认不出来就交给界面问用户。
    pub fn main_slot(&self) -> Option<&'static str> {
        let main = self.parent();
        main.is_main().then(|| main.slug())
    }

    /// 是否为主菜单上的四大战役之一。
    pub fn is_main(&self) -> bool {
        matches!(self, Self::Wol | Self::Hots | Self::Lotv | Self::Nova)
    }

    /// 主菜单上的四大战役。
    pub const MAIN: [Self; 4] = [Self::Wol, Self::Hots, Self::Lotv, Self::Nova];

    /// 全部官方槽位，按发布顺序。
    pub const ALL: [Self; 6] = [
        Self::Wol,
        Self::Hots,
        Self::HotsEvolution,
        Self::Lotv,
        Self::LotvPrologue,
        Self::Nova,
    ];

    /// 启用时地图应复制到的 `Maps/Campaign` 子目录。
    ///
    /// 返回 `None` 表示直接放在 `Maps/Campaign` 根下（自由之翼）。
    pub fn sub_directory(&self) -> Option<&'static str> {
        match self {
            Self::Wol => None,
            Self::Hots => Some("swarm"),
            Self::HotsEvolution => Some("swarm/evolution"),
            Self::Lotv => Some("void"),
            Self::LotvPrologue => Some("voidprologue"),
            Self::Nova => Some("nova"),
            Self::Other(_) => None,
        }
    }

    /// 是否可以启用（需要能确定归属资料片）。
    pub fn is_actionable(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// 面向用户的资料片名称。
    pub fn display_name(&self) -> String {
        match self {
            Self::Wol => "自由之翼".to_string(),
            Self::Hots => "虫群之心".to_string(),
            Self::HotsEvolution => "虫群之心 · 进化".to_string(),
            Self::Lotv => "虚空之遗".to_string(),
            Self::LotvPrologue => "虚空之遗 · 序章".to_string(),
            Self::Nova => "诺娃隐秘行动".to_string(),
            Self::Other(raw) if raw.is_empty() => "未标注资料片".to_string(),
            Self::Other(raw) => format!("未知资料片（{raw}）"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ccm_keys_are_case_insensitive() {
        // 参考实现在这里会得到空名字，进而触发目录穿越
        let meta = CcmMetadata::parse("Title=Reborn\nAUTHOR=SomeCreator\nCampaign=WOL\n");
        assert_eq!(meta.title.as_deref(), Some("Reborn"));
        assert_eq!(meta.author.as_deref(), Some("SomeCreator"));
        assert_eq!(meta.campaign.as_deref(), Some("WOL"));
    }

    #[test]
    fn ccm_skips_lines_without_equals_sign() {
        // 参考实现取 lineData[1] 得到 undefined，随后 .replace 抛 TypeError
        let meta = CcmMetadata::parse("title\njust some noise\nversion=1.0\n");
        assert_eq!(meta.title, None);
        assert_eq!(meta.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn ccm_keeps_equals_sign_inside_value() {
        let meta = CcmMetadata::parse("desc=Score = 10, Lives = 3\n");
        assert_eq!(meta.description.as_deref(), Some("Score = 10, Lives = 3"));
    }

    #[test]
    fn ccm_handles_crlf_and_bom() {
        let meta = CcmMetadata::parse("\u{feff}title=重制版自由之翼\r\ndesc=含 32 张关卡\r\n");
        assert_eq!(meta.title.as_deref(), Some("重制版自由之翼"));
        assert_eq!(meta.description.as_deref(), Some("含 32 张关卡"));
    }

    #[test]
    fn decode_text_falls_back_to_gbk() {
        let (bytes, _, _) = encoding_rs::GBK.encode("title=星际争霸：重制战役");
        let decoded = decode_text(&bytes);
        assert_eq!(decoded, "title=星际争霸：重制战役");

        let meta = CcmMetadata::parse(&decoded);
        assert_eq!(meta.title.as_deref(), Some("星际争霸：重制战役"));
    }

    #[test]
    fn campaign_type_tolerates_real_world_spellings() {
        assert_eq!(CampaignType::parse("WOL"), CampaignType::Wol);
        assert_eq!(CampaignType::parse("wings"), CampaignType::Wol);
        assert_eq!(CampaignType::parse("swarm"), CampaignType::Hots);
        assert_eq!(
            CampaignType::parse("swarm/evolution"),
            CampaignType::HotsEvolution
        );
        assert_eq!(
            CampaignType::parse("lotvprologue"),
            CampaignType::LotvPrologue
        );
        assert_eq!(CampaignType::parse("nova"), CampaignType::Nova);
        assert!(matches!(
            CampaignType::parse("something-else"),
            CampaignType::Other(_)
        ));
        assert!(matches!(CampaignType::parse(""), CampaignType::Other(_)));
    }

    #[test]
    fn campaign_order_puts_lotv_before_nova() {
        // 这正是用户提的问题：虚空之遗必须排在诺娃前面
        assert!(CampaignType::Lotv.order() < CampaignType::Nova.order());
        assert!(CampaignType::LotvPrologue.order() < CampaignType::Nova.order());
        assert!(CampaignType::Hots.order() < CampaignType::Lotv.order());

        let slugs: Vec<&str> = CampaignType::ALL.iter().map(CampaignType::slug).collect();
        assert_eq!(
            slugs,
            vec![
                "wol",
                "hots",
                "hotsevolution",
                "lotv",
                "lotvprologue",
                "nova"
            ]
        );
    }

    #[test]
    fn evolution_and_prologue_belong_to_their_parent_campaigns() {
        assert_eq!(CampaignType::HotsEvolution.parent(), CampaignType::Hots);
        assert_eq!(CampaignType::LotvPrologue.parent(), CampaignType::Lotv);
        assert_eq!(CampaignType::Wol.parent(), CampaignType::Wol);

        // 主菜单只有四大战役
        assert_eq!(CampaignType::MAIN.len(), 4);
        assert!(CampaignType::MAIN.iter().all(CampaignType::is_main));
        assert!(!CampaignType::HotsEvolution.is_main());
    }

    #[test]
    fn slug_round_trips() {
        for slot in CampaignType::ALL {
            assert_eq!(CampaignType::from_slug(slot.slug()), Some(slot));
        }
        assert_eq!(CampaignType::from_slug("nope"), None);
        assert_eq!(CampaignType::Other("x".into()).slug(), "other");
    }

    #[test]
    fn campaign_type_maps_to_expected_sub_directory() {
        assert_eq!(CampaignType::Wol.sub_directory(), None);
        assert_eq!(CampaignType::Hots.sub_directory(), Some("swarm"));
        assert_eq!(CampaignType::Lotv.sub_directory(), Some("void"));
        assert_eq!(CampaignType::Nova.sub_directory(), Some("nova"));
        assert!(!CampaignType::Other("x".into()).is_actionable());
    }

    #[test]
    fn standard_metadata_reads_miyin_extensions() {
        let text = r#"{
            "name": "重生",
            "author": "Creator",
            "campaign": "HOTSEVO",
            "cover": "top.png",
            "miyin": {
                "format": 1,
                "cover": "namespace.png",
                "tags": ["重制", "全语音"]
            }
        }"#;

        let meta = StandardMetadata::parse(text).expect("应解析成功");
        assert_eq!(meta.name.as_deref(), Some("重生"));
        // 命名空间写法优先于顶层字段
        assert_eq!(meta.cover_path(), Some("namespace.png"));
        assert_eq!(meta.tags(), vec!["重制".to_string(), "全语音".to_string()]);
        assert_eq!(meta.miyin.as_ref().and_then(|e| e.format), Some(1));
    }

    #[test]
    fn standard_metadata_falls_back_to_top_level_cover() {
        let meta = StandardMetadata::parse(r#"{"name":"A","cover":"cover.png"}"#).expect("解析");
        assert_eq!(meta.cover_path(), Some("cover.png"));
        assert!(meta.tags().is_empty());
        assert_eq!(meta.miyin.as_ref().and_then(|e| e.format), None);
    }

    #[test]
    fn ccm_tags_accept_common_separators() {
        let meta = CcmMetadata::parse("title=A\ntags=重制, 全语音、中文\n");
        assert_eq!(meta.tags, vec!["重制", "全语音", "中文"]);
    }

    #[test]
    fn standard_metadata_tolerates_missing_fields() {
        let meta = StandardMetadata::parse(r#"{"name":"A","author":"B"}"#).expect("应解析成功");
        assert_eq!(meta.name.as_deref(), Some("A"));
        assert_eq!(meta.version, None);
        assert_eq!(meta.maps_directory, None);
    }
}
