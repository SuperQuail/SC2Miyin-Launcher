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
    /// 相对内容根的路径（文件或目录）。
    pub source: String,
    /// 应该落到游戏目录的哪里。
    pub target: PayloadTarget,
    /// 是否是解开的目录树（false 表示单文件 MPQ）。
    pub expanded: bool,
    /// 载荷性质：`true` 是模组（`.SC2Mod`），`false` 是地图（`.SC2Map`）。
    ///
    /// 单独记一份，是因为落点形式（镜像 / Mods / Maps）不能代表性质 ——
    /// 包内写死 `Mods/x.SC2Mod` 的镜像载荷，落点是镜像，但它仍然是模组。
    pub is_mod: bool,
}

impl Payload {
    /// 落点在游戏目录内的相对路径。
    ///
    /// `Map` 类型只给出文件名，调用方需要补上战役的子目录
    /// （自由之翼在根下，虫群之心在 `swarm/` ……）。
    pub fn target_path(&self) -> String {
        match &self.target {
            PayloadTarget::Mirror { path } => path.clone(),
            PayloadTarget::Mod { name } => format!("Mods/{name}"),
            PayloadTarget::Map { name } => name.clone(),
        }
    }

    /// 载荷名（最后一个路径片段）。
    pub fn target_name(&self) -> String {
        match &self.target {
            PayloadTarget::Mirror { path } => path.rsplit('/').next().unwrap_or(path).to_string(),
            PayloadTarget::Mod { name } | PayloadTarget::Map { name } => name.clone(),
        }
    }
}

/// 载荷的落点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PayloadTarget {
    /// 包内已经是游戏目录镜像：按这个相对路径落盘。
    Mirror { path: String },
    /// 落到 `<游戏>/Mods/<name>`。
    Mod { name: String },
    /// 落到 `<游戏>/Maps/Campaign/[子目录]/<name>`，子目录由绑定的战役决定。
    Map { name: String },
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
