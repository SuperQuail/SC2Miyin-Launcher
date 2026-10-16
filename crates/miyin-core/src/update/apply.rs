//! 落地更新：下载 -> 解开 -> 生成替换脚本 -> 由调用方退出程序。
//!
//! # 为什么要脚本
//!
//! Windows 上**不能覆盖正在运行的 exe**。所以流程是：把新 exe 先放好，
//! 写一个 `.cmd`，它等主程序退出后重试拷贝、再启动新版本，最后删掉自己。
//! 这是绿色版软件的标准做法，也避免了往系统里装任何东西。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::safety;
use crate::update::Reporter;
use crate::update::check::{self, ReleaseAsset};
use crate::update::net::NetworkSettings;

/// 下载并解开好的更新，等用户点头就能换上去。
#[derive(Debug, Clone, Serialize)]
pub struct Staged {
    /// 新版本号。
    pub version: String,
    /// 更新包本体（留着，失败了还能手动解开）。
    pub archive: PathBuf,
    /// 解开后的目录。
    pub root: PathBuf,
    /// 解出来的新 exe。
    pub executable: PathBuf,
    /// 包里的其它文件（说明文档之类）。
    pub extras: Vec<PathBuf>,
    /// 实际用的下载地址（镜像还是直连）。
    pub url: String,
    pub bytes: u64,
}

/// 更新目录：`<data>/updates`。
pub fn updates_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("updates")
}

/// 下载一个发行包并解开，返回待安装的东西。
///
/// `version` 只用来给目录命名；解压完会**从包里找出真正的 exe**，
/// 因为 release 里那个 exe 叫 `SC2Miyin Launcher.exe`，
/// 而用户机器上可能是任意名字（甚至被改过名），不能写死。
pub fn stage(
    asset: &ReleaseAsset,
    settings: &NetworkSettings,
    data_dir: &Path,
    version: &str,
    reporter: Reporter<'_>,
) -> Result<Staged> {
    let root = updates_dir(data_dir).join(sanitize_version(version));
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    std::fs::create_dir_all(&root)?;

    let archive = root.join("package.zip");
    let outcome = check::download(asset, settings, &archive, reporter)?;

    reporter.say(format!("解开更新包到 {}", root.display()));
    let unpacked = root.join("unpacked");
    std::fs::create_dir_all(&unpacked)?;
    extract_zip(&archive, &unpacked)?;

    let (executable, extras) = find_payload(&unpacked)?;
    reporter.say(format!("准备就绪：{}", executable.display()));
    reporter.say(format!("包内还有 {} 个附带文件会一起更新", extras.len()));

    Ok(Staged {
        version: version.to_string(),
        archive,
        root: root.clone(),
        executable,
        extras,
        url: outcome.url,
        bytes: outcome.bytes,
    })
}

/// 把版本号里的危险字符换掉，当目录名用。
fn sanitize_version(version: &str) -> String {
    let cleaned: String = version
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "unknown".to_string()
    } else {
        cleaned
    }
}

/// 解开 zip（带 zip-slip 防护）。
fn extract_zip(archive: &Path, destination: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|error| Error::PackageRejected(format!("更新包不是有效的 zip：{error}")))?;

    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| Error::PackageRejected(format!("读取更新包失败：{error}")))?;

        let Some(relative) = entry.enclosed_name() else {
            // 条目名越界（含 .. 或绝对路径）—— 整个包不可信
            return Err(Error::PackageRejected(
                "更新包里有一条越界的路径，已拒绝安装".to_string(),
            ));
        };

        let target = safety::ensure_within(destination, &destination.join(relative))?;
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
    }

    Ok(())
}

/// 在解开的目录里找 exe 与其它要一起换掉的文件。
///
/// 只在**顶层**找：更新包的结构是平的（`exe` + `使用说明.txt`），
/// 递归找会把用户自己的东西也扫进来。
fn find_payload(root: &Path) -> Result<(PathBuf, Vec<PathBuf>)> {
    let mut executable: Option<PathBuf> = None;
    let mut extras: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(root)?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_exe = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
        if is_exe && executable.is_none() {
            executable = Some(path);
        } else {
            extras.push(path);
        }
    }

    let executable = executable.ok_or_else(|| {
        Error::PackageRejected("更新包里没有找到 exe，可能下错了文件".to_string())
    })?;

    Ok((executable, extras))
}

/// 生成并启动替换脚本，返回脚本路径。
///
/// 调用方拿到返回值后**应当立刻退出程序** —— 脚本在等我们让出 exe 的文件锁。
pub fn apply(
    staged: &Staged,
    current_exe: &Path,
    data_dir: &Path,
    reporter: Reporter<'_>,
) -> Result<PathBuf> {
    let script = updates_dir(data_dir).join("apply-update.cmd");
    let content = build_script(staged, current_exe)?;
    std::fs::write(&script, content)?;
    reporter.say(format!("已生成替换脚本 {}", script.display()));
    reporter.say("启动器即将退出，由脚本完成替换并重启");

    // 用 cmd /c 起一个独立进程：主程序退出后它继续跑
    let mut command = std::process::Command::new("cmd");
    // 它要弹黑框跑脚本，但那个框跳出来又关掉，观感很差
    crate::platform::hide_console(&mut command);
    command
        .arg("/c")
        .arg(&script)
        .current_dir(updates_dir(data_dir))
        .spawn()
        .map_err(|error| Error::Io(std::io::Error::other(format!("启动替换脚本失败：{error}"))))?;

    Ok(script)
}

/// 生成替换脚本的内容。
fn build_script(staged: &Staged, current_exe: &Path) -> Result<String> {
    let install_dir = current_exe
        .parent()
        .ok_or_else(|| Error::PackageRejected("拿不到程序所在目录".to_string()))?;

    let mut lines = vec![
        "@echo off".to_string(),
        "chcp 65001 >nul".to_string(),
        "rem 由 SC2Miyin Launcher 的自动更新生成；跑完会把自己删掉".to_string(),
        String::new(),
        format!("set \"SRC={}\"", staged.executable.display()),
        format!("set \"DST={}\"", current_exe.display()),
        String::new(),
        "rem 等主程序把 exe 的文件锁放掉；最多重试 30 次".to_string(),
        "set /a tries=0".to_string(),
        ":retry".to_string(),
        "copy /y \"%SRC%\" \"%DST%\" >nul 2>&1".to_string(),
        "if not errorlevel 1 goto copied".to_string(),
        "set /a tries+=1".to_string(),
        "if %tries% geq 30 goto failed".to_string(),
        "timeout /t 1 /nobreak >nul".to_string(),
        "goto retry".to_string(),
        String::new(),
        ":copied".to_string(),
    ];

    // 顺带把说明文档也换成新的（用户自己放的东西不碰）
    for extra in &staged.extras {
        if let Some(name) = extra.file_name() {
            lines.push(format!(
                "copy /y \"{}\" \"{}\" >nul 2>&1",
                extra.display(),
                install_dir.join(name).display()
            ));
        }
    }

    lines.extend([
        format!("start \"\" \"{}\"", current_exe.display()),
        ":failed".to_string(),
        "rem 清理：更新包与脚本自己".to_string(),
        format!("rmdir /s /q \"{}\" >nul 2>&1", staged.root.display()),
        "del \"%~f0\"".to_string(),
        String::new(),
    ]);

    Ok(lines.join("\r\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_version_for_directory_names() {
        assert_eq!(sanitize_version("0.2.0-alpha.1"), "0.2.0-alpha.1");
        assert_eq!(sanitize_version("v0.2.0/../evil"), "v0.2.0_.._evil");
        assert_eq!(sanitize_version(""), "unknown");
    }

    #[test]
    fn script_waits_retries_and_restarts() {
        let staged = Staged {
            version: "0.2.0".to_string(),
            archive: PathBuf::from(r"C:\data\updates\0.2.0\package.zip"),
            root: PathBuf::from(r"C:\data\updates\0.2.0"),
            executable: PathBuf::from(r"C:\data\updates\0.2.0\unpacked\SC2Miyin Launcher.exe"),
            extras: vec![PathBuf::from(
                r"C:\data\updates\0.2.0\unpacked\使用说明.txt",
            )],
            url: "https://github.com/x".to_string(),
            bytes: 1024,
        };

        let script = build_script(&staged, Path::new(r"C:\Game\Launcher\miyin-launcher.exe"))
            .expect("应当能生成");

        // 必须重试——主程序退出前 exe 是锁着的
        assert!(script.contains(":retry"));
        assert!(script.contains("copy /y"));
        assert!(script.contains("goto retry"));
        // 换完要自己启动回来
        assert!(script.contains("start \"\""));
        assert!(script.contains(r"C:\Game\Launcher\miyin-launcher.exe"));
        // 说明文档一起换
        assert!(script.contains("使用说明.txt"));
        // 收尾清理
        assert!(script.contains("del \"%~f0\""));
    }

    #[test]
    fn payload_search_stays_on_the_top_level() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("SC2Miyin Launcher.exe"), b"MZ").expect("write");
        std::fs::write(dir.path().join("使用说明.txt"), b"hi").expect("write");
        std::fs::create_dir_all(dir.path().join("data/campaigns")).expect("mkdir");
        std::fs::write(dir.path().join("data/campaigns/x.txt"), b"x").expect("write");

        let (exe, extras) = find_payload(dir.path()).expect("应当找到");
        assert!(exe.ends_with("SC2Miyin Launcher.exe"));
        assert_eq!(extras.len(), 1, "data/ 里的东西不该被当成要替换的负载");
        assert!(extras[0].ends_with("使用说明.txt"));
    }

    #[test]
    fn missing_exe_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("readme.txt"), b"hi").expect("write");
        assert!(find_payload(dir.path()).is_err());
    }

    #[test]
    fn rejects_zip_slip() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("tempdir");
        let archive = dir.path().join("evil.zip");
        {
            let file = std::fs::File::create(&archive).expect("create");
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("../escaped.txt", options).expect("start");
            zip.write_all(b"boom").expect("write");
            zip.finish().expect("finish");
        }

        let out = dir.path().join("out");
        std::fs::create_dir_all(&out).expect("mkdir");
        assert!(extract_zip(&archive, &out).is_err(), "越界条目必须整体拒绝");
        assert!(!dir.path().join("escaped.txt").exists());
    }
}
