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

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::campaign::identify::{self, Identification, MAX_SCANNED_MAPS};
use crate::campaign::metadata::{
    CampaignType, CcmMetadata, PackageKind, StandardMetadata, decode_text,
};
use crate::campaign::sanitize::sanitize_dir_name;
use crate::campaign::{CampaignFormat, HealthIssue};
use crate::error::{Error, Result};

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
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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

/// 识别 rar / 7z；zip 返回 `None`。
fn unsupported_archive(path: &Path) -> Option<&'static str> {
    use std::io::Read;

    let mut magic = [0u8; 6];
    let mut file = std::fs::File::open(path).ok()?;
    file.read_exact(&mut magic).ok()?;

    if magic.starts_with(b"Rar!") {
        return Some("RAR");
    }
    if magic == [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C] {
        return Some("7z");
    }
    None
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
    archive: &mut zip::ZipArchive<std::fs::File>,
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
                let name = first.as_os_str().to_string_lossy().to_ascii_lowercase();
                name == "maps" || name == "mods"
            })
            .unwrap_or(false)
    })
}

/// 从条目清单里找出所有载荷，并算出各自的落点。
///
/// 取"根"的办法：一条路径里**第一个**以 `.SC2Map` / `.SC2Mod` 结尾的组件就是载荷根，
/// 这样无论它是单文件还是解开的目录树都能正确识别。
fn collect_payloads(entries: &[Entry], content_root: &str) -> Vec<Payload> {
    let mut roots: Vec<(String, bool)> = Vec::new();

    for entry in entries {
        let Some(relative) = strip_prefix(&entry.relative, content_root) else {
            continue;
        };
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
            let target = payload_target(&source, is_mod);
            Payload {
                source,
                target,
                expanded,
                is_mod,
            }
        })
        .collect()
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

/// 决定载荷落到游戏目录的哪里。
fn payload_target(source: &str, is_mod: bool) -> PayloadTarget {
    let lower = source.to_ascii_lowercase();

    // 包内已经是游戏目录镜像 -> 按原路径落盘（仍然要过白名单校验）
    if lower.starts_with("maps/") || lower.starts_with("mods/") {
        return PayloadTarget::Mirror {
            path: source.replace('\\', "/"),
        };
    }

    let name = source.rsplit('/').next().unwrap_or(source).to_string();
    if is_mod {
        PayloadTarget::Mod { name }
    } else {
        PayloadTarget::Map { name }
    }
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
    // 先说清楚不支持的格式，而不是让用户对着"不是有效 zip"发呆
    if let Some(kind) = unsupported_archive(path) {
        return Ok(unusable(
            path,
            "UNSUPPORTED_ARCHIVE",
            format!("暂不支持 {kind} 压缩包"),
            "请先用压缩软件解压，再重新打成 .zip",
        ));
    }

    let file = std::fs::File::open(path)?;
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(archive) => archive,
        Err(error) => {
            return Ok(unusable(
                path,
                "NOT_AN_ARCHIVE",
                format!("不是有效的 zip 压缩包：{error}"),
                "请确认下载完整；如果是 .rar / .7z，请解压后重新打成 zip",
            ));
        }
    };

    let mut issues: Vec<HealthIssue> = Vec::new();
    let mut entries: Vec<Entry> = Vec::with_capacity(archive.len());
    let mut unpacked_bytes: u64 = 0;
    let mut unsafe_name: Option<String> = None;
    let mut lossy_names = 0usize;

    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        let raw = file.name().to_string();
        let lossy_name = std::str::from_utf8(file.name_raw()).is_err();
        if lossy_name {
            lossy_names += 1;
        }
        let size = file.size();
        let is_dir = file.is_dir();
        drop(file);

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

    let ccm_entry = entries
        .iter()
        .filter(|entry| {
            entry
                .relative
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("metadata.txt"))
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

    // ---- 归属判定：按可靠度从高到低 ----
    //
    // 1. 元数据里的 campaign 字段（读到就不进这里）
    // 2. 包内镜像路径       Maps/Campaign/void/…
    // 3. 地图内的依赖声明   Void Story (Campaign)
    // 4. 地图文件名前缀     p*
    //
    // 补丁不参与：它的目标战役由"挂到谁身上"决定，不由包里声明。
    let mut identification: Option<Identification> = None;
    if declared_kind == PackageKind::Campaign && !campaign_type.is_actionable() {
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
                HealthIssue::warning("CAMPAIGN_UNKNOWN", "无法判断这个包属于哪部战役")
                    .with_hint("导入时请手动选择目标战役"),
            ),
        }
    }

    let map_count = payloads.iter().filter(|payload| !payload.is_mod).count();
    let mod_count = payloads.iter().filter(|payload| payload.is_mod).count();

    if payloads.is_empty() {
        issues.push(
            HealthIssue::warning("NO_CONTENT", "包内没有找到 .SC2Map 地图或 .SC2Mod 模组")
                .with_hint("这可能不是战役包，或使用了未支持的打包方式"),
        );
    }

    // 最小地图包：没有任何元数据时，名字取压缩包名、作者记为未知
    if format == CampaignFormat::Plain && author.is_none() {
        author = Some(UNKNOWN_AUTHOR.to_string());
    }

    // 启发式：**只有模组、一张地图都没有** 且元数据没表态 -> 判定为补丁。
    // 现实里的补丁（幼儿园补丁、优化覆盖补丁……）正是这个样子：一堆 .SC2Mod，没有 metadata。
    if declared_kind == PackageKind::Campaign
        && !payloads.is_empty()
        && payloads.iter().all(|payload| payload.is_mod)
        && !matches!(campaign_type, CampaignType::Other(_))
    {
        declared_kind = PackageKind::Patch;
    } else if declared_kind == PackageKind::Campaign
        && !payloads.is_empty()
        && payloads.iter().all(|payload| payload.is_mod)
    {
        declared_kind = PackageKind::Patch;
        issues.push(HealthIssue::warning(
            "PATCH_INFERRED",
            "包内只有模组、没有地图，已按补丁处理",
        ));
    }

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
    let file = std::fs::File::open(package)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| Error::PackageRejected(format!("不是有效的 zip 压缩包：{error}")))?;

    let mut stats = ExtractStats::default();

    for (index, relative, is_dir) in collect_entries(&mut archive) {
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
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut source = archive
            .by_index(index)
            .map_err(|error| Error::Parse(format!("读取压缩包条目失败：{error}")))?;
        let mut destination_file = std::fs::File::create(&target)?;
        let written = std::io::copy(&mut source, &mut destination_file)?;

        stats.files += 1;
        stats.bytes += written;
        match extension_of(&stripped).as_str() {
            "sc2map" => stats.maps += 1,
            "sc2mod" => stats.mods += 1,
            _ => {}
        }
    }

    Ok(stats)
}

/// 收集包内条目（供解压使用）。
pub(crate) fn collect_entries(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Vec<(usize, PathBuf, bool)> {
    let mut out = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        let raw = file.name().to_string();
        let is_dir = file.is_dir();
        drop(file);
        if let Some(relative) = safe_entry_path(&raw) {
            out.push((index, relative, is_dir));
        }
    }
    out
}

/// 读取包内某个条目的全部字节。
pub(crate) fn read_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    index: usize,
) -> Option<Vec<u8>> {
    let mut file = archive.by_index(index).ok()?;
    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer).ok()?;
    Some(buffer)
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
    fn rejects_unsupported_archive_by_magic() {
        let dir = tempfile::tempdir().expect("tempdir");

        let rar = dir.path().join("x.rar");
        std::fs::write(&rar, b"Rar!\x1a\x07\x01\x00rest").expect("write");
        assert_eq!(unsupported_archive(&rar), Some("RAR"));

        let zip = dir.path().join("x.zip");
        std::fs::write(&zip, b"PK\x03\x04rest").expect("write");
        assert_eq!(unsupported_archive(&zip), None);
    }
}
