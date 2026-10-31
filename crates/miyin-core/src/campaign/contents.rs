//! **任意打包形式**的统一读取层。
//!
//! - **zip**：原生读取（快路径，也是绝大多数包的形态）
//! - **其它（7z / rar / tar / cpio / …）**：交给系统自带的解压器解到临时目录，再按目录遍历
//!
//! Windows 10 1803+ 自带 `tar.exe`，那其实是 **bsdtar（libarchive）**，
//! 能读 zip / tar / 7z / **rar5** / cpio 等等 —— 所以"兼容任意打包形式"不需要引入任何新依赖。
//! 找不到 `tar` 时再退回 7-Zip / UnRAR（装了就用）。
//!
//! # 密码
//!
//! 加密的包**不当作"文件坏了"**：缺密码返回 [`Error::PasswordRequired`]，
//! 密码错返回 [`Error::WrongPassword`]，调用方据此去问用户。
//!
//! - zip：原生解密（`zip` crate 带 aes-crypto），条目清单照旧走 `by_index_raw`
//! - 7z / rar：把密码交给 `7z` / `unrar` 的 `-p`
//! - tar（bsdtar）**不认密码**，有密码时直接跳过它
//!
//! # 安全
//!
//! 外部解压器写出来的东西**一律按不可信处理**：条目名照旧要过 [`safe_entry_path`]，
//! 有任何越界就整体拒绝这个包。bsdtar 默认也会拒绝带 `..` 的条目。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

/// 缺密码时给用户看的那句话（各处提示保持一致，别一个地方一个说法）。
const NEED_PASSWORD: &str = "这个压缩包有密码，请输入密码后再导入";

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
    /// 按普通目录遍历。
    Dir {
        /// 先解到临时目录时保住它的生命周期；**直接读一个现成目录时是 `None`**。
        _temp: Option<TempDir>,
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
    /// 读加密条目用的密码（只有 zip 需要）。
    password: Option<String>,
    /// 最小的那个**加密**条目的下标；`None` 表示包里没有加密条目。
    ///
    /// 留一个下标是为了验密码：`by_index_decrypt` 只定位条目、不解压，
    /// 开最小的那条几乎不花时间。
    encrypted: Option<usize>,
}

impl Contents {
    /// 打开一个包；格式由魔数判断，不认识的交给系统解压器试。
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_source(path, None)
    }

    /// 打开一个包，带密码。
    ///
    /// 加密的包缺密码 / 密码错会分别返回 [`Error::PasswordRequired`] / [`Error::WrongPassword`] ——
    /// 调用方据此去要密码，而不是把它当成"文件坏了"。
    pub fn open_with_password(path: &Path, password: Option<&str>) -> Result<Self> {
        Self::open_source(path, password)
    }

    /// 打开一个来源：**目录**直接就着盘读（「从文件夹导入」不复制），文件按格式挑后端。
    pub fn open_source(path: &Path, password: Option<&str>) -> Result<Self> {
        if path.is_dir() {
            return Self::open_dir(path);
        }

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

            let (entries, encrypted) = list_zip_entries(&mut archive);

            // 有加密条目：没密码就去要；给了就先验一次（只开条目，不解压）
            if let Some(index) = encrypted {
                match password {
                    None => return Err(Error::PasswordRequired(NEED_PASSWORD.to_string())),
                    Some(password) => verify_zip_password(&mut archive, index, password)?,
                }
            }

            return Ok(Self {
                backend: Backend::Zip(Box::new(archive)),
                entries,
                password: password.map(str::to_string),
                encrypted,
            });
        }

        // tar（bsdtar / libarchive，Win10 1803+ 自带）能列清单也能单条读 ——
        // 那就别把整个包解出来。它**不认密码**，所以只在没密码时走。
        if password.is_none()
            && let Some((program, entries, names)) = tar_backend(path)
        {
            return Ok(Self {
                backend: Backend::External {
                    program,
                    archive: path.to_path_buf(),
                    names,
                },
                entries,
                password: None,
                encrypted: None,
            });
        }

        // 其它格式：老老实实借系统解压器整体解到临时目录
        let temp = TempDir::new()?;
        unpack_with_system_tool(path, temp.path(), password)?;

        // 条目清单与 files 必须**一一对齐**（目录占位为空路径），
        // 否则按下标读内容会串位。
        let mut collected = Vec::new();
        collect_dir(temp.path(), temp.path(), &mut collected)?;
        collected.sort_by(|left, right| left.0.cmp(&right.0));
        let (files, entries) = split_collected(collected);

        Ok(Self {
            backend: Backend::Dir {
                _temp: Some(temp),
                files,
            },
            entries,
            password: None,
            encrypted: None,
        })
    }

    /// 直接把一个**目录**当成包读：只列清单，不复制、不落盘。
    ///
    /// 「从文件夹导入」和收编游戏目录里已有的战役都走这里。
    pub fn open_dir(dir: &Path) -> Result<Self> {
        if !dir.is_dir() {
            return Err(Error::PackageRejected(format!(
                "找不到文件夹：{}",
                dir.display()
            )));
        }

        let mut collected = Vec::new();
        collect_dir(dir, dir, &mut collected)?;
        collected.sort_by(|left, right| left.0.cmp(&right.0));
        let (files, entries) = split_collected(collected);

        Ok(Self {
            backend: Backend::Dir { _temp: None, files },
            entries,
            password: None,
            encrypted: None,
        })
    }

    /// 条目清单。
    pub fn entries(&self) -> &[ContentEntry] {
        &self.entries
    }

    /// 读出一个条目的全部字节。
    pub fn read(&mut self, index: usize) -> Option<Vec<u8>> {
        let password = self.password.as_deref();
        let encrypted = self.encrypted;

        match &mut self.backend {
            Backend::Zip(archive) => read_zip_entry(archive, index, password, encrypted).ok(),
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

        let password = self.password.as_deref();
        let encrypted = self.encrypted;

        match &mut self.backend {
            Backend::Zip(archive) => {
                let mut file = zip_entry(archive, index, password, encrypted)?;
                let mut out = std::fs::File::create(target)?;
                std::io::copy(&mut file, &mut out)
                    .map_err(|error| decrypt_read_error(password, encrypted, error))?;
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

/// 列 zip 清单，顺带找出**最小的加密条目**（验密码时只开它，不解压）。
///
/// 名字照旧走 `by_index_raw`：加密条目的名字与大小在中央目录里，不需要密码。
fn list_zip_entries(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> (Vec<ContentEntry>, Option<usize>) {
    let mut entries = Vec::with_capacity(archive.len());
    let mut smallest: Option<(usize, u64)> = None;

    for index in 0..archive.len() {
        let Ok(item) = archive.by_index_raw(index) else {
            continue;
        };
        let raw = item.name_raw().to_vec();
        let is_dir = item.is_dir();

        if item.encrypted() && !is_dir && smallest.is_none_or(|(_, size)| item.size() < size) {
            smallest = Some((index, item.size()));
        }

        entries.push(ContentEntry {
            lossy: std::str::from_utf8(&raw).is_err(),
            name: decode_entry_name(&raw),
            size: item.size(),
            is_dir,
        });
    }

    (entries, smallest.map(|(index, _)| index))
}

/// 用一条加密条目验密码。
///
/// AES 会校验 2 字节的验证值、ZipCrypto 校验 1 字节 —— 错密码**基本**当场就能发现
/// （这两种校验都有极小概率放过错密码，所以读的时候还要再兜一道）。
fn verify_zip_password(
    archive: &mut zip::ZipArchive<std::fs::File>,
    index: usize,
    password: &str,
) -> Result<()> {
    match archive.by_index_decrypt(index, password.as_bytes()) {
        Ok(_) => Ok(()),
        Err(zip::result::ZipError::InvalidPassword) => Err(Error::WrongPassword),
        Err(error) => Err(error.into()),
    }
}

/// 打开 zip 里的一个条目；包里有加密条目时用密码解密。
fn zip_entry<'a>(
    archive: &'a mut zip::ZipArchive<std::fs::File>,
    index: usize,
    password: Option<&str>,
    encrypted: Option<usize>,
) -> Result<zip::read::ZipFile<'a, std::fs::File>> {
    match password.filter(|_| encrypted.is_some()) {
        None => archive.by_index(index).map_err(Error::from),
        Some(password) => archive
            .by_index_decrypt(index, password.as_bytes())
            .map_err(|error| match error {
                zip::result::ZipError::InvalidPassword => Error::WrongPassword,
                other => Error::from(other),
            }),
    }
}

/// 读一个条目的全部字节；解密后读失败同样算密码不对。
fn read_zip_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    index: usize,
    password: Option<&str>,
    encrypted: Option<usize>,
) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let mut file = zip_entry(archive, index, password, encrypted)?;
    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)
        .map_err(|error| decrypt_read_error(password, encrypted, error))?;
    Ok(buffer)
}

/// 解密之后才炸的读错误。
///
/// ZipCrypto 允许错误的密码"打开"条目（只有 1/256 的校验位），所以真正失败会
/// 落在解压 / CRC 校验这一步 —— 归到「密码不对」比说「包坏了」更有用。
fn decrypt_read_error(
    password: Option<&str>,
    encrypted: Option<usize>,
    error: std::io::Error,
) -> Error {
    if encrypted.is_some() && password.is_some() {
        Error::WrongPassword
    } else {
        Error::Io(error)
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

/// tar 那条快路：找到 tar 并列出清单。读不了（例如加密包）就返回 `None`，
/// 交给后面的解压器。
fn tar_backend(path: &Path) -> Option<(PathBuf, Vec<ContentEntry>, Vec<Option<String>>)> {
    let program = locate_tar()?;
    let (entries, names) = list_with_tar(&program, path)?;
    Some((program, entries, names))
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

/// 解压器：只认内嵌的 7-Zip。
///
/// 7-Zip 能覆盖 zip / tar / 7z / rar / cpio 和加密包，启动器启动时会把
/// 内嵌的 7z.exe 路径放进 MIYIN_7Z；不再调用系统的 tar / unrar（它们对 7z、
/// 加密头、RAR5 的支持并不一致，而且用户机器上不一定有）。
const UNPACKERS: [Unpacker; 1] = [Unpacker {
    program: "7z",
    arguments: |package, target| {
        let mut output = std::ffi::OsString::from("-o");
        output.push(target);
        vec!["x".into(), "-y".into(), output, package.into()]
    },
}];

/// 解压器程序名 → 实际路径。
///
/// `7z` 允许被启动器内嵌的版本覆盖（环境变量 `MIYIN_7Z`）：这样用户机器上没装
/// 7-Zip 也能解 7z/rar，而不必在 PATH 里塞东西。没设置时仍按原来的 PATH 查找。
fn unpacker_program(program: &str) -> std::path::PathBuf {
    let overridden = (program == "7z")
        .then(|| std::env::var_os("MIYIN_7Z"))
        .flatten()
        .map(std::path::PathBuf::from);
    unpacker_path(program, overridden.as_deref())
}

/// 纯函数版：给定覆盖路径时只接受存在的文件，否则回退到程序名。
fn unpacker_path(program: &str, overridden: Option<&std::path::Path>) -> std::path::PathBuf {
    overridden
        .filter(|path| path.is_file())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(program))
}

/// 用系统解压器把包解开到目标目录。
///
/// 有密码时只交给 7-Zip / UnRAR（`-p`）—— tar **不认密码**，试了只会把
/// "需要密码"说成"解不开"。
///
/// shortcut: 密码是走命令行的，本机其他进程能在进程列表里看到；要更严得改用
/// 解压器的 stdin / 临时加密清单，等真有这类抱怨再说。
fn unpack_with_system_tool(package: &Path, target: &Path, password: Option<&str>) -> Result<()> {
    let mut tried = Vec::new();
    let mut ran = false;
    // 跑过的是**认得密码**的解压器（7z / unrar）—— 决定"要不要问用户密码"
    let mut ran_with_password_support = false;

    for unpacker in &UNPACKERS {
        if password.is_some() && unpacker.program == "tar" {
            continue;
        }

        let mut arguments = (unpacker.arguments)(package, target);
        if let Some(password) = password {
            arguments.push(format!("-p{password}").into());
        }

        let mut command = Command::new(unpacker_program(unpacker.program));
        command.args(&arguments);
        // **别弹控制台窗口** —— 从界面点导入时闪一个黑框很难看
        crate::platform::hide_console(&mut command);

        match command.output() {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                ran = true;
                if unpacker.program != "tar" {
                    ran_with_password_support = true;
                }
                let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
                tried.push(format!("{}（{message}）", unpacker.program));
            }
            Err(_) => tried.push(format!("{}（没装）", unpacker.program)),
        }
    }

    // 解压器跑过却全失败：有密码就是密码不对；没密码而它喊着要密码，就去要密码
    if ran {
        if password.is_some() {
            return Err(Error::WrongPassword);
        }
        if tried.iter().any(|line| asks_for_password(line)) {
            return Err(Error::PasswordRequired(if ran_with_password_support {
                NEED_PASSWORD.to_string()
            } else {
                // 只有 tar 能跑，可它不认密码 —— 直说，别让人对着密码框干瞪眼
                "该格式需要密码，请用 7-Zip 解压后重新导入".to_string()
            }));
        }
    }

    Err(Error::PackageRejected(format!(
        "无法解开这个压缩包（内置 7-Zip 也没解开）。请确认包没坏、密码正确；已尝试：{}",
        tried.join("、")
    )))
}

/// 解压器的输出是不是在说「这个包有密码」。
fn asks_for_password(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("password") || lower.contains("passphrase") || message.contains("密码")
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

/// 把 [`collect_dir`] 的结果拆成「与条目一一对齐的磁盘路径」+ 条目清单。
///
/// 两半必须**一一对齐**（目录的路径占空串），否则按下标读内容会串位。
fn split_collected(collected: Vec<(String, PathBuf)>) -> (Vec<PathBuf>, Vec<ContentEntry>) {
    let files: Vec<PathBuf> = collected.iter().map(|(_, path)| path.clone()).collect();
    let entries = collected
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
    (files, entries)
}

/// zip 条目名的解码：优先 UTF-8，失败退回 GBK。
///
/// （`zip` crate 的 `name()` 在编码不合法时会丢字符，所以这里自己解。）
fn decode_entry_name(raw: &[u8]) -> String {
    crate::campaign::metadata::decode_text(raw)
}

#[cfg(test)]
mod tests;
