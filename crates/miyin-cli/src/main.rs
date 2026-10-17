//! 弥音启动器命令行工具：把**战役包的预检结果**变成机器可读的 JSON 与退出码。
//!
//! 为什么存在：其它开发工具与 agent 需要按我们的约定检查一个包（能不能装、
//! 属于哪部战役、会落到游戏目录的哪里），而不该去读 Rust 源码或复刻解析逻辑。
//!
//! 契约（见 docs/cli.md，改了要同步）：
//!
//! - 默认 stdout 只输出 JSON：一个包一个对象，多条时是数组
//! - 退出码：0 全部可安装 / 1 有 broken 级问题 / 2 参数错 / 路径不存在
//! - 人话走 stderr，绝不混进 stdout
//! - --pretty 缩进输出，默认紧凑
//!
//! 只有三条命令，所以参数解析是手写的一小段 —— 为这个引 clap 不划算。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

use miyin_core::campaign::identify::{CampaignEvidence, Identification};
use miyin_core::campaign::metadata::{CampaignType, PackageKind};
use miyin_core::campaign::package::{PackageInspection, Payload, PayloadTarget, inspect};
use miyin_core::library::compose::{Placement, payload_target_path};

/// 参数错 / 路径不存在。
const EXIT_USAGE: u8 = 2;
/// 有 broken 级问题。
const EXIT_BROKEN: u8 = 1;
/// 全部可安装。
const EXIT_OK: u8 = 0;

/// 用法说明（中文，因为人话走 stderr；JSON 的键名仍是英文）。
const USAGE: &str = "\
弥音启动器 CLI —— 检查战役包，输出 JSON

用法：
  miyin-cli inspect <包路径>...        输出 PackageInspection（与界面同源）
  miyin-cli identify <包路径>          输出归属判定（认不出来就如实说不知道）
  miyin-cli placements <包路径> [--slot <slug>]
                                       输出每个载荷在游戏目录内的相对路径

选项：
  --pretty       缩进输出 JSON（默认紧凑）
  -h, --help     显示这份说明
  -V, --version  显示版本

退出码：
  0  全部可安装
  1  存在 broken 级问题（包坏了、下载不完整、越界条目……）
  2  参数错 / 路径不存在

slug 取值：wol / hots / hotsevolution / lotv / lotvprologue / nova / custom
";

/// 运行期错误：一律映射到退出码 2（参数错 / 路径不存在）。
#[derive(Debug)]
struct CliError(String);

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

type CliResult<T> = std::result::Result<T, CliError>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("miyin-cli: {error}");
            eprintln!("试试 miyin-cli --help");
            EXIT_USAGE
        }
    };
    // 显式冲刷：改成 std::process::exit 的话缓冲里的 JSON 会丢
    let _ = std::io::stdout().flush();
    ExitCode::from(code)
}

fn run(args: &[String]) -> CliResult<u8> {
    let mut pretty = false;
    let mut command: Option<String> = None;
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut slot: Option<String> = None;

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        index += 1;
        match arg {
            "--pretty" => pretty = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(EXIT_OK);
            }
            "-V" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return Ok(EXIT_OK);
            }
            "--slot" => {
                let value = args.get(index).ok_or_else(|| {
                    CliError("--slot 后面要跟一个 slug，例如 --slot lotv".to_string())
                })?;
                index += 1;
                slot = Some(value.clone());
            }
            other if other.starts_with('-') => {
                return Err(CliError(format!("不认识的选项：{other}")));
            }
            other if command.is_none() => command = Some(other.to_string()),
            other => paths.push(PathBuf::from(other)),
        }
    }

    let Some(command) = command else {
        return Err(CliError(
            "没有给命令（inspect / identify / placements）".to_string(),
        ));
    };

    match command.as_str() {
        "inspect" => inspect_command(&paths, pretty),
        "identify" => identify_command(&paths, pretty),
        "placements" => placements_command(&paths, slot.as_deref(), pretty),
        other => Err(CliError(format!("不认识命令：{other}"))),
    }
}

/// 恰好一个包路径 —— identify / placements 只要一个，多给就是参数错。
fn single_path<'a>(paths: &'a [PathBuf], command: &str) -> CliResult<&'a Path> {
    match paths {
        [only] => Ok(only.as_path()),
        [] => Err(CliError(format!("{command} 需要一个包路径"))),
        _ => Err(CliError(format!("{command} 只接受一个包路径"))),
    }
}

/// 先证文件存在再动手。
///
/// 核心的 inspect 对"读不了的包"不报错 —— 它返回一条 NOT_AN_ARCHIVE 的 broken 结论，
/// 所以路径写错要靠这里拦，退出码才是 2（参数错）而不是 1（包坏了）。
fn require_file(path: &Path) -> CliResult<()> {
    if path.is_file() {
        return Ok(());
    }
    Err(CliError(format!("找不到这个文件：{}", path.display())))
}

fn inspect_command(paths: &[PathBuf], pretty: bool) -> CliResult<u8> {
    if paths.is_empty() {
        return Err(CliError("inspect 至少需要一个包路径".to_string()));
    }
    // 全部路径先验一遍：错一个就整体不输出，免得调用方拿到半截数组
    for path in paths {
        require_file(path)?;
    }

    let mut results: Vec<PackageInspection> = Vec::with_capacity(paths.len());
    for path in paths {
        results.push(inspect_package(path)?);
    }

    // 一个包一个对象，多条时是数组 —— 别的工具按这个形状取
    match results.as_slice() {
        [only] => emit(only, pretty)?,
        many => emit(&many, pretty)?,
    }

    for result in &results {
        for issue in &result.issues {
            eprintln!(
                "{} [{}] {}",
                result.path.display(),
                issue.code,
                issue.message
            );
        }
    }

    let broken = results.iter().any(PackageInspection::has_blocking_issue);
    Ok(if broken { EXIT_BROKEN } else { EXIT_OK })
}

fn identify_command(paths: &[PathBuf], pretty: bool) -> CliResult<u8> {
    let path = single_path(paths, "identify")?;
    require_file(path)?;

    let inspection = inspect_package(path)?;
    let found = inspection.identification.as_ref();

    // 元数据里声明了归属时，核心不会走证据链（identification 是 None）——
    // 这里如实标成 metadata 这一层，免得调用方把"没有 evidence"理解成"没认出来"
    let evidence = match found {
        Some(found) => Some(found.evidence),
        None if inspection.campaign_type.is_actionable() => Some(CampaignEvidence::Metadata),
        None => None,
    };

    let identified = evidence.is_some();
    if !identified {
        eprintln!("{}", identify_failure(path, &inspection));
    }

    let report = IdentifyReport {
        path: path.to_path_buf(),
        // 认不出来时也如实给出包内声明 —— Other 分支序列化成
        // {"other": "作者写的原文"}，调用方能看见"它自称是什么"
        campaign_type: Some(inspection.campaign_type.clone()),
        slug: identified.then(|| inspection.campaign_type.slug().to_string()),
        evidence,
        evidence_label: evidence.map(|item| item.label().to_string()),
        is_exact: evidence.map(CampaignEvidence::is_exact),
        detail: evidence.map(|item| detail_of(found, item)),
        installable: inspection.installable,
    };
    emit(&report, pretty)?;

    for issue in &inspection.issues {
        eprintln!("{} [{}] {}", path.display(), issue.code, issue.message);
    }

    Ok(if inspection.has_blocking_issue() {
        EXIT_BROKEN
    } else {
        EXIT_OK
    })
}

fn placements_command(paths: &[PathBuf], slot: Option<&str>, pretty: bool) -> CliResult<u8> {
    let path = single_path(paths, "placements")?;
    require_file(path)?;

    let inspection = inspect_package(path)?;
    let file_stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();

    // 槽位：--slot 优先（手动指定永远最高优先级，见 AGENTS.md 14.8），
    // 其次用包内声明的归属，都没有就只能请调用方指定 —— 不猜
    let raw_slot = match slot {
        Some(slot) => slot.to_string(),
        None => match &inspection.suggested_slot {
            Some(found) => found.clone(),
            None => {
                return Err(CliError(format!(
                    "认不出这个包属于哪个槽位（{}），请用 --slot <slug> 指定",
                    path.display()
                )));
            }
        },
    };
    let Some(campaign_type) = CampaignType::from_slug(&raw_slot) else {
        return Err(CliError(format!(
            "不认识的 slug：{raw_slot}（可用：wol / hots / hotsevolution / lotv / lotvprologue / nova / custom）"
        )));
    };

    // 自制战役要用版本目录名当 Maps/CustomCampaigns/ 下面那层；
    // 还没入库的包就用建议的安装目录名顶上
    let folder = inspection
        .suggested_dir_name
        .clone()
        .unwrap_or_else(|| file_stem.clone());
    let placement = Placement::of(campaign_type.slug(), &folder, campaign_type.sub_directory());

    let payloads = expand_payloads(&inspection.payloads, &placement);
    let outside = payloads.iter().filter(|item| item.target.is_none()).count();
    for item in payloads.iter().filter(|item| item.target.is_none()) {
        eprintln!(
            "{}: 载荷「{}」不进游戏目录（包内落点 {}）",
            path.display(),
            item.source,
            item.raw_target
        );
    }

    let report = PlacementsReport {
        path: path.to_path_buf(),
        slot: campaign_type.slug().to_string(),
        custom: campaign_type.is_custom(),
        folder: placement_folder(&placement),
        base: placement.base(),
        summary: PlacementsSummary {
            payloads: payloads.len(),
            inside_game: payloads.len() - outside,
            outside_game: outside,
        },
        payloads,
    };
    emit(&report, pretty)?;

    Ok(if inspection.has_blocking_issue() {
        EXIT_BROKEN
    } else {
        EXIT_OK
    })
}

/// 预检一个包。核心对读不了的包返回一条 broken 结论，这里只兜住真正的 IO 失败。
fn inspect_package(path: &Path) -> CliResult<PackageInspection> {
    inspect(path).map_err(|error| CliError(format!("读不了 {}：{error}", path.display())))
}

/// 认不出来时给一句人话（走 stderr），stdout 只留 JSON。
fn identify_failure(path: &Path, inspection: &PackageInspection) -> String {
    let hint = if inspection.kind == PackageKind::Patch {
        "补丁没有归属 —— 目标由挂到哪个战役身上决定"
    } else {
        "四层证据都没命中，请手动指定目标战役"
    };
    format!("认不出 {} 属于哪部战役：{hint}", path.display())
}

/// 这条证据的具体内容。
///
/// 元数据那一层核心不留 detail（包内声明的原文就在 campaign_type 里），这里补一句。
fn detail_of(found: Option<&Identification>, evidence: CampaignEvidence) -> String {
    match found {
        Some(found) => found.detail.clone(),
        None => format!("包内声明的资料片（{evidence:?}）"),
    }
}

fn placement_folder(placement: &Placement) -> Option<String> {
    match placement {
        Placement::Custom { folder } => Some(folder.clone()),
        Placement::Campaign { .. } => None,
    }
}

/// 每个载荷在游戏目录内的相对路径。
///
/// 核心的 payload_target_path 返回 None 表示**这个载荷不进游戏目录**，
/// 这时如实把 target 留成 null、同时给出包内落点 —— 不凭空编一条路径。
fn expand_payloads(payloads: &[Payload], placement: &Placement) -> Vec<PlacementItem> {
    payloads
        .iter()
        .map(|payload| {
            let kind = match &payload.target {
                PayloadTarget::Mirror { .. } => "mirror",
                PayloadTarget::Mod { .. } => "mod",
                PayloadTarget::Map { .. } => "map",
            };
            PlacementItem {
                source: payload.source.clone(),
                kind: kind.to_string(),
                expanded: payload.expanded,
                is_mod: payload.is_mod,
                target: payload_target_path(&payload.target, placement),
                raw_target: payload.target_path(),
            }
        })
        .collect()
}

/// 把一个值序列化进 stdout（默认紧凑，--pretty 缩进）。
fn emit<T: Serialize>(value: &T, pretty: bool) -> CliResult<()> {
    let text = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|error| CliError(format!("序列化失败：{error}")))?;

    let mut stdout = std::io::stdout();
    writeln!(stdout, "{text}").map_err(|error| CliError(format!("写 stdout 失败：{error}")))
}

/// identify 的输出。campaign_type 与 slug 为 null 就是**认不出来**。
#[derive(Debug, Serialize)]
struct IdentifyReport {
    path: PathBuf,
    campaign_type: Option<CampaignType>,
    slug: Option<String>,
    evidence: Option<CampaignEvidence>,
    evidence_label: Option<String>,
    is_exact: Option<bool>,
    detail: Option<String>,
    installable: bool,
}

/// placements 的输出。
#[derive(Debug, Serialize)]
struct PlacementsReport {
    path: PathBuf,
    slot: String,
    custom: bool,
    /// 自制战役在 Maps/CustomCampaigns/ 下的那层目录名；官方战役为 null。
    folder: Option<String>,
    /// 这个槽位的落点基址。
    base: String,
    summary: PlacementsSummary,
    payloads: Vec<PlacementItem>,
}

#[derive(Debug, Serialize)]
struct PlacementsSummary {
    payloads: usize,
    inside_game: usize,
    outside_game: usize,
}

#[derive(Debug, Serialize)]
struct PlacementItem {
    /// 包内相对内容根的路径。
    source: String,
    /// 载荷落点类型：mirror / mod / map。
    kind: String,
    /// 是不是解开的目录树（false 表示单文件 MPQ）。
    expanded: bool,
    /// 载荷性质：true 是模组，false 是地图。
    is_mod: bool,
    /// 游戏目录内的相对路径；null = 不进游戏目录。
    target: Option<String>,
    /// 包自己给的落点，target 为 null 时看它。
    raw_target: String,
}

// 仓库惯例是测试放同文件的 mod tests，但那样 main.rs 会越过 500 行门禁（AGENTS.md §6）——
// 挪成兄弟文件，模块名保持不变。
#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests;
