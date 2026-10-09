//! **任意打包形式**的统一读取层。
//!
//! - **zip**：原生读取（快路径，也是绝大多数包的形态）
//! - **其它（7z / rar / tar / cpio / …）**：交给系统自带的解压器解到临时目录，再按目录遍历
//!
//! Windows 10 1803+ 自带 `tar.exe`，那其实是 **bsdtar（libarchive）**，
//! 能读 zip / tar / 7z / **rar5** / cpio 等等 —— 所以"兼容任意打包形式"不需要引入任何新依赖。
//! 找不到 `tar` 时再退回 7-Zip / UnRAR（装了就用）。
//!
//! # 安全
//!
//! 外部解压器写出来的东西**一律按不可信处理**：条目名照旧要过 [`safe_entry_path`]，
//! 有任何越界就整体拒绝这个包。bsdtar 默认也会拒绝带 `..` 的条目。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

/// 包里的一个条目。
#[derive(Debug, Clone)]
pub struct ContentEntry {
    /// 包内相对路径（正斜杠）。
    pub name: String,
    /// 解压后的字节数。
    pub size: u64,
    /// 是不是目录。
    pub is_dir: bool,
    /// 名字是不是"非 UTF-8、靠 GBK 兜底解出来的"。
    pub lossy: bool,
}

/// 解压出来的临时目录，离开作用域自动删掉。
struct TempDir(PathBuf);

impl TempDir {
    /// 在系统临时目录下开一个唯一目录。
    fn new() -> Result<Self> {
        let base = std::env::temp_dir().join(format!(
            "miyin-unpack-{}-{}",
            std::process::id(),
            crate::library::unique_suffix()
        ));
        std::fs::create_dir_all(&base)?;
        Ok(Self(base))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 包内容的两种后端。
enum Backend {
    /// 原生 zip。
    Zip(Box<zip::ZipArchive<std::fs::File>>),
    /// 先解到临时目录，再按普通目录遍历。
    Dir {
        /// 保住临时目录的生命周期；离开作用域自动删除。
        _temp: TempDir,
        files: Vec<PathBuf>,
    },
    /// 外部解压器，**按需读**：列清单用 `-t`，读单条用 `-O`。
    ///
    /// 这一层是为了**别「为了列个清单就把上 G 的包全解一遍」** ——
    /// 实测 1.4 GB 的 rar 光开一次就要 11 秒（那就是一次完整解压），
    /// 而一次导入会开三次（预检、导入里的预检、真正解包）→ 33 秒白花。
    /// 现在列清单只要 `tar -tf`，读一个 `DocumentHeader` 只要 `tar -xOf`。
    External {
        /// 用哪个解压器（目前只有 tar 走这条路）。
        program: PathBuf,
        archive: PathBuf,
        /// 与 `entries` 一一对齐的包内路径；目录是 `None`。
        names: Vec<Option<String>>,
    },
}

/// 一个包的内容，屏蔽掉底层打包格式。
pub struct Contents {
    backend: Backend,
    entries: Vec<ContentEntry>,
}

impl Contents {
    /// 打开一个包；格式由魔数判断，不认识的交给系统解压器试。
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Err(Error::PackageRejected(format!(
                "找不到文件：{}",
                path.display()
            )));
        }

        // zip 走原生：既快，也不用把上 G 的内容先落一遍盘
        if is_zip(path) {
            let file = std::fs::File::open(path)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|error| Error::PackageRejected(format!("zip 解析失败：{error}")))?;
            let mut entries = Vec::with_capacity(archive.len());

            for index in 0..archive.len() {
                let Ok(item) = archive.by_index_raw(index) else {
                    continue;
                };
                let raw = item.name_raw().to_vec();
                entries.push(ContentEntry {
                    lossy: std::str::from_utf8(&raw).is_err(),
                    name: decode_entry_name(&raw),
                    size: item.size(),
                    is_dir: item.is_dir(),
                });
            }

            return Ok(Self {
                backend: Backend::Zip(Box::new(archive)),
                entries,
            });
        }

        // tar（bsdtar / libarchive，Win10 1803+ 自带）能列清单也能单条读 ——
        // 那就别把整个包解出来
        if let Some(program) = locate_tar()
            && let Some((entries, names)) = list_with_tar(&program, path)
        {
            return Ok(Self {
                backend: Backend::External {
                    program,
                    archive: path.to_path_buf(),
                    names,
                },
                entries,
            });
        }

        // 其它格式：老老实实借系统解压器整体解到临时目录
        let temp = TempDir::new()?;
        unpack_with_system_tool(path, temp.path())?;

        // 条目清单与 files 必须**一一对齐**（目录占位为空路径），
        // 否则按下标读内容会串位。
        let mut collected = Vec::new();
        collect_dir(temp.path(), temp.path(), &mut collected)?;
        collected.sort_by(|left, right| left.0.cmp(&right.0));

        let files: Vec<PathBuf> = collected.iter().map(|(_, path)| path.clone()).collect();
        let entries: Vec<ContentEntry> = collected
            .into_iter()
            .map(|(name, path)| ContentEntry {
                lossy: false,
                size: if path.as_os_str().is_empty() {
                    0
                } else {
                    std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0)
                },
                is_dir: path.as_os_str().is_empty(),
                name,
            })
            .collect();

        Ok(Self {
            backend: Backend::Dir { _temp: temp, files },
            entries,
        })
    }

    /// 条目清单。
    pub fn entries(&self) -> &[ContentEntry] {
        &self.entries
    }

    /// 读出一个条目的全部字节。
    pub fn read(&mut self, index: usize) -> Option<Vec<u8>> {
        match &mut self.backend {
            Backend::Zip(archive) => {
                use std::io::Read as _;
                let mut file = archive.by_index(index).ok()?;
                let mut buffer = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buffer).ok()?;
                Some(buffer)
            }
            Backend::Dir { files, .. } => {
                let path = files.get(index)?;
                if path.as_os_str().is_empty() {
                    return None;
                }
                std::fs::read(path).ok()
            }
            Backend::External {
                program,
                archive,
                names,
            } => {
                let name = names.get(index)?.as_deref()?;
                read_one_with_tar(program, archive, name)
            }
        }
    }

    /// 把一个条目复制到目标路径（大文件不必先读进内存）。
    pub fn copy_to(&mut self, index: usize, target: &Path) -> Result<()> {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match &mut self.backend {
            Backend::Zip(archive) => {
                let mut file = archive
                    .by_index(index)
                    .map_err(|error| Error::PackageRejected(error.to_string()))?;
                let mut out = std::fs::File::create(target)?;
                std::io::copy(&mut file, &mut out)?;
                Ok(())
            }
            Backend::Dir { files, .. } => {
                let source = files
                    .get(index)
                    .filter(|path| !path.as_os_str().is_empty())
                    .ok_or_else(|| Error::PackageRejected("条目下标越界".to_string()))?;
                std::fs::copy(source, target)?;
                Ok(())
            }
            Backend::External {
                program,
                archive,
                names,
            } => {
                let name = names
                    .get(index)
                    .and_then(|item| item.as_deref())
                    .ok_or_else(|| Error::PackageRejected("条目下标越界".to_string()))?;

                let bytes = read_one_with_tar(program, archive, name)
                    .ok_or_else(|| Error::PackageRejected(format!("读不出包里的 {name}")))?;
                std::fs::write(target, bytes)?;
                Ok(())
            }
        }
    }

    /// 把**整个包**解到目标目录 —— 外部格式专用。
    ///
    /// 返回 `false` 表示这个后端不走整体解（zip 请逐条 `copy_to`）。
    ///
    /// 为什么整体解要单独开一条路：`extract_to` 是逐条目调的，
    /// 外部格式逐条调就是**一个条目起一个进程** —— 164 个条目 164 次 `tar`，
    /// 比整体解一次还慢。所以两种场景各用各的方式：
    /// 列清单 + 读几个小文件 -> 单条读；真要全部落盘 -> 一次 ``tar -xf`。
    pub fn unpack_all(&mut self, destination: &Path, strip: usize) -> Result<bool> {
        let Backend::External {
            program, archive, ..
        } = &self.backend
        else {
            return Ok(false);
        };

        let mut command = Command::new(program);
        command.arg("-xf").arg(archive).arg("-C").arg(destination);
        if strip > 0 {
            command.arg("--strip-components").arg(strip.to_string());
        }
        crate::platform::hide_console(&mut command);

        let output = command
            .output()
            .map_err(|error| Error::PackageRejected(format!("解压失败：{error}")))?;

        if !output.status.success() {
            return Err(Error::PackageRejected(format!(
                "解压失败：{}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        Ok(true)
    }
}

/// 找到可用的 tar。
fn locate_tar() -> Option<PathBuf> {
    // Windows 10 1803+ 自带 System32\tar.exe；其余平台看 PATH
    let system = PathBuf::from(r"C:\Windows\System32\tar.exe");
    if system.is_file() {
        return Some(system);
    }

    let output = Command::new("tar").arg("--version").output().ok()?;
    output.status.success().then(|| PathBuf::from("tar"))
}

/// 用 tar 列一份清单，返回 (条目, 与条目对齐的包内路径)。
fn list_with_tar(
    program: &Path,
    archive: &Path,
) -> Option<(Vec<ContentEntry>, Vec<Option<String>>)> {
    // 名字以 -tf 为准（带空格的路径不会被拆坏），大小只能从 -tvf 里读 ——
    // 少了大小，MAX_UNPACKED_BYTES 那道闸对 tar/7z/rar 就等于不存在：
    // 一个几十 KB、解开来几十 GB 的包会一路铺满用户的盘
    let text = tar_listing(program, archive, "-tf")?;
    let verbose = tar_listing(program, archive, "-tvf")?;
    let mut sizes = verbose.lines().map(verbose_size);

    let mut entries = Vec::new();
    let mut names = Vec::new();

    for line in text.lines() {
        let raw = line.trim_end_matches('\r');
        if raw.is_empty() {
            continue;
        }
        // tar 列出来的目录带尾斜杠
        let is_dir = raw.ends_with('/') || raw.ends_with('\\');
        let name = raw.trim_end_matches(['/', '\\']).replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        entries.push(ContentEntry {
            lossy: false,
            size: sizes.next().unwrap_or(0),
            is_dir,
            name: name.clone(),
        });
        names.push((!is_dir).then_some(name));
    }

    (!entries.is_empty()).then_some((entries, names))
}

/// 跑一次 tar 的列表模式，拿到标准输出。
fn tar_listing(program: &Path, archive: &Path, flag: &str) -> Option<String> {
    let mut command = Command::new(program);
    command.arg(flag).arg(archive);
    crate::platform::hide_console(&mut command);
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 从 `tar -tvf` 的一行里读条目大小。
///
/// bsdtar（Windows 自带的就是它）打印 mode/links/uid/gid/**size**/日期/名字，
/// GNU tar 少两个数字字段。两种都试一下，认不出来算 0 —— 宁可少算，
/// 也不能把正常包判成炸弹。
fn verbose_size(line: &str) -> u64 {
    let fields: Vec<&str> = line.split_whitespace().collect();
    [4usize, 2]
        .iter()
        .find_map(|index| fields.get(*index)?.parse::<u64>().ok())
        .unwrap_or(0)
}

/// 用 tar 把**单个**条目读出来（`-O` 写到标准输出）。
fn read_one_with_tar(program: &Path, archive: &Path, name: &str) -> Option<Vec<u8>> {
    let mut command = Command::new(program);
    command.arg("-xOf").arg(archive).arg(name);
    crate::platform::hide_console(&mut command);
    let output = command.output().ok()?;
    output.status.success().then_some(output.stdout)
}

/// zip 的魔数。
fn is_zip(path: &Path) -> bool {
    use std::io::Read;

    let mut magic = [0u8; 4];
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.read_exact(&mut magic).is_err() {
        return false;
    }

    magic.starts_with(b"PK")
}

/// 解压器的一条候选命令。
struct Unpacker {
    program: &'static str,
    /// 组装命令行参数。
    arguments: fn(&Path, &Path) -> Vec<std::ffi::OsString>,
}

/// 按顺序尝试的解压器。
///
/// `tar` 排第一：Windows 10 1803+ 自带，而且它是 bsdtar（libarchive），
/// 一个工具就能覆盖 zip / tar / 7z / rar / cpio。
const UNPACKERS: [Unpacker; 3] = [
    Unpacker {
        program: "tar",
        arguments: |package, target| vec!["-xf".into(), package.into(), "-C".into(), target.into()],
    },
    Unpacker {
        program: "7z",
        arguments: |package, target| {
            let mut output = std::ffi::OsString::from("-o");
            output.push(target);
            vec!["x".into(), "-y".into(), output, package.into()]
        },
    },
    Unpacker {
        program: "unrar",
        arguments: |package, target| {
            vec!["x".into(), "-y".into(), package.into(), {
                let mut dir = target.as_os_str().to_os_string();
                dir.push("\\");
                dir
            }]
        },
    },
];

/// 用系统解压器把包解开到目标目录。
fn unpack_with_system_tool(package: &Path, target: &Path) -> Result<()> {
    let mut tried = Vec::new();

    for unpacker in &UNPACKERS {
        let arguments = (unpacker.arguments)(package, target);
        let mut command = Command::new(unpacker.program);
        command.args(&arguments);
        // **别弹控制台窗口** —— 从界面点导入时闪一个黑框很难看
        crate::platform::hide_console(&mut command);

        match command.output() {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
                tried.push(format!("{}（{message}）", unpacker.program));
            }
            Err(_) => tried.push(format!("{}（没装）", unpacker.program)),
        }
    }

    Err(Error::PackageRejected(format!(
        "无法解开这个压缩包。请安装 7-Zip，或解压后重新打成 zip。已尝试：{}",
        tried.join("、")
    )))
}

/// 递归收集目录：返回 (包内相对路径, 磁盘路径)。
///
/// 目录的磁盘路径用**空串**占位，保证与条目清单一一对齐。
fn collect_dir(root: &Path, current: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for item in std::fs::read_dir(current)?.flatten() {
        let path = item.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let name = relative.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            out.push((name, PathBuf::new()));
            collect_dir(root, &path, out)?;
        } else {
            out.push((name, path));
        }
    }

    Ok(())
}

/// zip 条目名的解码：优先 UTF-8，失败退回 GBK。
///
/// （`zip` crate 的 `name()` 在编码不合法时会丢字符，所以这里自己解。）
fn decode_entry_name(raw: &[u8]) -> String {
    crate::campaign::metadata::decode_text(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_magic_is_recognised() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("tempdir");
        let zipped = dir.path().join("a.zip");
        std::fs::write(&zipped, b"PK\x03\x04rest").expect("write");
        assert!(is_zip(&zipped));

        let other = dir.path().join("a.7z");
        let mut file = std::fs::File::create(&other).expect("create");
        file.write_all(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C])
            .expect("write");
        assert!(!is_zip(&other));
    }

    #[test]
    fn dir_backend_lists_and_reads() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("Maps/Campaign/void")).expect("mkdir");
        std::fs::write(dir.path().join("Maps/Campaign/void/a.SC2Map"), b"hello").expect("write");

        let mut out = Vec::new();
        collect_dir(dir.path(), dir.path(), &mut out).expect("collect");

        let names: Vec<&str> = out.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"Maps"));
        assert!(names.contains(&"Maps/Campaign/void/a.SC2Map"));

        let (_, path) = out
            .iter()
            .find(|(name, _)| name.ends_with("a.SC2Map"))
            .expect("找到地图");
        assert_eq!(path.file_name().unwrap(), "a.SC2Map");
        assert!(
            out.iter()
                .any(|(name, path)| name == "Maps" && path.as_os_str().is_empty()),
            "目录要用空路径占位，保证与条目清单对齐"
        );
    }

    /// tar -tvf 的大小列。
    ///
    /// 少了它，MAX_UNPACKED_BYTES 那道闸对 tar / 7z / rar 就等于不存在 ——
    /// 实测一个 61 KB 的 .tar.gz 在清单里声称解压后 0 字节。
    #[test]
    fn verbose_listing_gives_sizes() {
        // Windows 自带的 bsdtar：mode links uid gid size 日期 名字
        let bsdtar = "-rw-rw-rw-  0 0      0        1000 10月 09 11:25 a.txt";
        assert_eq!(verbose_size(bsdtar), 1000);

        // GNU tar 少两个数字字段：mode owner/group size 日期 名字
        let gnu = "-rw-r--r-- root/root 332800 2020-01-01 00:00 a.txt";
        assert_eq!(verbose_size(gnu), 332800);

        assert_eq!(
            verbose_size("drwxrwxrwx  0 0      0     0 10月 09 11:25 sub/"),
            0
        );
        // 认不出来算 0：宁可少算，也不能把正常包判成炸弹
        assert_eq!(verbose_size(""), 0);
        assert_eq!(verbose_size("garbage"), 0);
    }
}
