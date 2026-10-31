//! inspect 这一层：造小 zip，看它报什么。
//!
//! 路径推导本身在 paths / payloads 的测试里；这里只测编排起来的结果。

use super::*;
use std::io::Write;
use std::path::PathBuf;

/// issue #36 的复现：声明侧不带 `.SC2Mod`、实际侧带，两边必须认定为同一个模组。
#[test]
fn declared_mod_without_suffix_matches_payload_with_suffix() {
    let payload = |source: &str| Payload {
        source: source.to_string(),
        target: PayloadTarget::Mirror {
            path: source.to_string(),
        },
        expanded: true,
        is_mod: true,
    };
    let payloads = vec![payload("Mods/zeratul-zealot-kit.SC2Mod/Base.SC2Data/x")];

    // 作者写的是去掉后缀的名字 —— 必须认
    assert!(declared_mod_present(&payloads, "zeratul-zealot-kit"));
    // 带前缀 / 带后缀也认
    assert!(declared_mod_present(&payloads, "Mods/zeratul-zealot-kit"));
    // 真的没有的仍然要报
    assert!(!declared_mod_present(&payloads, "another-mod"));
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
/// 用 AES 加密造一个 zip（`zip` crate 默认带 aes-crypto，才有得测）。
fn build_encrypted_zip(dir: &Path, name: &str, password: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .with_aes_encryption(zip::AesMode::Aes256, password);

    for (entry_name, content) in files {
        writer.start_file(*entry_name, options).expect("start file");
        writer.write_all(content).expect("write");
    }
    writer.finish().expect("finish");

    path
}

/// 加密包：**没密码不是错误**，是"请给密码"；密码错换一条提示；对了才正常预检。
#[test]
fn encrypted_zip_asks_for_password_then_inspects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_encrypted_zip(
        dir.path(),
        "locked.zip",
        "miyin",
        &[
            (
                "pack/metadata.txt",
                b"title=Locked\ncampaign=WOL\nversion=1.0\n",
            ),
            ("pack/maps/a.SC2Map", b"stub"),
        ],
    );

    let locked = inspect(&archive).expect("预检本身不该失败");
    assert!(!locked.installable);
    assert!(locked.needs_password, "界面靠它弹密码框");
    assert!(
        locked
            .issues
            .iter()
            .any(|issue| issue.code == "PASSWORD_REQUIRED"),
        "问题：{:?}",
        locked.issues
    );

    let wrong = inspect_with_password(&archive, Some("wrong")).expect("预检本身不该失败");
    assert!(wrong.needs_password);
    assert!(
        wrong
            .issues
            .iter()
            .any(|issue| issue.code == "PASSWORD_WRONG"),
        "问题：{:?}",
        wrong.issues
    );

    let ok = inspect_with_password(&archive, Some("miyin")).expect("预检");
    assert!(ok.installable, "问题：{:?}", ok.issues);
    assert!(!ok.needs_password);
    assert_eq!(ok.name.as_deref(), Some("Locked"));
    assert_eq!(ok.content_root, "pack");
    assert_eq!(ok.map_count, 1);
}

/// 「从文件夹导入」：目录直接就着盘读，不用先解到临时目录。
#[test]
fn inspects_a_directory_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("Folder Pack");
    std::fs::create_dir_all(root.join("maps")).expect("mkdir");
    std::fs::write(
        root.join("metadata.txt"),
        "title=Folder Pack\ncampaign=WOL\n",
    )
    .expect("write");
    std::fs::write(root.join("maps/a.SC2Map"), b"stub").expect("write");

    let inspection = inspect(&root).expect("预检");
    assert!(inspection.installable, "问题：{:?}", inspection.issues);
    assert_eq!(inspection.format, CampaignFormat::Ccm);
    assert_eq!(inspection.name.as_deref(), Some("Folder Pack"));
    assert_eq!(inspection.content_root, "");
    assert_eq!(inspection.map_count, 1);
    // 目录名里带空格也要能当安装目录名（file_stem 会把 "v1.2" 截坏）
    assert_eq!(
        inspection.suggested_dir_name.as_deref(),
        Some("Folder Pack")
    );
}

/// 唯一子文件夹兜底：用户选的那层是**包外面一层**，真正的内容根在唯一的子目录里。
///
/// 这里用子目录里的 `metadata.json` 证明兜底真的生效了 —— 按原有规则，
/// 不在包根的 metadata.json 一律忽略（只报 `NESTED_METADATA_JSON`）。
#[test]
fn falls_back_to_the_only_subfolder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outer = dir.path().join("downloads");
    let pack = outer.join("Void Story");
    std::fs::create_dir_all(&pack).expect("mkdir");
    std::fs::write(
        pack.join("metadata.json"),
        br#"{"name":"Void Story","version":"2.0","campaign":"void"}"#,
    )
    .expect("write");
    std::fs::write(pack.join("a.SC2Map"), b"stub").expect("write");

    let inspection = inspect(&outer).expect("预检");
    assert!(inspection.installable, "问题：{:?}", inspection.issues);
    assert_eq!(inspection.format, CampaignFormat::Standard);
    assert_eq!(inspection.name.as_deref(), Some("Void Story"));
    assert_eq!(inspection.suggested_slot.as_deref(), Some("lotv"));
    // 内容根必须补上那一层，之后 extract_to(outer, 这个值) 才自洽
    assert_eq!(inspection.content_root, "Void Story");
    // 路径仍是用户选的目录（导入时按它去读）
    assert_eq!(inspection.path, outer);
}

/// 兜底只管**一层**、且要求顶层真的只有一个子目录。
#[test]
fn does_not_fall_back_when_there_are_several_subfolders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outer = dir.path().join("downloads");
    std::fs::create_dir_all(outer.join("A")).expect("mkdir");
    std::fs::create_dir_all(outer.join("B")).expect("mkdir");
    std::fs::write(outer.join("A/metadata.json"), br#"{"name":"A"}"#).expect("write");
    std::fs::write(outer.join("B/metadata.json"), br#"{"name":"B"}"#).expect("write");

    let inspection = inspect(&outer).expect("预检");
    assert_eq!(inspection.format, CampaignFormat::Plain, "不该挑一个子目录");
}
