//! 战役包预检。
//!
//! 在真正解压之前，先把压缩包"读一遍"，回答三个问题：
//!
//! 1. **这是什么格式**（CCM `metadata.txt` / 弥音标准 `metadata.json` / 无元数据）
//! 2. **里面写了什么**（名称、作者、版本、归属资料片、地图数量）
//! 3. **能不能装**（条目名是否越界、是否超过体积与条目数上限、是否根本没有地图）
//!
//! # 相比参考实现修正了什么
//!
//! - **补上了 zip-slip 校验**。参考实现完全依赖第三方库的路径处理行为，
//!   一旦换库或升级就可能把文件写到游戏目录之外。
//! - **解压前先算总字节数与条目数**，挡掉 zip bomb。
//! - **识别元数据所在的内容根**（`MyCampaign/metadata.txt` 这种最常见布局），
//!   解压时剥离该前缀 —— 参考实现处理这种包时必然报 `EPERM/ENOTEMPTY`。
//! - **zip 内文件名编码异常时给出提示**，而不是让用户面对乱码文件夹。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::campaign::contents;
use crate::campaign::identify::{self, Identification, MAX_SCANNED_MAPS};
use crate::campaign::metadata::{
    CampaignType, CcmMetadata, PackageKind, StandardMetadata, decode_text,
};
use crate::campaign::sanitize::sanitize_dir_name;
use crate::campaign::{CampaignFormat, HealthIssue};
use crate::error::Result;

/// 允许解压的最大总字节数（防 zip bomb）。
pub const MAX_UNPACKED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// 允许的最大条目数。
pub const MAX_ENTRIES: usize = 100_000;

/// 当前启动器支持的**弥音扩展格式**版本（见 `docs/package-format.md`）。
///
/// 包内 `miyin.format` 高于这个值时会被明确拒绝，而不是猜着解析。
pub const MIYIN_FORMAT_VERSION: u32 = 1;

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
struct Entry {
    index: usize,
    /// 校验并清理后的相对路径。
    relative: PathBuf,
    /// 解压后的字节数（依赖扫描时用来设上限）。
    size: u64,
}

/// 包预检结果。
#[derive(Debug, Clone, Serialize)]
pub struct PackageInspection {
    pub path: PathBuf,
    /// 是否可以安装（没有任何「无法运行」级别的问题）。
    pub installable: bool,
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
fn unusable(path: &Path, code: &str, message: String, hint: &str) -> PackageInspection {
    PackageInspection {
        path: path.to_path_buf(),
        installable: false,
        format: CampaignFormat::Unknown,
        name: None,
        author: None,
        version: None,
        description: None,
        campaign_type: CampaignType::Other(String::new()),
        cover: None,
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

/// 单张地图最多读多少字节（畸形包防线）。
const MAX_MAP_BYTES: u64 = 64 * 1024 * 1024;

/// 条目路径统一成正斜杠。
fn entry_path(entry: &Entry) -> String {
    entry.relative.to_string_lossy().replace('\\', "/")
}

/// 从地图内部读取依赖声明。
///
/// - **展开形态**（`xxx.SC2Map/DocumentHeader`）：那个小文件就在包里，直接读
/// - **单文件 MPQ**：扫里面的 zlib 流再解压（见 [`identify::inflate_streams`]）
///
/// 只扫前 [`MAX_SCANNED_MAPS`] 张地图 —— 同一个包里的地图几乎不会有不同归属，
/// 扫太多张既慢又没用。
fn collect_map_declarations(
    archive: &mut contents::Contents,
    entries: &[Entry],
    content_root: &str,
    payloads: &[Payload],
) -> Vec<String> {
    let mut declarations = Vec::new();
    let mut scanned = 0usize;

    for payload in payloads {
        if payload.is_mod || scanned >= MAX_SCANNED_MAPS {
            continue;
        }
        scanned += 1;

        let relative = if content_root.is_empty() {
            payload.source.clone()
        } else {
            format!("{content_root}/{}", payload.source)
        };
        let wanted = if payload.expanded {
            format!("{relative}/DocumentHeader")
        } else {
            relative
        };

        let Some(entry) = entries.iter().find(|entry| entry_path(entry) == wanted) else {
            continue;
        };
        if entry.size > MAX_MAP_BYTES {
            continue;
        }
        let Some(blob) = read_entry(archive, entry.index) else {
            continue;
        };

        if payload.expanded {
            declarations.extend(identify::extract_campaign_declarations(&blob));
        } else {
            for stream in identify::inflate_streams(&blob) {
                declarations.extend(identify::extract_campaign_declarations(&stream));
            }
        }
    }

    declarations
}

/// 包内是否直接就是游戏目录镜像（顶层出现 `Maps` 或 `Mods`）。
fn has_mirror_root(entries: &[Entry]) -> bool {
    entries.iter().any(|entry| {
        entry
            .relative
            .components()
            .next()
            .map(|first| {
                // 与 `payload_target` 一致：只认规范拼写
                let name = first.as_os_str().to_string_lossy();
                name == "Maps" || name == "Mods"
            })
            .unwrap_or(false)
    })
}

/// 按**载荷**统计地图与模组数量。
///
/// 不能按文件扩展名数：真实样本里的地图是**解开的目录树**
/// （`tarcade.SC2Map/Base.SC2Data/…`），里面的文件是 `.xml` / `.galaxy`，
/// 按扩展名数会得到「0 张地图」。载荷是按「第一个 .SC2Map/.SC2Mod 组件」认出来的，
/// 单文件和目录树都算一个。
///
/// 模组还要**按文件夹去重**：`Mods/Alenger/` 下面是 15 个 `.SC2Mod`，
/// 那是**一个**模组，不是 15 个。
pub fn count_payloads(payloads: &[Payload]) -> (usize, usize) {
    let maps = payloads.iter().filter(|payload| !payload.is_mod).count();

    let mut keys: Vec<String> = Vec::new();
    for payload in payloads.iter().filter(|payload| payload.is_mod) {
        if let Some(found) = crate::library::mod_identity(payload)
            && !keys.contains(&found.key)
        {
            keys.push(found.key);
        }
    }

    (maps, keys.len())
}

/// 从条目清单里找出所有载荷，并算出各自的落点。
///
/// 取"根"的办法：一条路径里**第一个**以 `.SC2Map` / `.SC2Mod` 结尾的组件就是载荷根，
/// 这样无论它是单文件还是解开的目录树都能正确识别。
fn collect_payloads(entries: &[Entry], content_root: &str) -> Vec<Payload> {
    // 这个包是不是按游戏根目录摆的（根上有 Mods/）—— 决定地图要不要保留目录结构
    let mirrors = mirrors_game_root(entries);
    let mut roots: Vec<(String, bool)> = Vec::new();

    for entry in entries {
        let Some(relative) = strip_prefix(&entry.relative, content_root) else {
            continue;
        };
        // `Miyin/` 是我们自己的附加数据目录（导出时会写上）——
        // 它不是战役内容，重新导入时不能被当成载荷。
        if is_our_metadata_dir(&relative) {
            continue;
        }
        let Some(root) = payload_root(&relative) else {
            continue;
        };
        if !roots.iter().any(|(existing, _)| existing == &root.0) {
            roots.push(root);
        }
    }

    roots
        .into_iter()
        .map(|(source, is_mod)| {
            let expanded = entries.iter().any(|entry| {
                strip_prefix(&entry.relative, content_root)
                    .is_some_and(|relative| is_under(&relative, &source))
            });
            let target = payload_target(&source, is_mod, content_root, mirrors);
            Payload {
                source,
                target,
                expanded,
                is_mod,
            }
        })
        .collect()
}

/// 这条路径是不是落在我们自己的附加数据目录里。
pub fn is_our_metadata_dir(relative: &Path) -> bool {
    relative
        .components()
        .next()
        .map(|first| {
            first
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(crate::library::export::MIYIN_DIR)
        })
        .unwrap_or(false)
}

/// 取相对路径里的载荷根：第一个以 `.SC2Map` / `.SC2Mod` 结尾的组件（含它自己）。
fn payload_root(relative: &Path) -> Option<(String, bool)> {
    let mut parts: Vec<String> = Vec::new();

    for component in relative.components() {
        let name = component.as_os_str().to_string_lossy().into_owned();
        parts.push(name.clone());
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".sc2mod") {
            return Some((parts.join("/"), true));
        }
        if lower.ends_with(".sc2map") {
            return Some((parts.join("/"), false));
        }
    }

    None
}

/// `candidate` 是否位于 `root` 之下（用来判断载荷是不是解开的目录树）。
fn is_under(candidate: &Path, root: &str) -> bool {
    candidate
        .strip_prefix(root)
        .map(|rest| rest.components().next().is_some())
        .unwrap_or(false)
}

/// 这个包是不是**按游戏根目录摆的**。
///
/// 判据：包根直接有个 `Mods/`（不是包名目录下面的）。
///
/// 真实样本教我们的 —— 复刻战役 SCMR 长这样：
///
/// `@text
/// Mods/SCMRassets.SC2Mod          <- 已经在游戏根的位置上了
/// Mods/SCMRmod.SC2Mod
/// Starcraft Mass Recall/          <- 那这个兄弟目录就是 Maps/ 底下的
/// ├── 1. Rebel Yell/Terran01.SC2Map
/// └── SCMR Campaign Launcher.SC2Map
/// `@
///
/// 它的启动器地图里写的是 `GameSetNextMap("Starcraft Mass Recall/1. Rebel Yell/Terran01")`
/// —— 这个路径**相对 Maps/**。所以装完必须长成
/// `Maps/Starcraft Mass Recall/1. Rebel Yell/Terran01.SC2Map`。
///
/// 踩过：这些地图被当成「作者的分类目录」拍平成 `Maps/Campaign/Terran01.SC2Map`，
/// 层级一没，启动器地图就联动不了任何关卡 —— 用户看到的就是
/// 「能打开启动器，但点哪一关都进不去」。
///
/// `Mods/` 是作者给出来的**信号**，不是我们在猜。
fn mirrors_game_root(entries: &[Entry]) -> bool {
    entries.iter().any(|entry| {
        let path = entry.relative.to_string_lossy().replace('\\', "/");
        path.len() > 4 && path.starts_with("Mods/")
    })
}

/// 路径里第一段 `Maps` / `Mods`，从那里往后就是游戏目录内的相对路径。
///
/// **为什么不能只看开头**：包常常多一层「包名」目录 ——
/// 真实样本是 `疯批帝国军械库2.3/Mods/Alenger/1钢铁.SC2Mod`。
/// 只判断 `starts_with("Mods/")` 的话这一层就把整个前缀吃掉了，
/// 掉到下面按 `is_mod` 取「最后一段当名字」，
/// 结果 `Mods/Alenger/1钢铁.SC2Mod` 变成 `Mods/1钢铁.SC2Mod` ——
/// **`Alenger/` 这一层没了**，地图里声明的 `Mods\Alenger\…` 自然找不到。
///
/// 只认**规范拼写**：游戏目录就叫 `Maps` / `Mods`，而包作者拿小写 `maps/`
/// 当普通分类目录用的情况很常见，一律按镜像处理会把它们误送到游戏根下。
fn game_relative(source: &str) -> Option<String> {
    let normalised = source.replace('\\', "/");
    let parts: Vec<&str> = normalised.split('/').collect();

    for (index, part) in parts.iter().enumerate() {
        if *part == "Maps" || *part == "Mods" {
            return Some(parts[index..].join("/"));
        }
    }

    None
}

/// 决定载荷落到游戏目录的哪里。
///
/// 对 crate 内可见是为了能直接测：包名那层怎么剥，只有在这里才说得清。
pub(crate) fn payload_target(
    source: &str,
    is_mod: bool,
    content_root: &str,
    mirrors_game_root: bool,
) -> PayloadTarget {
    // 包内已经摆好了游戏目录那一层（`Maps/…` 或 `Mods/…`）-> 按原路径落盘
    if let Some(relative) = game_relative(source) {
        return PayloadTarget::Mirror { path: relative };
    }

    // 包是**按游戏根目录摆的**（根上有 `Mods/`）：那其余顶层目录就是 `Maps/`
    // 底下的，**结构和名字都要原样保留**。
    //
    // 判据来自作者自己：他既然把 `Mods/` 摆在根上，兄弟目录就是 `Maps/` 的内容。
    // 这里**不能**拍平 —— 复刻战役的启动器地图按 `Starcraft Mass Recall/…`
    // 这个相对 Maps/ 的路径联动下一关，拍平了它就找不到任何关卡。
    if mirrors_game_root {
        let path = source.replace('\\', "/");
        return PayloadTarget::Mirror {
            path: format!("Maps/{path}"),
        };
    }

    let path = source.replace('\\', "/");

    if is_mod {
        // 走到这里说明路径里**没有** `Mods/` 这一段（比如包根光秃秃一个
        // `X.SC2Mod`），那就当单文件模组，落到 `Mods/` 下。
        // 有 `Mods/` 的情况上面已经按原路径处理掉了。
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        return PayloadTarget::Mod { name };
    }

    // 地图该不该保留包内的相对目录，取决于**内容根站在哪里**：
    //
    // - 内容根已经落在官方战役目录里（`X/swarm/metadata.txt`）-> 包内相对路径是
    //   该战役目录**之内**的结构，要原样保留（`evolution/…` -> swarm/evolution/…）
    // - 否则相对路径是**并列**于战役目录的：顶层是官方目录名（`voidprologue/…`）
    //   就说明作者按游戏结构摆好了；其余（`maps/…`）只是作者的分类习惯，只取文件名
    let leading_is_campaign_dir = known_campaign_prefix(&path).is_some();
    let root_is_campaign_dir = content_root
        .rsplit('/')
        .next()
        .is_some_and(is_known_campaign_dir);

    if path.contains('/') && (root_is_campaign_dir || leading_is_campaign_dir) {
        return PayloadTarget::Map { name: path };
    }

    let name = path.rsplit('/').next().unwrap_or(&path).to_string();
    PayloadTarget::Map { name }
}

/// 这个名字是不是官方战役目录（`Maps/Campaign` 下那一层的名字）。
fn is_known_campaign_dir(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "swarm" | "void" | "voidprologue" | "nova" | "campaign"
    )
}

/// 包内相对路径的**顶层目录**是不是官方战役目录名。
///
/// 是的话说明作者已经按游戏的结构摆好了（`voidprologue/…`），落点直接用这条路径；
/// 否则相对路径是相对"这个战役自己的目录"的（`evolution/…` -> swarm/evolution/）。
pub fn known_campaign_prefix(source: &str) -> Option<&'static str> {
    // 长的排前面，避免 `swarm/evolution` 被 `swarm` 抢先匹配
    const KNOWN: [&str; 6] = [
        "swarm/evolution",
        "voidprologue",
        "swarm",
        "void",
        "nova",
        "customcampaigns",
    ];

    let lower = source.to_ascii_lowercase();
    KNOWN
        .into_iter()
        .find(|prefix| lower.starts_with(&format!("{prefix}/")))
}

/// 校验 zip 条目名，并返回清理后的相对路径。
///
/// 返回 `None` 表示该条目**越界或不可用**，必须整体拒绝这个包：
/// 绝对路径、盘符、UNC、以及任何 `..` 组件。
pub fn safe_entry_path(name: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");

    if normalized.starts_with('/') {
        return None;
    }
    // 盘符形式：C:\\foo
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        return None;
    }

    let mut out = PathBuf::new();
    for component in normalized.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            return None;
        }
        // 复用目录名安全化：非法字符替换、保留名拒绝、长度截断
        out.push(sanitize_dir_name(component)?);
    }

    (!out.as_os_str().is_empty()).then_some(out)
}

/// 读取并检查一个战役包。
pub fn inspect(path: &Path) -> Result<PackageInspection> {
    // 任意打包形式：zip 原生读，其它交给系统解压器（见 contents 模块）
    let mut archive = match contents::Contents::open(path) {
        Ok(archive) => archive,
        Err(error) => {
            return Ok(unusable(
                path,
                "NOT_AN_ARCHIVE",
                format!("读不了这个压缩包：{error}"),
                "请确认文件完整；实在读不了就解压后重新打成 zip",
            ));
        }
    };

    let mut issues: Vec<HealthIssue> = Vec::new();
    let mut unpacked_bytes: u64 = 0;
    let mut unsafe_name: Option<String> = None;
    let mut lossy_names = 0usize;

    // 先把清单拷出来，后面读内容还要可变借用 archive
    let listing: Vec<(String, u64, bool, bool)> = archive
        .entries()
        .iter()
        .map(|item| (item.name.clone(), item.size, item.is_dir, item.lossy))
        .collect();

    let mut entries: Vec<Entry> = Vec::with_capacity(listing.len());

    for (index, (raw, size, is_dir, lossy_name)) in listing.into_iter().enumerate() {
        if lossy_name {
            lossy_names += 1;
        }

        let Some(relative) = safe_entry_path(&raw) else {
            unsafe_name.get_or_insert(raw);
            continue;
        };

        if !is_dir {
            unpacked_bytes = unpacked_bytes.saturating_add(size);
        }
        entries.push(Entry {
            index,
            relative,
            size,
        });
    }

    // ---- 安全闸门 ----
    if let Some(name) = unsafe_name {
        issues.push(
            HealthIssue::broken(
                "UNSAFE_ENTRY",
                format!("压缩包内存在越界路径条目，已拒绝安装：{name}"),
            )
            .with_hint("该包含有指向压缩包之外的路径（zip-slip），可能来自恶意文件"),
        );
    }

    if entries.len() > MAX_ENTRIES {
        issues.push(HealthIssue::broken(
            "TOO_MANY_ENTRIES",
            format!("压缩包条目过多（{} 个，上限 {MAX_ENTRIES}）", entries.len()),
        ));
    }

    if unpacked_bytes > MAX_UNPACKED_BYTES {
        issues.push(HealthIssue::broken(
            "TOO_LARGE",
            format!(
                "解压后体积过大（{:.1} GB，上限 {:.0} GB）",
                unpacked_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                MAX_UNPACKED_BYTES as f64 / 1024.0 / 1024.0 / 1024.0
            ),
        ));
    }

    if lossy_names > 0 {
        issues.push(HealthIssue::warning(
            "NON_UTF8_NAMES",
            format!("压缩包内有 {lossy_names} 个文件名不是 UTF-8 编码，解压后可能显示为乱码"),
        ));
    }

    // ---- 元数据定位 ----
    let standard_entry = entries.iter().find(|entry| {
        entry.relative.as_os_str() == "metadata.json"
            || entry.relative.as_os_str() == "Metadata.json"
    });

    // metadata.txt 是 CCM 的；patch.txt 是我们给**没有战役元数据的补丁**准备的同格式文件
    // （现实里的补丁往往只有一个说明.txt，作者想声明依赖就得有个地方写）。
    let ccm_entry = entries
        .iter()
        .filter(|entry| {
            entry.relative.file_name().is_some_and(|name| {
                name.eq_ignore_ascii_case("metadata.txt") || name.eq_ignore_ascii_case("patch.txt")
            })
        })
        .min_by_key(|entry| entry.relative.components().count());

    // 子目录里的 metadata.json 会被忽略：这是参考实现踩过的坑（装了却扫不到）
    if standard_entry.is_none()
        && entries.iter().any(|entry| {
            entry
                .relative
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("metadata.json"))
        })
    {
        issues.push(HealthIssue::warning(
            "NESTED_METADATA_JSON",
            "metadata.json 不在压缩包根目录，已按无元数据包处理",
        ));
    }

    let mut name = None;
    let mut author = None;
    let mut version = None;
    let mut description = None;
    let mut campaign_raw = String::new();
    let mut declared_main_map: Option<String> = None;
    let mut declared_doc: Option<String> = None;
    let mut declared_mods_raw: Vec<String> = Vec::new();
    let mut declared_mod: Option<String> = None;
    let mut declared_cover = None;
    let mut declared_tags: Vec<String> = Vec::new();
    let mut declared_id: Option<String> = None;
    let mut declared_kind = PackageKind::Campaign;
    let mut declared_requires: Vec<String> = Vec::new();
    let mut declared_priority: Option<i64> = None;
    let mut content_root = String::new();
    let mut format = CampaignFormat::Plain;

    if let Some(entry) = standard_entry {
        format = CampaignFormat::Standard;
        content_root = parent_prefix(&entry.relative);
        if let Some(text) = read_entry(&mut archive, entry.index) {
            match StandardMetadata::parse(&decode_text(&text)) {
                Ok(meta) => {
                    // 先借走扩展信息，后面几个字段会被移出
                    declared_main_map = meta.main_map_path();
                    declared_doc = meta.doc_path();
                    declared_mods_raw = meta.mods();
                    declared_mod = meta.modid().map(str::to_string);
                    declared_cover = clean(meta.cover_path().map(str::to_owned));
                    declared_tags = meta.tags();
                    declared_id = clean(meta.id().map(str::to_owned));
                    declared_kind = meta.package_kind();
                    declared_requires = meta.patch_requires();
                    declared_priority = meta.patch_priority();
                    let extension_format =
                        meta.miyin.as_ref().and_then(|extensions| extensions.format);

                    name = clean(meta.name);
                    author = clean(meta.author);
                    version = clean(meta.version);
                    description = clean(meta.description);
                    campaign_raw = clean(meta.campaign).unwrap_or_default();

                    // 扩展格式版本比启动器新 -> 明确拒绝，而不是猜着解析
                    if let Some(format) = extension_format
                        && format > MIYIN_FORMAT_VERSION
                    {
                        issues.push(
                            HealthIssue::broken(
                                "FORMAT_TOO_NEW",
                                format!(
                                    "该包使用了更新的扩展格式（v{format}），当前启动器只支持到 v{MIYIN_FORMAT_VERSION}"
                                ),
                            )
                            .with_hint("请升级弥音启动器后再导入"),
                        );
                    }

                    if let Some(kind) = meta.kind.as_deref()
                        && !kind.eq_ignore_ascii_case("campaign")
                    {
                        issues.push(HealthIssue::broken(
                            "NOT_A_CAMPAIGN",
                            format!("该包的 type 是 {kind}，不是战役包"),
                        ));
                    }
                }
                Err(error) => issues.push(
                    HealthIssue::warning(
                        "BAD_METADATA_JSON",
                        format!("metadata.json 解析失败：{error}"),
                    )
                    .with_hint("将退回使用压缩包文件名作为战役名"),
                ),
            }
        }
    } else if let Some(entry) = ccm_entry {
        format = CampaignFormat::Ccm;
        content_root = parent_prefix(&entry.relative);
        if let Some(text) = read_entry(&mut archive, entry.index) {
            let meta = CcmMetadata::parse(&decode_text(&text));
            name = clean(meta.title);
            author = clean(meta.author);
            version = clean(meta.version);
            description = clean(meta.description);
            campaign_raw = clean(meta.campaign).unwrap_or_default();
            declared_main_map = clean(meta.main_map.clone());
            declared_doc = clean(meta.doc.clone());
            declared_mods_raw = meta.mods.clone();
            declared_mod = clean(meta.modid.clone());
            declared_cover = clean(meta.cover);
            declared_tags = meta.tags.clone();
            declared_id = clean(meta.id.clone());
            declared_kind = meta
                .kind
                .as_deref()
                .map(PackageKind::parse)
                .unwrap_or(PackageKind::Campaign);
            declared_requires = meta.requires.clone();
            declared_priority = meta.priority;
        }
    } else {
        issues.push(
            HealthIssue::warning(
                "NO_METADATA",
                "包内没有 metadata.txt / metadata.json，将按无元数据包安装",
            )
            .with_hint("仍然可以安装，但启动器无法显示作者与版本信息"),
        );
    }

    let mut campaign_type = CampaignType::parse(&campaign_raw);
    if format != CampaignFormat::Plain && !campaign_type.is_actionable() {
        issues.push(
            HealthIssue::warning(
                "UNKNOWN_CAMPAIGN",
                format!("无法识别归属资料片（campaign = \"{campaign_raw}\"），安装后无法启用"),
            )
            .with_hint("该战役仍可安装到自制战役目录，供支持手动选择的工具使用"),
        );
    }

    // ---- 镜像布局优先 ----
    //
    // 包内直接就是游戏目录的样子（顶层有 Maps/ 或 Mods/）时，内容根应当是**包根**。
    // 否则 metadata 所在的那一层会把同一包里的其它目录切掉 —— 真实案例：
    // metadata 在 Maps/Campaign/void/，但内容同时包含 Maps/Campaign/voidprologue/。
    if has_mirror_root(&entries) {
        content_root = String::new();
    }

    // ---- 载荷：地图与模组（文件或解开的目录树） ----
    let payloads = collect_payloads(&entries, &content_root);

    // 启发式：**只有模组、一张地图都没有** 且元数据没表态 -> 判定为补丁。
    // 现实里的补丁（幼儿园补丁、优化覆盖补丁……）正是这个样子：一堆 .SC2Mod，没有 metadata。
    //
    // **必须排在归属判定之前**：补丁本来就没有"归属"这回事（目标由挂到谁身上决定），
    // 先判补丁就不会去猜它是哪部战役的，也不会刷无意义的 CAMPAIGN_UNKNOWN。
    if declared_kind == PackageKind::Campaign
        && !payloads.is_empty()
        && payloads.iter().all(|payload| payload.is_mod)
    {
        declared_kind = PackageKind::Patch;
        issues.push(HealthIssue::warning(
            "PATCH_INFERRED",
            "包内只有模组、没有地图，已按补丁处理",
        ));
    }

    // ---- 归属判定：按可靠度从高到低 ----
    //
    // 1. 元数据里的 campaign 字段（读到就不进这里）
    // 2. 包内镜像路径       Maps/Campaign/void/…
    // 3. 地图内的依赖声明   Void Story (Campaign)
    // 4. 地图文件名前缀     p*
    //
    // 补丁不参与：它的目标战役由"挂到谁身上"决定，不由包里声明。
    let mut identification: Option<Identification> = None;
    // 包是空的就没什么可判的 —— 已经报过 NO_CONTENT，不必再刷一条
    if declared_kind == PackageKind::Campaign
        && !campaign_type.is_actionable()
        && !payloads.is_empty()
    {
        let mirror_paths: Vec<String> = payloads
            .iter()
            .map(|payload| payload.target_path())
            .collect();
        let map_names: Vec<String> = payloads
            .iter()
            .filter(|payload| !payload.is_mod)
            .map(|payload| payload.target_name())
            .collect();
        let declarations =
            collect_map_declarations(&mut archive, &entries, &content_root, &payloads);

        identification = identify::from_mirror_paths(&mirror_paths)
            .or_else(|| identify::from_dependencies(&declarations))
            .or_else(|| identify::from_map_names(&map_names));

        match &identification {
            Some(found) => {
                issues.push(HealthIssue::warning(
                    "CAMPAIGN_IDENTIFIED",
                    format!(
                        "包内没有声明归属，已按{}判定为「{}」（依据：{}）",
                        found.evidence.label(),
                        found.campaign_type.display_name(),
                        found.detail,
                    ),
                ));
                campaign_type = found.campaign_type.clone();
            }
            None => issues.push(
                // 补丁本来就没有"归属"这回事 —— 它的目标由挂到谁身上决定，
                // 所以这里只对战役包报错，免得刷无意义的告警。
                HealthIssue::warning("CAMPAIGN_UNKNOWN", "无法判断这个包属于哪部战役")
                    .with_hint("导入时请手动选择目标战役"),
            ),
        }
    }

    // 按**载荷**数（单文件和目录树都算一个），模组再按文件夹去重
    let (map_count, mod_count) = count_payloads(&payloads);

    // 空包直接判为不可安装，而不是"可安装但没有内容" ——
    // 现实里这多半意味着包是坏的或下载不完整
    if payloads.is_empty() {
        issues.push(
            HealthIssue::broken("NO_CONTENT", "包内没有找到 .SC2Map 地图或 .SC2Mod 模组")
                .with_hint("包可能是坏的、下载不完整，或者根本不是战役包"),
        );
    }

    // 最小地图包：没有任何元数据时，名字取压缩包名、作者记为未知
    if format == CampaignFormat::Plain && author.is_none() {
        author = Some(UNKNOWN_AUTHOR.to_string());
    }

    // 主地图：CCM 写 `mainmap=`，我们的 JSON 写 `main_map`
    let main_map_claim = declared_main_map
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('\\', "/"));

    // 依赖模组：归一化成键（作者写不写 `Mods/` 前缀都认），
    // 再核对包里是不是真有 —— 声明了却没有，几乎总是写错了名字
    let declared_mods: Vec<String> = {
        let mut keys: Vec<String> = declared_mods_raw
            .iter()
            .map(|raw| crate::library::normalize_mod_key(raw))
            .filter(|key| !key.is_empty())
            .collect();
        keys.dedup();

        for key in &keys {
            let present = payloads.iter().any(|payload| {
                crate::library::mod_identity(payload).is_some_and(|found| &found.key == key)
            });
            if !present {
                issues.push(
                    HealthIssue::warning(
                        "MOD_MISSING",
                        format!("元数据声明依赖模组「{key}」，但包里没有它"),
                    )
                    .with_hint("请确认模组打在包里，或者改掉元数据里的 mods"),
                );
            }
        }

        keys
    };

    if declared_kind == PackageKind::Patch && declared_requires.is_empty() {
        issues.push(
            HealthIssue::warning(
                "PATCH_UNBOUND",
                "这是一个没有声明依赖的补丁，只能手动指定要打给哪个战役",
            )
            .with_hint("包作者可以在元数据里写 requires 来让启动器自动匹配"),
        );
    }

    // ---- 目录名 ----
    let zip_stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();

    // 元数据里的名字不可用时（空、纯点、保留名、全是非法字符），
    // 退回使用压缩包文件名 —— 而不是像参考实现那样硬着头皮拿去拼路径。
    if let Some(raw_name) = &name
        && sanitize_dir_name(raw_name).is_none()
    {
        issues.push(
            HealthIssue::warning(
                "UNUSABLE_TITLE",
                format!("元数据中的名称无法用作目录名（{raw_name}），已改用压缩包文件名"),
            )
            .with_hint("请确认这个包是否被损坏或被恶意构造"),
        );
    }

    let suggested_dir_name = sanitize_dir_name(name.as_deref().unwrap_or_default())
        .or_else(|| sanitize_dir_name(&zip_stem));

    if suggested_dir_name.is_none() {
        issues.push(HealthIssue::broken(
            "UNUSABLE_NAME",
            "从包中无法推导出可用的安装目录名",
        ));
    }

    // 先算出来：campaign_type 马上要被移进结构体
    let suggested_slot = campaign_type.main_slot().map(str::to_string);

    Ok(PackageInspection {
        path: path.to_path_buf(),
        installable: !issues
            .iter()
            .any(|issue| issue.level == crate::campaign::HealthLevel::Broken),
        format,
        name,
        author,
        version,
        description,
        campaign_type,
        cover: declared_cover,
        tags: declared_tags,
        kind: declared_kind,
        id: declared_id,
        requires: declared_requires,
        priority: declared_priority,
        payloads,
        main_map: main_map_claim,
        doc: declared_doc,
        declared_mods,
        modid: declared_mod,
        identification,
        suggested_slot,
        content_root,
        suggested_dir_name,
        map_count,
        mod_count,
        entry_count: entries.len(),
        unpacked_bytes,
        issues,
    })
}

/// 解压统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExtractStats {
    pub files: usize,
    pub maps: usize,
    pub mods: usize,
    pub bytes: u64,
}

/// 把压缩包内容解压到目标目录（自动剥离内容根）。
///
/// 每一条落盘路径都会**再做一次包含性校验**：这是写盘的最后一道闸门，
/// 不依赖上游已经校验过。
pub fn extract_to(package: &Path, content_root: &str, destination: &Path) -> Result<ExtractStats> {
    let mut archive = contents::Contents::open(package)?;
    let mut stats = ExtractStats::default();

    // **外部格式一次解完**，别一个条目起一个进程。
    //
    // 逐条 `copy_to` 对 zip 是对的（流式，不用先整包落一遍盘），
    // 但对 rar/7z 就是 164 个条目 = 164 次 tar —— 比整体解一次还慢。
    //
    // `strip` 用内容根有几段目录来算：包常常多套一层「包名」目录，
    // 那一层不该出现在库目录里。
    let strip = content_root
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .count();

    if archive.unpack_all(destination, strip)? {
        // 文件已经在盘上了，统计按条目清单算
        for entry in archive.entries() {
            if entry.is_dir {
                continue;
            }
            stats.files += 1;
            stats.bytes += entry.size;
            match extension_of(Path::new(&entry.name)).as_str() {
                "sc2map" => stats.maps += 1,
                "sc2mod" => stats.mods += 1,
                _ => {}
            }
        }
        return Ok(stats);
    }

    let listing: Vec<(String, bool)> = archive
        .entries()
        .iter()
        .map(|item| (item.name.clone(), item.is_dir))
        .collect();

    for (index, (name, is_dir)) in listing.into_iter().enumerate() {
        let Some(relative) = safe_entry_path(&name) else {
            continue;
        };
        let Some(stripped) = strip_prefix(&relative, content_root) else {
            continue;
        };
        if stripped.as_os_str().is_empty() {
            continue;
        }

        let target = crate::safety::ensure_within(destination, &destination.join(&stripped))?;

        if is_dir {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        archive.copy_to(index, &target)?;
        stats.files += 1;
        stats.bytes += std::fs::metadata(&target)
            .map(|meta| meta.len())
            .unwrap_or(0);
        match extension_of(&stripped).as_str() {
            "sc2map" => stats.maps += 1,
            "sc2mod" => stats.mods += 1,
            _ => {}
        }
    }

    Ok(stats)
}

/// 读取包内某个条目的全部字节。
pub(crate) fn read_entry(archive: &mut contents::Contents, index: usize) -> Option<Vec<u8>> {
    archive.read(index)
}

/// 去掉内容根前缀，得到包内相对路径。
pub(crate) fn strip_prefix(path: &Path, prefix: &str) -> Option<PathBuf> {
    if prefix.is_empty() {
        return Some(path.to_path_buf());
    }
    path.strip_prefix(prefix).ok().map(Path::to_path_buf)
}

/// 取相对路径的父目录字符串（统一用 `/` 分隔）。
fn parent_prefix(path: &Path) -> String {
    let Some(parent) = path.parent() else {
        return String::new();
    };
    parent
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 小写扩展名。
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// 把空字符串与纯空白视为「没有值」。
fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_traversal_and_absolute_entry_names() {
        assert_eq!(safe_entry_path("../evil.txt"), None);
        assert_eq!(safe_entry_path("a/../../evil.txt"), None);
        assert_eq!(safe_entry_path("/etc/passwd"), None);
        assert_eq!(safe_entry_path("C:\\Windows\\system32"), None);
        assert_eq!(safe_entry_path("\\\\server\\share\\x"), None);
    }

    #[test]
    fn accepts_and_normalizes_normal_entry_names() {
        assert_eq!(
            safe_entry_path("MyCampaign/maps/a.SC2Map"),
            Some(PathBuf::from("MyCampaign/maps/a.SC2Map"))
        );
        assert_eq!(
            safe_entry_path("./MyCampaign//b.txt"),
            Some(PathBuf::from("MyCampaign/b.txt"))
        );
    }

    #[test]
    fn derives_content_root_from_nested_metadata() {
        assert_eq!(
            parent_prefix(Path::new("MyCampaign/metadata.txt")),
            "MyCampaign"
        );
        assert_eq!(parent_prefix(Path::new("metadata.txt")), "");
        assert_eq!(parent_prefix(Path::new("a/b/metadata.txt")), "a/b");
    }

    #[test]
    fn strip_prefix_drops_content_root() {
        let stripped = strip_prefix(Path::new("MyCampaign/maps/a.SC2Map"), "MyCampaign");
        assert_eq!(stripped, Some(PathBuf::from("maps/a.SC2Map")));
    }

    /// 用指定条目构造一个 zip。
    fn build_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).expect("create zip");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (entry_name, content) in files {
            writer.start_file(*entry_name, options).expect("start file");
            writer.write_all(content).expect("write");
        }
        writer.finish().expect("finish");
        path
    }

    #[test]
    fn inspects_ccm_package_with_nested_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(
            dir.path(),
            "reborn.zip",
            &[
                (
                    "Reborn/metadata.txt",
                    "title=Wings of Liberty: Reborn\ndesc=重制版\nauthor=SomeCreator\ncampaign=WOL\nversion=1.4.2\n"
                        .as_bytes(),
                ),
                ("Reborn/maps/01.SC2Map", b"stub"),
                ("Reborn/maps/02.SC2Map", b"stub"),
                ("Reborn/Mods/extra.SC2Mod", b"stub"),
            ],
        );

        let inspection = inspect(&archive).expect("预检应成功");
        assert!(inspection.installable, "问题：{:?}", inspection.issues);
        assert_eq!(inspection.format, CampaignFormat::Ccm);
        assert_eq!(inspection.name.as_deref(), Some("Wings of Liberty: Reborn"));
        assert_eq!(inspection.author.as_deref(), Some("SomeCreator"));
        assert_eq!(inspection.version.as_deref(), Some("1.4.2"));
        assert_eq!(inspection.campaign_type, CampaignType::Wol);
        // 最常见的 CCM 布局：元数据在子目录里，解压时必须剥离这一层
        assert_eq!(inspection.content_root, "Reborn");
        assert_eq!(inspection.map_count, 2);
        assert_eq!(inspection.mod_count, 1);
        assert_eq!(
            inspection.suggested_dir_name.as_deref(),
            Some("Wings of Liberty_ Reborn")
        );
    }

    #[test]
    fn refuses_archive_containing_traversal_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(
            dir.path(),
            "evil.zip",
            &[
                ("metadata.txt", b"title=Evil\ncampaign=WOL\n"),
                ("../../escaped.txt", b"gotcha"),
            ],
        );

        let inspection = inspect(&archive).expect("预检应成功");
        assert!(!inspection.installable, "含越界条目的包必须被拒绝");
        assert!(
            inspection
                .issues
                .iter()
                .any(|issue| issue.code == "UNSAFE_ENTRY")
        );
    }

    #[test]
    fn flags_package_without_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(dir.path(), "Plain Pack.zip", &[("maps/a.SC2Map", b"stub")]);

        let inspection = inspect(&archive).expect("预检应成功");
        assert_eq!(inspection.format, CampaignFormat::Plain);
        assert!(inspection.installable, "无元数据包仍可安装");
        assert!(
            inspection
                .issues
                .iter()
                .any(|issue| issue.code == "NO_METADATA")
        );
        // 没有 title 时退回压缩包文件名
        assert_eq!(inspection.suggested_dir_name.as_deref(), Some("Plain Pack"));
    }

    #[test]
    fn reports_non_archive_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("not-a-zip.zip");
        std::fs::write(&path, b"hello world").expect("write");

        let inspection = inspect(&path).expect("预检应成功");
        assert!(!inspection.installable);
        assert!(
            inspection
                .issues
                .iter()
                .any(|issue| issue.code == "NOT_AN_ARCHIVE")
        );
    }
}

#[cfg(test)]
mod payload_tests {
    use super::*;
    use crate::campaign::identify::CampaignEvidence;
    use std::path::PathBuf;

    fn entry(path: &str, _is_dir: bool) -> Entry {
        Entry {
            index: 0,
            relative: PathBuf::from(path),
            size: 0,
        }
    }

    #[test]
    fn detects_single_file_payloads() {
        let entries = vec![
            entry("metadata.txt", false),
            entry("paiur01.SC2Map", false),
            entry("HTXL.SC2Mod", false),
        ];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 2);

        let map = payloads
            .iter()
            .find(|p| p.source == "paiur01.SC2Map")
            .expect("map");
        assert!(!map.expanded, "单文件 MPQ 不应标记为解开的目录");
        assert!(matches!(&map.target, PayloadTarget::Map { name } if name == "paiur01.SC2Map"));

        let mode = payloads
            .iter()
            .find(|p| p.source == "HTXL.SC2Mod")
            .expect("mod");
        assert!(matches!(&mode.target, PayloadTarget::Mod { name } if name == "HTXL.SC2Mod"));
    }

    #[test]
    fn detects_expanded_directory_payloads() {
        // 真实数据里 .SC2Map 经常是一棵解开的目录树
        let entries = vec![
            entry("Maps/Campaign/void/metadata.txt", false),
            entry("Maps/Campaign/void/epiloguestory01.SC2Map/", true),
            entry(
                "Maps/Campaign/void/epiloguestory01.SC2Map/DocumentHeader",
                false,
            ),
            entry(
                "Maps/Campaign/void/epiloguestory01.SC2Map/Base.SC2Data/x.xml",
                false,
            ),
        ];
        let payloads = collect_payloads(&entries, "Maps/Campaign/void");
        assert_eq!(payloads.len(), 1);

        let payload = &payloads[0];
        assert_eq!(payload.source, "epiloguestory01.SC2Map");
        assert!(payload.expanded, "目录树应标记为 expanded");
        // 包内已经是游戏目录镜像，落点保持原路径
        assert!(payload.target_name().starts_with("epiloguestory01.SC2Map"));
    }

    #[test]
    fn mirror_paths_are_preserved() {
        let entries = vec![entry("Mods/Foo.SC2Mod/Base.SC2Data/a.xml", false)];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0].target_path(),
            "Mods/Foo.SC2Mod",
            "镜像路径应当原样保留"
        );
    }

    #[test]
    fn identifies_packages_by_mirror_path_before_falling_back() {
        // 镜像路径是"精确"证据，应当优先于文件名启发式
        let entries = vec![
            entry("Maps/Campaign/void/paiur01.SC2Map", false),
            entry("Maps/Campaign/void/pkorhal01.SC2Map", false),
        ];
        let payloads = collect_payloads(&entries, "");
        let mirror: Vec<String> = payloads.iter().map(|p| p.target_path()).collect();
        let found = identify::from_mirror_paths(&mirror).expect("应当按镜像路径判定");
        assert_eq!(found.campaign_type, CampaignType::Lotv);
        assert_eq!(found.evidence, CampaignEvidence::MirrorPath);
        assert!(found.evidence.is_exact());
    }

    #[test]
    fn falls_back_to_map_name_prefixes() {
        let types = collect_payloads(
            &[
                entry("paiur01.SC2Map", false),
                entry("pkorhal01.SC2Map", false),
                entry("ppurifier02.SC2Map", false),
            ],
            "",
        );
        let names: Vec<String> = types.iter().map(|p| p.target_name()).collect();
        let found = identify::from_map_names(&names).expect("应当按文件名判定");
        assert_eq!(found.campaign_type, CampaignType::Lotv);
        assert_eq!(found.evidence, CampaignEvidence::MapNamePrefix);
        assert!(!found.evidence.is_exact(), "文件名只是启发式，不算精确证据");

        let hots = collect_payloads(
            &[
                entry("zchar01.SC2Map", false),
                entry("zlab02.SC2Map", false),
            ],
            "",
        );
        let names: Vec<String> = hots.iter().map(|p| p.target_name()).collect();
        assert_eq!(
            identify::from_map_names(&names).map(|found| found.campaign_type),
            Some(CampaignType::Hots)
        );

        // 分不清时必须返回 None，交给界面问用户
        let mixed = collect_payloads(
            &[
                entry("paiur01.SC2Map", false),
                entry("zchar01.SC2Map", false),
            ],
            "",
        );
        let names: Vec<String> = mixed.iter().map(|p| p.target_name()).collect();
        assert!(identify::from_map_names(&names).is_none());

        // 自定义命名认不出来，但也不算错
        let unknown = collect_payloads(&[entry("mymap.SC2Map", false)], "");
        let names: Vec<String> = unknown.iter().map(|p| p.target_name()).collect();
        assert!(identify::from_map_names(&names).is_none());
    }

    #[test]
    fn keeps_relative_directories_inside_the_campaign_folder() {
        // 真实样本「酒馆合作虫心」：内容根就是 X/swarm，进化地图在 evolution/ 下。
        // 这一层必须保留，否则进化地图会被摆到虫心主目录，游戏里找不到。
        let entries = vec![
            entry("酒馆合作虫心/swarm/metadata.txt", false),
            entry("酒馆合作虫心/swarm/zchar01.SC2Map", false),
            entry(
                "酒馆合作虫心/swarm/evolution/zevolutionbaneling.SC2Map",
                false,
            ),
        ];
        let payloads = collect_payloads(&entries, "酒馆合作虫心/swarm");

        let plain = payloads
            .iter()
            .find(|p| p.source == "zchar01.SC2Map")
            .expect("普通地图");
        assert_eq!(plain.target_name(), "zchar01.SC2Map");

        let evolution = payloads
            .iter()
            .find(|p| p.source.contains("evolution"))
            .expect("进化地图");
        assert_eq!(
            evolution.target_name(),
            "evolution/zevolutionbaneling.SC2Map",
            "战役目录之内的相对结构要原样保留"
        );
        assert_eq!(
            crate::library::compose::payload_target_path(
                &evolution.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("swarm".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/swarm/evolution/zevolutionbaneling.SC2Map")
        );
    }

    #[test]
    fn recognises_official_campaign_directories_inside_the_package() {
        // 真实样本「净化者纪元幼儿园」：内容根是包根，序章地图放在 voidprologue/。
        // voidprologue 是官方目录名，说明作者按游戏结构摆好了，落点直接用。
        let entries = vec![
            entry("metadata.txt", false),
            entry("paiur01.SC2Map", false),
            entry("voidprologue/voidprologue01.SC2Map", false),
        ];
        let payloads = collect_payloads(&entries, "");

        let main = payloads
            .iter()
            .find(|p| p.source == "paiur01.SC2Map")
            .expect("主线地图");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &main.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/paiur01.SC2Map")
        );

        let prologue = payloads
            .iter()
            .find(|p| p.source.contains("voidprologue"))
            .expect("序章地图");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &prologue.target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/voidprologue/voidprologue01.SC2Map"),
            "官方目录名要当绝对路径用，而不是塞进 void/ 下面"
        );
    }

    #[test]
    fn arbitrary_container_folders_are_flattened() {
        // 作者的分类习惯（maps/、files/…）没有游戏语义，只取文件名 ——
        // 否则地图会落到游戏根本不扫描的目录里
        let entries = vec![entry("metadata.txt", false), entry("maps/01.SC2Map", false)];
        let payloads = collect_payloads(&entries, "");
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].target_name(), "01.SC2Map");
        assert_eq!(
            crate::library::compose::payload_target_path(
                &payloads[0].target,
                &crate::library::compose::Placement::Campaign {
                    sub: Some("void".into())
                },
            )
            .as_deref(),
            Some("Maps/Campaign/void/01.SC2Map")
        );
    }
}
