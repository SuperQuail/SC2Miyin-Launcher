//! 把库里的版本导出成战役包：**弥音包**（CCM 也能读）与**星际枢纽兼容包**。
//!
//! 两种格式各写一份内容，布局不同：
//!
//! - **弥音包**：顶层平铺地图/模组 + `metadata.txt`（CCM 读）+ 根 `metadata.json`
//!   （我们自己的标准格式，带 `miyin` 扩展）+ `Miyin/`（封面、说明、补丁绑定与原件）。
//!   启动器读回时能还原名称、作者、版本、注册 ID、标签、封面、说明、主地图与覆盖规则。
//! - **星际枢纽兼容包**：根 `metadata.json` 满足 SCNexus 标准 schema
//!   （`reference/scnexus/packages/standard/src/metadata/metadata.type.ts`），
//!   内容按 `Maps/**` / `Mods/**` 保留。SCNexus 表达不了的东西逐条记进 [`ExportLoss`]，
//!   **不静默丢**。
//!
//! 导出前先用 [`preview`] 算一遍（不写盘），界面拿 `losses` 警告用户。

mod miyin;
mod patch;
mod scnexus;

#[cfg(test)]
mod tests;

pub use miyin::MIYIN_DIR;
pub use patch::export_patch;

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::library::compose::{self, ComposedFile, Layer};
use crate::library::{Library, Variant};

/// 导出格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// 弥音包：CCM 能读，启动器读回时尽量保真。**推荐**。
    #[default]
    Miyin,
    /// 星际枢纽兼容包：满足 SCNexus 标准，能被星际枢纽直接导入。
    Scnexus,
}

impl ExportFormat {
    /// 解析前端传来的格式名（`miyin` / `scnexus`，大小写不敏感）。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "miyin" => Some(Self::Miyin),
            "scnexus" => Some(Self::Scnexus),
            _ => None,
        }
    }

    /// 面向用户的说法。
    pub fn label(self) -> &'static str {
        match self {
            Self::Miyin => "弥音包（推荐）",
            Self::Scnexus => "星际枢纽兼容包",
        }
    }
}

/// 导出选项。
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// 目标 zip 路径。
    pub destination: PathBuf,
    /// 把启用中的补丁**合并进顶层**（导出物就是"打完补丁的样子"）。
    ///
    /// 无论开不开，弥音包都会把补丁原件放进 `Miyin/patches/`，导入方可以继续单独开关。
    pub merge_patches: bool,
    /// 导出成哪种格式。
    pub format: ExportFormat,
}

/// 一条"带不走"的信息 —— 导出物里没有、但原版本里有的东西。
///
/// 空的 `losses` 表示完整保真；界面要把非空列表**明确展示**给用户，
/// 不许把"少了东西"说成"完整导出"。
#[derive(Debug, Clone, Serialize)]
pub struct ExportLoss {
    /// 机器可读分类（`cover` / `doc` / `patches` / `custom_campaign` …）。
    pub kind: String,
    /// 面向用户的短标题（「封面图」「未合并的补丁」）。
    pub label: String,
    /// 具体是哪一件、为什么带不走。
    pub detail: String,
}

/// 导出结果，界面用来告诉用户"都带了什么"。
#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub path: PathBuf,
    /// 写进 zip 的文件数（含元数据与附加数据）。
    pub files: usize,
    pub maps: usize,
    pub mods: usize,
    /// **目录树形态**的载荷名 —— CCM 大概率读不了，界面要提示。
    pub expanded: Vec<String>,
    /// 一起带走的补丁名。
    pub patches: Vec<String>,
    /// 这次用的格式。
    pub format: ExportFormat,
    /// 星际枢纽包表达不了的内容；弥音包通常为空。
    pub losses: Vec<ExportLoss>,
}

/// 导出前预览：会带什么、会丢什么。**不写盘。**
#[derive(Debug, Clone, Serialize)]
pub struct ExportPreview {
    pub maps: usize,
    pub mods: usize,
    /// 会写进包里的内容文件数（不含元数据）。
    pub files: usize,
    pub losses: Vec<ExportLoss>,
    /// 这个格式能不能完整承载这份内容（`false` 时仍然允许导出，但界面应明确劝阻）。
    pub supported: bool,
}

/// 导出前算一遍：带哪些文件、丢哪些信息。**只读**，preview 与 export 共用。
pub fn preview(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    merge_patches: bool,
    format: ExportFormat,
) -> Result<ExportPreview> {
    let plan = plan(library, slot_slug, variant, merge_patches, format)?;
    Ok(ExportPreview {
        maps: plan.maps(),
        mods: plan.mods(),
        files: plan.files.len(),
        losses: plan.losses.clone(),
        supported: plan.supported,
    })
}

/// 把某个版本导出成指定格式的战役包。
pub fn export(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    options: &ExportOptions,
) -> Result<ExportReport> {
    let plan = plan(
        library,
        slot_slug,
        variant,
        options.merge_patches,
        options.format,
    )?;

    if let Some(parent) = options.destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&options.destination)?;
    let mut zip = zip::ZipWriter::new(file);

    let written = match plan.format {
        ExportFormat::Miyin => miyin::write(&mut zip, library, slot_slug, variant, &plan)?,
        ExportFormat::Scnexus => scnexus::write(&mut zip, variant, &plan)?,
    };

    zip.finish().map_err(zip_error)?;

    Ok(ExportReport {
        path: options.destination.clone(),
        files: written.files,
        maps: plan.maps(),
        mods: plan.mods(),
        expanded: written.expanded,
        patches: plan.patches.clone(),
        format: plan.format,
        losses: plan.losses.clone(),
    })
}

/// 一个计划写进包里的顶层内容文件。
#[derive(Debug, Clone)]
pub(super) struct PlanFile {
    /// 游戏目录内的相对落点（一律正斜杠）。
    target: String,
    /// 库里的来源文件或目录。
    source: PathBuf,
    /// 来源是不是解开的目录树。
    expanded: bool,
    /// 是不是模组（按落点扩展名判断）。
    is_mod: bool,
    /// 弥音包顶层用的平铺文件名（已去重）。
    flat_name: String,
}

/// 两种格式的公共计划。
pub(super) struct Plan {
    format: ExportFormat,
    files: Vec<PlanFile>,
    patches: Vec<String>,
    losses: Vec<ExportLoss>,
    supported: bool,
    miyin: Option<miyin::MiyinPlan>,
    scnexus: Option<scnexus::ScnexusPlan>,
}

impl Plan {
    /// 这次会带多少张地图（按格式真正写进元数据/包里的数量算）。
    pub(super) fn maps(&self) -> usize {
        match self.format {
            ExportFormat::Miyin => self.files.iter().filter(|file| !file.is_mod).count(),
            ExportFormat::Scnexus => self
                .scnexus
                .as_ref()
                .map(|plan| plan.maps().len())
                .unwrap_or_default(),
        }
    }

    /// 这次会带多少个模组。
    pub(super) fn mods(&self) -> usize {
        match self.format {
            ExportFormat::Miyin => self.files.iter().filter(|file| file.is_mod).count(),
            ExportFormat::Scnexus => self
                .scnexus
                .as_ref()
                .map(|plan| plan.mods().len())
                .unwrap_or_default(),
        }
    }
}

/// 写了多少文件、哪些是目录树（两种格式共用的小结果）。
pub(super) struct Written {
    files: usize,
    expanded: Vec<String>,
}

/// 真正算一遍：合成 → 顶层内容 → 格式特有的附加数据与丢失清单。
fn plan(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
    merge_patches: bool,
    format: ExportFormat,
) -> Result<Plan> {
    let kind = crate::library::require_slot(slot_slug)?;
    let sub = variant
        .target_sub
        .clone()
        .or_else(|| kind.sub_directory().map(str::to_string));

    // 导出的是**整份版本内容**：挂载勾选只影响铺盘，不影响包里该带哪些模组。
    // （不然用户取消勾选一个模组再导出，包里就静默少东西。）
    let mut full = variant.clone();
    full.mounted_mods = None;

    let composition = compose::compose(library, slot_slug, &full, sub.as_deref());
    if composition.files.is_empty() {
        return Err(Error::PackageRejected(
            "这个版本里没有可导出的内容".to_string(),
        ));
    }

    // 附加模式：顶层只放战役本体；合成模式：顶层就是合成结果
    let top: Vec<&ComposedFile> = composition
        .files
        .iter()
        .filter(|file| merge_patches || matches!(file.layer, Layer::Campaign))
        .collect();

    if top.is_empty() {
        return Err(Error::PackageRejected(
            "这个版本本体里没有内容，只有补丁".to_string(),
        ));
    }

    let mut used: Vec<String> = vec![
        "metadata.txt".to_string(),
        "metadata.json".to_string(),
        MIYIN_DIR.to_string(),
    ];
    let mut files = Vec::with_capacity(top.len());
    for item in top {
        let raw = item.target.rsplit('/').next().unwrap_or(&item.target);
        let name = crate::campaign::sanitize::sanitize_dir_name(raw)
            .ok_or_else(|| Error::PackageRejected(format!("文件名不可用：{}", item.target)))?;
        let flat_name = unique_name(&mut used, &name);
        files.push(PlanFile {
            target: item.target.clone(),
            source: item.source.clone(),
            expanded: item.expanded,
            is_mod: item.target.to_ascii_lowercase().ends_with(".sc2mod"),
            flat_name,
        });
    }

    let patches: Vec<String> = composition
        .patch_layers()
        .into_iter()
        .filter_map(|layer| match layer {
            Layer::Patch { name, .. } => Some(name),
            Layer::Campaign => None,
        })
        .collect();

    let mut losses = Vec::new();
    let mut supported = true;
    let (miyin_plan, scnexus_plan) = match format {
        ExportFormat::Miyin => {
            let (extra, extra_losses) = miyin::plan(library, slot_slug, variant, &files);
            losses.extend(extra_losses);
            (Some(extra), None)
        }
        ExportFormat::Scnexus => {
            let (extra, extra_losses) = scnexus::plan(
                library,
                &kind,
                slot_slug,
                variant,
                &files,
                merge_patches,
                &patches,
            );
            losses.extend(extra_losses);
            supported = extra.supported;
            (None, Some(extra))
        }
    };

    Ok(Plan {
        format,
        files,
        patches,
        losses,
        supported,
        miyin: miyin_plan,
        scnexus: scnexus_plan,
    })
}

/// 一条丢失记录。
pub(super) fn loss(kind: &str, label: &str, detail: impl Into<String>) -> ExportLoss {
    ExportLoss {
        kind: kind.to_string(),
        label: label.to_string(),
        detail: detail.into(),
    }
}

/// 统一的 zip 写入选项：Deflated，够用。
pub(super) fn zip_options() -> zip::write::SimpleFileOptions {
    zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)
}

/// zip 错误统一包成 IO 错误（领域层不关心 zip 的细分错误类型）。
pub(super) fn zip_error(error: zip::result::ZipError) -> Error {
    Error::Io(std::io::Error::other(error.to_string()))
}

/// 写一个文本条目。
pub(super) fn write_text<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    name: &str,
    content: &str,
) -> Result<()> {
    zip.start_file(name, zip_options()).map_err(zip_error)?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

/// 把一个文件或目录树写进 zip（`name` 是 zip 内的相对路径）。
pub(super) fn add_path<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    source: &Path,
    name: &str,
    files: &mut usize,
) -> Result<()> {
    if source.is_dir() {
        return add_tree(zip, source, name, files);
    }

    zip.start_file(name, zip_options()).map_err(zip_error)?;
    let mut input = std::fs::File::open(source)?;
    std::io::copy(&mut input, zip)?;
    *files += 1;
    Ok(())
}

/// 把一棵目录树写进 zip（保持内部结构）。
pub(super) fn add_tree<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    root: &Path,
    prefix: &str,
    files: &mut usize,
) -> Result<()> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        let name = format!("{prefix}/{}", relative.to_string_lossy().replace('\\', "/"));

        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), zip_options())
                .map_err(zip_error)?;
            continue;
        }

        zip.start_file(&name, zip_options()).map_err(zip_error)?;
        let mut input = std::fs::File::open(entry.path())?;
        std::io::copy(&mut input, zip)?;
        *files += 1;
    }

    Ok(())
}

/// 同名去重：真实包里两张地图撞名的可能性不为零。
pub(super) fn unique_name(used: &mut Vec<String>, name: &str) -> String {
    if !used.iter().any(|item| item.eq_ignore_ascii_case(name)) {
        used.push(name.to_string());
        return name.to_string();
    }

    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem.to_string(), format!(".{extension}")),
        None => (name.to_string(), String::new()),
    };

    for round in 2..1000 {
        let candidate = format!("{stem} ({round}){extension}");
        if !used
            .iter()
            .any(|item| item.eq_ignore_ascii_case(&candidate))
        {
            used.push(candidate.clone());
            return candidate;
        }
    }

    let fallback = format!("{stem}-{}{extension}", crate::library::now_seconds());
    used.push(fallback.clone());
    fallback
}

/// 取路径的文件名（不合法时退回占位名）。
pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string())
}

/// 版本声明的主地图在库里的绝对路径。
pub(super) fn main_map_source(
    library: &Library,
    slot_slug: &str,
    variant: &Variant,
) -> Option<std::path::PathBuf> {
    let declared = variant.main_map.as_deref()?;
    Some(
        library
            .slot_dir(slot_slug)
            .join(&variant.id)
            .join(declared.replace('\\', "/")),
    )
}

/// 在计划里按来源路径找文件（忽略大小写与斜杠方向）。
pub(super) fn find_by_source<'a>(files: &'a [PlanFile], wanted: &Path) -> Option<&'a PlanFile> {
    let wanted = normalize_path(wanted);
    files
        .iter()
        .find(|file| normalize_path(&file.source) == wanted)
}

/// 路径比较用：正斜杠 + 小写。
fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}
