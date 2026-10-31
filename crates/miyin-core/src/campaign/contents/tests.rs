//! 读取层的测试：目录来源、加密 zip 的密码错误、tar 清单解析。
//!
//! 拆出来是为了守住「单文件 ≤800 行」（AGENTS §6）—— 逻辑本身还在 upper 模块。

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

/// 直接读一个目录：清单与内容都对，而且**不复制**（路径就在原目录里）。
#[test]
fn opens_a_directory_in_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pack = dir.path().join("MyCampaign");
    std::fs::create_dir_all(pack.join("maps")).expect("mkdir");
    std::fs::write(pack.join("maps/a.SC2Map"), b"hello").expect("write");

    let mut contents = Contents::open_source(&pack, None).expect("打开目录");
    let index = contents
        .entries()
        .iter()
        .position(|entry| entry.name == "maps/a.SC2Map")
        .expect("列得到文件");

    assert_eq!(contents.read(index).as_deref(), Some(b"hello".as_slice()));
    assert!(
        contents
            .entries()
            .iter()
            .any(|entry| entry.name == "maps" && entry.is_dir),
        "目录也要列出来（解压时按它建目录）"
    );
}

/// 加密 zip：没密码 / 密码错都是**类型化的错误**，不是"文件坏了"。
#[test]
fn encrypted_zip_reports_password_errors() {
    use std::io::Write as _;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("locked.zip");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).expect("create"));
    let options =
        zip::write::SimpleFileOptions::default().with_aes_encryption(zip::AesMode::Aes256, "miyin");
    writer.start_file("a.txt", options).expect("start");
    writer.write_all(b"secret").expect("write");
    writer.finish().expect("finish");

    assert!(matches!(
        Contents::open(&path),
        Err(Error::PasswordRequired(_))
    ));
    assert!(matches!(
        Contents::open_with_password(&path, Some("wrong")),
        Err(Error::WrongPassword)
    ));

    let mut opened = Contents::open_with_password(&path, Some("miyin")).expect("密码对了就能开");
    assert_eq!(opened.read(0).as_deref(), Some(b"secret".as_slice()));
}
#[test]
#[cfg(windows)]
fn unpacker_path_prefers_existing_override() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("7z.exe");
    std::fs::write(&fake, b"x").expect("write");
    assert_eq!(unpacker_path("7z", Some(&fake)), fake);
    assert_eq!(
        unpacker_path("7z", Some(&dir.path().join("missing.exe"))),
        std::path::PathBuf::from("7z")
    );
}

#[test]
#[cfg(windows)]
fn bundled_7z_unpacks_a_real_archive() {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().expect("lock");

    let sevenz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src-tauri/assets/7zip/7z.exe")
        .canonicalize()
        .ok()
        .filter(|path| path.is_file());
    // 资产不在（例如只拿到 core 子目录）就跳过，别把测试变成环境检查。
    let Some(sevenz) = sevenz else { return };

    let dir = tempfile::tempdir().expect("tempdir");
    let source = dir.path().join("hello.txt");
    std::fs::write(&source, b"hello 7z").expect("write");
    let archive = dir.path().join("a.7z");
    let packed = std::process::Command::new(&sevenz)
        .current_dir(dir.path())
        .arg("a")
        .arg(&archive)
        .arg(&source)
        .status()
        .expect("run 7z");
    assert!(packed.success(), "7z 打包失败");

    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("mkdir");
    let previous = std::env::var_os("MIYIN_7Z");
    // SAFETY: 测试里串行（上面的 LOCK），且只在本测试内改这个变量。
    unsafe { std::env::set_var("MIYIN_7Z", &sevenz) };
    let result = unpack_with_system_tool(&archive, &out, None);
    unsafe {
        match previous {
            Some(value) => std::env::set_var("MIYIN_7Z", value),
            None => std::env::remove_var("MIYIN_7Z"),
        }
    }
    result.expect("7z 解压");
    assert_eq!(
        std::fs::read(out.join("hello.txt")).expect("read"),
        b"hello 7z"
    );
}
