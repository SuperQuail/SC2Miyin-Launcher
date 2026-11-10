//! 包预检与解压共用的数据结构。
//!
//! 这里的类型都是 `inspect` / `extract_to` 的**输入输出形状**，不含任何判断逻辑
//! （唯一例外是 `unusable`：构造「连打开都失败」的那份结果）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::campaign::identify::Identification;
use crate::campaign::metadata::{CampaignType, PackageKind};
use crate::campaign::{CampaignFormat, HealthIssue};

/// 允许解压的最大总字节数（防 zip bomb）。
pub const MAX_UNPACKED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// 允许的最大条目数。
pub const MAX_ENTRIES: usize = 100_000;

/// 当前启动器支持的**弥音扩展格式**版本（见 `docs/package-format.md` §5）。
///
/// 包内 `miyin.format` 高于这个值时会被明确拒绝，而不是猜着解析。
///
/// 三档分别是（和文档里的对应关系要一致）：
///
/// - `1` 基础字段：name / author / version / campaign / id / cover / tags
/// - `2` 补丁字段：kind / priority / requires（这些**必须理解**才能正确安装）
/// - `3` 覆盖规则：overrides
pub const MIYIN_FORMAT_VERSION: u32 = 3;

/// 包内的一个**载荷**：一张地图或一个模组。
///
/// 关键点：`.SC2Map` / `.SC2Mod` 在真实包里**既可能是单文件 MPQ，也可能是一棵解开的目录树**
/// （`xxx.SC2Map/Base.SC2Data/...`），所以这里用 `expanded` 区分，落盘时保持原形态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// 相对内容根的路径（文件或目录）。**文件访问用这条**，与落点无关。
    pub source: String,
    /// 应该落到游戏目录的哪里。
    pub target: PayloadTarget,
    /// 是否是解开的目录树（false 表示单文件 MPQ）。
    pub expanded: bool,
    /// 载荷性质：`true` 是模组（`.SC2Mod`），`false` 是地图（`.SC2Map`）。
    ///
    /// 单独记一份，是因为落点类型不代表性质 —— 旧记录里的模组落点可能是
    /// `GamePath`（包内写死了 `Mods/…`），但它仍然是模组。
    pub is_mod: bool,
}

impl Payload {
    /// 落点在游戏目录内的相对路径（**还没有展开战役子目录**，见下）。
    ///
    /// `CampaignMap` 只给出相对战役目录的路径，调用方还得补上战役的子目录
    /// （自由之翼在根下，虫群之心在 `swarm/` ……）—— 展开用
    /// `library::compose::payload_target_path`。
    pub fn target_path(&self) -> String {
        match &self.target {
            PayloadTarget::GamePath { path } => path.clone(),
            PayloadTarget::Mod { name } => format!("Mods/{name}"),
            PayloadTarget::CampaignMap { name } => name.clone(),
        }
    }

    /// 载荷名（最后一个路径片段）。
    pub fn target_name(&self) -> String {
        match &self.target {
            PayloadTarget::GamePath { path } => path.rsplit('/').next().unwrap_or(path).to_string(),
            PayloadTarget::Mod { name } | PayloadTarget::CampaignMap { name } => name.clone(),
        }
    }
}

/// 载荷的落点（三种语义，判定表见 `package::paths@@payload_target`）。
///
/// 序列化后的 `kind` 值保持旧写法（`mirror@@mod@@map`）——
/// 战役库 `library.json` 与前端 IPC 都按它读，改语义不能改协议。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PayloadTarget {
    /// **包内写死了游戏目录内的相对路径**（序列化写作 `mirror`，沿用旧名）：
    /// `Maps/Campaign/…`、`Mods/…`、`Interfaces/…` 或镜像根下的兄弟目录。
    #[serde(rename = "mirror", alias = "game_path")]
    GamePath { path: String },
    /// 模组：`name` 是相对 `Mods/` 的路径（可以带目录），落 `<游戏>/Mods/<name>`。
    ///
    /// 目录结构必须保留 —— 地图依赖里写的就是 `Mods\<作者目录>\…`。
    Mod { name: String },
    /// 战役地图：`name` 是相对**这个战役自己的目录**的路径（可以带目录，如 `evolution/…`）。
    ///
    /// 最终落点由导入槽位决定：官方改版进 `<游戏>/Maps/Campaign[/子目录]/`，
    /// 自制战役进 `<游戏>/Maps/CustomCampaigns/<版本目录>/`。
    ///
    /// 序列化沿用旧名 `map`（见上一条：协议不能改）。
    #[serde(rename = "map", alias = "campaign_map")]
    CampaignMap { name: String },
}

/// 包内一个条目的摘要。
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) index: usize,
    /// 校验并清理后的相对路径。
    pub(crate) relative: PathBuf,
    /// 解压后的字节数（依赖扫描时用来设上限）。
    pub(crate) size: u64,
}

/// 包预检结果。
#[derive(Debug, Clone, Serialize)]
pub struct PackageInspection {
    pub path: PathBuf,
    /// 是否可以安装（没有任何「无法运行」级别的问题）。
    pub installable: bool,
    /// 是不是**缺密码 / 密码不对**（加密包）。
    ///
    /// 界面据此弹密码框：填完带着密码再预检一次。别的失败（包坏了、越界条目）
    /// 都是 `false` —— 那些再问密码也没用。
    pub needs_password: bool,
    pub format: CampaignFormat,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub campaign_type: CampaignType,
    /// 包自报的封面图（相对内容根的路径）；没写就是 `None`，由导入逻辑再按文件名找一次。
    pub cover: Option<String>,
    /// 包自报的标签。
    pub tags: Vec<String>,
    /// 包类型：战役本体，还是覆盖层补丁。
    pub kind: PackageKind,
    /// 注册 ID（补丁靠它引用战役）。
    pub id: Option<String>,
    /// 补丁依赖的战役（注册 ID 或战役名）。
    pub requires: Vec<String>,
    /// 补丁默认优先级；数值大的后覆盖。
    pub priority: Option<i64>,
    /// 载荷清单：地图与模组，以及各自的落点。
    pub payloads: Vec<Payload>,
    /// 包内声明的**主地图**（自制战役的游玩入口），相对包根的路径。
    ///
    /// 只作参考：导入时会校验它是否真的存在，找不到只记一条警告，
    /// 由用户在界面上自己挑（见 `library::resolve_main_map`）。
    #[serde(default)]
    pub main_map: Option<String>,
    /// 作者声明的覆盖规则（v3）。
    pub overrides: Vec<crate::campaign::metadata::OverrideRule>,
    /// 包内声明的 **modid**：这个模组的身份。
    ///
    /// 判定「同一个模组的不同版本，还是另一个模组」全看它。
    #[serde(default)]
    pub modid: Option<String>,
    /// 包内**声明为依赖**的模组键（`Mods/` 之后的第一段）。
    ///
    /// 空表示作者没声明 —— **不代表包不带模组**，界面上按「可选」处理。
    #[serde(default)]
    pub declared_mods: Vec<String>,
    /// 包内声明的**说明文档（PDF）**，相对包根的路径。
    ///
    /// 导入时会连整个包一起解开，所以这份文档就在版本目录里，
    /// 存下相对路径即可，之后直接读出来给界面渲染。
    #[serde(default)]
    pub doc: Option<String>,
    /// 归属判定的结论与依据；元数据已声明时是 `None`。
    pub identification: Option<Identification>,
    /// 按包内声明推断出的目标战役槽位；`None` 表示认不出来，需要用户指定。
    pub suggested_slot: Option<String>,
    /// 包内实际内容根（元数据所在目录），解压时需剥离；空串表示包根。
    pub content_root: String,
    /// 建议的安装目录名（已安全化）。
    pub suggested_dir_name: Option<String>,
    pub map_count: usize,
    pub mod_count: usize,
    pub entry_count: usize,
    pub unpacked_bytes: u64,
    pub issues: Vec<HealthIssue>,
}

impl PackageInspection {
    /// 解析出的显示名，取不到时退回目录名。
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.suggested_dir_name.clone())
            .unwrap_or_else(|| "未命名战役".to_string())
    }

    /// 是否存在「无法运行」级别的问题。
    pub fn has_blocking_issue(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.level == crate::campaign::HealthLevel::Broken)
    }
}

/// 没有任何元数据时的作者名。
pub const UNKNOWN_AUTHOR: &str = "未知作者";

/// 构造一个"不可用"的预检结果。
pub(crate) fn unusable(path: &Path, code: &str, message: String, hint: &str) -> PackageInspection {
    PackageInspection {
        path: path.to_path_buf(),
        installable: false,
        needs_password: false,
        format: CampaignFormat::Unknown,
        name: None,
        author: None,
        version: None,
        description: None,
        campaign_type: CampaignType::Other(String::new()),
        cover: None,
        overrides: Vec::new(),
        tags: Vec::new(),
        main_map: None,
        doc: None,
        declared_mods: Vec::new(),
        modid: None,
        kind: PackageKind::Campaign,
        id: None,
        requires: Vec::new(),
        priority: None,
        payloads: Vec::new(),
        identification: None,
        suggested_slot: None,
        content_root: String::new(),
        suggested_dir_name: None,
        map_count: 0,
        mod_count: 0,
        entry_count: 0,
        unpacked_bytes: 0,
        issues: vec![HealthIssue::broken(code, message).with_hint(hint)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 序列化协议不能变：战役库 `library.json` 与前端 IPC 都按 `kind` 读，
    /// 旧记录（`mirror`/`mod`/`map`）必须还能读，新记录旧版启动器也要能读。
    #[test]
    fn payload_target_json_stays_backward_compatible() {
        let game: PayloadTarget =
            serde_json::from_str(r#"{"kind":"mirror","path":"Maps/Campaign/a.SC2Map"}"#)
                .expect("旧记录能读");
        assert!(
            matches!(&game, PayloadTarget::GamePath { path } if path == "Maps/Campaign/a.SC2Map")
        );
        assert_eq!(
            serde_json::to_string(&game).expect("能写"),
            r#"{"kind":"mirror","path":"Maps/Campaign/a.SC2Map"}"#
        );

        let module: PayloadTarget =
            serde_json::from_str(r#"{"kind":"mod","name":"Alenger/1钢铁.SC2Mod"}"#)
                .expect("旧记录能读");
        assert!(matches!(&module, PayloadTarget::Mod { name } if name == "Alenger/1钢铁.SC2Mod"));
        assert_eq!(
            serde_json::to_string(&module).expect("能写"),
            r#"{"kind":"mod","name":"Alenger/1钢铁.SC2Mod"}"#
        );

        let card: PayloadTarget =
            serde_json::from_str(r#"{"kind":"map","name":"evolution/zevo.SC2Map"}"#)
                .expect("旧记录能读");
        assert!(
            matches!(&card, PayloadTarget::CampaignMap { name } if name == "evolution/zevo.SC2Map")
        );
        assert_eq!(
            serde_json::to_string(&card).expect("能写"),
            r#"{"kind":"map","name":"evolution/zevo.SC2Map"}"#
        );
    }
}
