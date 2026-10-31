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
//!
//! # 拆分
//!
//! - types：结果与载荷的形状、体积上限（对外的类型都在这里）
//! - paths：包内路径 -> 游戏目录落点的纯函数
//! - payloads：从条目清单里认出地图与模组
//! - extract：解压到目标目录
//!
//! inspect 本身只做编排：安全闸门 -> 元数据 -> 镜像根 -> 载荷 -> 归属 -> 命名。
//! 六步各有一个私有函数，**顺序不能换**（后一步依赖前一步的结论，问题清单也按这个顺序出）。

mod declared;
mod extract;
mod paths;
mod payloads;
mod types;

#[cfg(test)]
mod tests;

pub use extract::{ExtractStats, extract_to, extract_to_with_password};
#[cfg(test)]
pub(crate) use paths::payload_target;
pub use paths::{known_campaign_prefix, safe_entry_path};
pub use payloads::{count_payloads, is_our_metadata_dir};
pub use types::{
    MAX_ENTRIES, MAX_UNPACKED_BYTES, MIYIN_FORMAT_VERSION, PackageInspection, Payload,
    PayloadTarget, UNKNOWN_AUTHOR,
};

use std::path::{Path, PathBuf};

use crate::campaign::contents;
use crate::campaign::identify::{self, Identification, MAX_SCANNED_MAPS};
use crate::campaign::metadata::{CampaignType, PackageKind};
use crate::campaign::sanitize::sanitize_dir_name;
use crate::campaign::{CampaignFormat, HealthIssue};
use crate::error::{Error, Result};

use self::declared::read_declarations;
use self::extract::read_entry;
use self::payloads::{collect_payloads, has_mirror_root};
use self::types::{Entry, unusable};

/// 读取并检查一个战役包。
pub fn inspect(path: &Path) -> Result<PackageInspection> {
    inspect_with_password(path, None)
}

/// 读取并检查一个战役包，带密码。
///
/// 加密的包**不当作错误**：返回 `installable = false` + `needs_password = true`
/// 和一条稳定的 issue（`PASSWORD_REQUIRED` / `PASSWORD_WRONG`），界面据此弹密码框，
/// 用户填完带着密码再预检一次。
pub fn inspect_with_password(path: &Path, password: Option<&str>) -> Result<PackageInspection> {
    let inspection = inspect_source(path, password)?;
    fallback_single_subdir(path, inspection)
}

/// 真正的预检（不带「唯一子文件夹」兜底，兜底那边要复用这一份）。
fn inspect_source(path: &Path, password: Option<&str>) -> Result<PackageInspection> {
    // 任意打包形式：zip / 目录原生读，其它交给系统解压器（见 contents 模块）
    let mut archive = match contents::Contents::open_source(path, password) {
        Ok(archive) => archive,
        Err(error @ (Error::PasswordRequired(_) | Error::WrongPassword)) => {
            return Ok(password_needed(path, &error));
        }
        Err(error) => {
            return Ok(unusable(
                path,
                "NOT_AN_ARCHIVE",
                format!("读不了这个压缩包：{error}"),
                "请确认文件完整；实在读不了就解压后重新打成 zip",
            ));
        }
    };

    // ---- 安全闸门 ----
    let (entries, unpacked_bytes, mut issues) = scan_entries(&archive);

    // ---- 元数据 ----
    let (declared, mut campaign_type) = read_declarations(&mut archive, &entries, &mut issues);

    // ---- 镜像根 ----
    let content_root = resolve_content_root(&entries, declared.content_root);

    // ---- 载荷 ----
    let mut kind = declared.kind;
    let payloads = build_payloads(&entries, &content_root, &mut kind, &mut issues);

    // ---- 归属 ----
    let identification = resolve_identification(
        &mut archive,
        &entries,
        &content_root,
        &payloads,
        kind,
        &mut campaign_type,
        &mut issues,
    );

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
    let mut author = declared.author;
    if declared.format == CampaignFormat::Plain && author.is_none() {
        author = Some(UNKNOWN_AUTHOR.to_string());
    }

    let declared_mods = resolve_declared_mods(&declared.mods_raw, &payloads, &mut issues);

    if kind == PackageKind::Patch && declared.requires.is_empty() {
        issues.push(
            HealthIssue::warning(
                "PATCH_UNBOUND",
                "这是一个没有声明依赖的补丁，只能手动指定要打给哪个战役",
            )
            .with_hint("包作者可以在元数据里写 requires 来让启动器自动匹配"),
        );
    }

    // ---- 命名 ----
    let suggested_dir_name = resolve_dir_name(path, declared.name.as_deref(), &mut issues);

    // 先算出来：campaign_type 马上要被移进结构体
    let suggested_slot = campaign_type.main_slot().map(str::to_string);

    Ok(PackageInspection {
        path: path.to_path_buf(),
        installable: !issues
            .iter()
            .any(|issue| issue.level == crate::campaign::HealthLevel::Broken),
        needs_password: false,
        format: declared.format,
        name: declared.name,
        author,
        version: declared.version,
        description: declared.description,
        campaign_type,
        cover: declared.cover,
        tags: declared.tags,
        kind,
        id: declared.id,
        requires: declared.requires,
        priority: declared.priority,
        payloads,
        overrides: declared.overrides,
        main_map: declared.main_map,
        doc: declared.doc,
        declared_mods,
        modid: declared.modid,
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

/// 「缺密码 / 密码不对」那一份结果。
///
/// **不是错误** —— 加密包本来就该先问密码，把它报成「读不了」会让用户以为文件坏了。
fn password_needed(path: &Path, error: &Error) -> PackageInspection {
    let mut result = match error {
        Error::PasswordRequired(message) => unusable(
            path,
            "PASSWORD_REQUIRED",
            message.clone(),
            "请输入包作者提供的密码；忘了密码只能请作者重新打包",
        ),
        _ => unusable(
            path,
            "PASSWORD_WRONG",
            "压缩包密码不对，请重新输入".to_string(),
            "确认密码是否抄错（注意大小写）；多次失败也可能是包已损坏",
        ),
    };
    result.needs_password = true;
    result
}

/// 唯一子文件夹兜底：用户「从文件夹导入」时常常选的是**包外面那一层**。
///
/// 只在来源是文件夹、预检说不清归属、且顶层恰好只有一个子目录时，拿那个子目录
/// 当根再读一次（**最多一层**）。读到东西就换成它的结果，并把那一层补进内容根 ——
/// 之后 `extract_to(原路径, 这个内容根)` 仍然自洽，剥离出来的就是真正的包内容。
fn fallback_single_subdir(path: &Path, inspection: PackageInspection) -> Result<PackageInspection> {
    if !path.is_dir() || inspection.suggested_slot.is_some() {
        return Ok(inspection);
    }

    let Some(child) = single_subdir(path) else {
        return Ok(inspection);
    };
    let Some(prefix) = child
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
    else {
        return Ok(inspection);
    };

    let nested = inspect_source(&child, None)?;
    if nested.has_blocking_issue() || nested.payloads.is_empty() {
        return Ok(inspection);
    }

    Ok(PackageInspection {
        path: path.to_path_buf(),
        content_root: if nested.content_root.is_empty() {
            prefix
        } else {
            format!("{prefix}/{}", nested.content_root)
        },
        ..nested
    })
}

/// 顶层唯一的子目录（散文件不算 —— 用户选的那层常带一个说明.txt）。
fn single_subdir(dir: &Path) -> Option<PathBuf> {
    let mut found: Option<PathBuf> = None;

    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(entry.path());
    }

    found
}

/// 阶段 1：安全闸门。
///
/// 把条目清单清出来，同时挡掉越界路径、超量条目、超大体量与非 UTF-8 文件名。
fn scan_entries(archive: &contents::Contents) -> (Vec<Entry>, u64, Vec<HealthIssue>) {
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

    (entries, unpacked_bytes, issues)
}

/// 阶段 3：内容根。
///
/// 镜像包（顶层直接就是 Maps / Mods / Interfaces）的内容根必须是**包根** ——
/// 否则 metadata 所在的那一层会把同包的其它目录切掉：真实案例是 metadata 在
/// Maps/Campaign/void/，可内容同时包含 Maps/Campaign/voidprologue/。
fn resolve_content_root(entries: &[Entry], discovered: String) -> String {
    if has_mirror_root(entries) {
        return String::new();
    }
    discovered
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

/// 阶段 4：载荷（地图与模组，单文件或解开的目录树）。
///
/// 顺带判补丁：**只有模组、一张地图都没有** 且元数据没表态时，按补丁处理。
fn build_payloads(
    entries: &[Entry],
    content_root: &str,
    kind: &mut PackageKind,
    issues: &mut Vec<HealthIssue>,
) -> Vec<Payload> {
    let payloads = collect_payloads(entries, content_root);

    // 启发式：**只有模组、一张地图都没有** 且元数据没表态 -> 判定为补丁。
    // 现实里的补丁（幼儿园补丁、优化覆盖补丁……）正是这个样子：一堆 .SC2Mod，没有 metadata。
    //
    // **必须排在归属判定之前**：补丁本来就没有"归属"这回事（目标由挂到谁身上决定），
    // 先判补丁就不会去猜它是哪部战役的，也不会刷无意义的 CAMPAIGN_UNKNOWN。
    if *kind == PackageKind::Campaign
        && !payloads.is_empty()
        && payloads.iter().all(|payload| payload.is_mod)
    {
        *kind = PackageKind::Patch;
        issues.push(HealthIssue::warning(
            "PATCH_INFERRED",
            "包内只有模组、没有地图，已按补丁处理",
        ));
    }

    payloads
}

/// 阶段 5：归属判定。
///
/// 按可靠度从高到低：包内镜像路径 -> 地图内的依赖声明 -> 地图文件名前缀。
/// 元数据已经声明归属、是补丁、或者包是空的时候都不进来。
fn resolve_identification(
    archive: &mut contents::Contents,
    entries: &[Entry],
    content_root: &str,
    payloads: &[Payload],
    kind: PackageKind,
    campaign_type: &mut CampaignType,
    issues: &mut Vec<HealthIssue>,
) -> Option<Identification> {
    // 包是空的就没什么可判的 —— 已经报过 NO_CONTENT，不必再刷一条；
    // 补丁也不参与：它的目标战役由"挂到谁身上"决定，不由包里声明。
    if kind != PackageKind::Campaign || campaign_type.is_actionable() || payloads.is_empty() {
        return None;
    }

    let mirror_paths: Vec<String> = payloads
        .iter()
        .map(|payload| payload.target_path())
        .collect();
    let map_names: Vec<String> = payloads
        .iter()
        .filter(|payload| !payload.is_mod)
        .map(|payload| payload.target_name())
        .collect();
    let declarations = collect_map_declarations(archive, entries, content_root, payloads);

    let identification = identify::from_mirror_paths(&mirror_paths)
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
            *campaign_type = found.campaign_type.clone();
        }
        None => issues.push(
            // 补丁本来就没有"归属"这回事 —— 它的目标由挂到谁身上决定，
            // 所以这里只对战役包报错，免得刷无意义的告警。
            HealthIssue::warning("CAMPAIGN_UNKNOWN", "无法判断这个包属于哪部战役")
                .with_hint("导入时请手动选择目标战役"),
        ),
    }

    identification
}

/// 依赖模组：归一化成键（作者写不写 Mods/ 前缀都认），
/// 再核对包里是不是真有 —— 声明了却没有，几乎总是写错了名字。
fn resolve_declared_mods(
    declared: &[String],
    payloads: &[Payload],
    issues: &mut Vec<HealthIssue>,
) -> Vec<String> {
    let mut keys: Vec<String> = declared
        .iter()
        .map(|raw| crate::library::normalize_mod_key(raw))
        .filter(|key| !key.is_empty())
        .collect();
    keys.dedup();

    for key in &keys {
        if !declared_mod_present(payloads, key) {
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
}

/// 包里有没有这个模组。
///
/// **两边都要归一化**：声明侧已经过 `normalize_mod_key`（去掉 `.SC2Mod`），
/// 而 `mod_identity` 的 key 是目录里的真名（可能带后缀）。直接 `==` 会误报
/// 「元数据声明依赖模组 X，但包里没有它」—— issue #36。
fn declared_mod_present(payloads: &[Payload], key: &str) -> bool {
    // 传进来的 key 可能已经是归一化的（正常调用路径），也可能是作者手写的原始名字（测试/诊断用）——
    // 两边都过一遍，这个函数才对调用方无害。
    let wanted = crate::library::normalize_mod_key(key);
    payloads.iter().any(|payload| {
        crate::library::mod_identity(payload)
            .is_some_and(|found| crate::library::normalize_mod_key(&found.key) == wanted)
    })
}

/// 来源的名字：文件取文件名（去掉后缀），**目录取目录名**。
///
/// 目录不能走 `file_stem`：`StarCraft v1.2` 那种会被截成 `StarCraft v1`。
pub fn source_stem(path: &Path) -> Option<String> {
    let name = if path.is_dir() {
        path.file_name()
    } else {
        path.file_stem()
    };
    name.map(|value| value.to_string_lossy().into_owned())
}

/// 阶段 6：安装目录名。
///
/// 元数据里的名字不可用时（空、纯点、保留名、全是非法字符）退回压缩包文件名，
/// 而不是像参考实现那样硬着头皮拿去拼路径。
fn resolve_dir_name(
    path: &Path,
    name: Option<&str>,
    issues: &mut Vec<HealthIssue>,
) -> Option<String> {
    let zip_stem = source_stem(path).unwrap_or_default();

    if let Some(raw_name) = name
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

    let suggested_dir_name =
        sanitize_dir_name(name.unwrap_or_default()).or_else(|| sanitize_dir_name(&zip_stem));

    if suggested_dir_name.is_none() {
        issues.push(HealthIssue::broken(
            "UNUSABLE_NAME",
            "从包中无法推导出可用的安装目录名",
        ));
    }

    suggested_dir_name
}
