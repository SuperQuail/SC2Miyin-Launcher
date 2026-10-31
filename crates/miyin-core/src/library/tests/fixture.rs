//! 共享夹具：假游戏安装 + 临时库 + 造包工具。
//!
//! 各测试域只从这里拿夹具，不自己造临时目录。

use std::path::{Path, PathBuf};

use crate::library::Library;
use crate::sc2::{DiscoverySource, Installation};

pub(super) const MARKER: &str = "StarCraft II.exe";

/// 用给定条目构造一个 zip。
pub(super) fn build_zip(dir: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    use std::io::Write;

    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    for (entry, content) in files {
        writer.start_file(*entry, options).expect("start file");
        writer.write_all(content.as_bytes()).expect("write");
    }
    writer.finish().expect("finish");

    path
}

/// 一套临时环境：假游戏安装 + 独立的库目录 + 造包用的工作目录。
pub(super) struct Fixture {
    _game: tempfile::TempDir,
    _library_dir: tempfile::TempDir,
    pub(super) work: tempfile::TempDir,
    pub(super) installation: Installation,
    pub(super) library: Library,
}

pub(super) fn fixture() -> Fixture {
    let game = tempfile::tempdir().expect("game dir");
    std::fs::write(game.path().join(MARKER), b"stub").expect("marker");
    std::fs::write(
        game.path().join(".build.info"),
        "Branch!STRING:0|Active!DEC:1|Version!STRING:0\ncn|1|5.0.16.97579\n",
    )
    .expect("build info");

    let installation =
        Installation::from_root(game.path(), DiscoverySource::Manual).expect("valid installation");

    let library_dir = tempfile::tempdir().expect("library dir");

    Fixture {
        installation,
        library: Library::new(library_dir.path().join("data")),
        work: tempfile::tempdir().expect("work dir"),
        _game: game,
        _library_dir: library_dir,
    }
}

/// 造一个 **AES 加密**的战役包（内容与 [`package`] 同形）。
pub(super) fn encrypted_package(
    fixture: &Fixture,
    name: &str,
    password: &str,
    title: &str,
) -> PathBuf {
    use std::io::Write;

    let path = fixture.work.path().join(name);
    let file = std::fs::File::create(&path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .with_aes_encryption(zip::AesMode::Aes256, password);

    let entries = [
        (
            "Reborn/metadata.txt".to_string(),
            format!("title={title}\nauthor=Creator\ncampaign=WOL\nversion=1.0\n"),
        ),
        ("Reborn/maps/01.SC2Map".to_string(), "stub".to_string()),
    ];
    for (entry, content) in entries {
        writer.start_file(entry, options).expect("start file");
        writer.write_all(content.as_bytes()).expect("write");
    }
    writer.finish().expect("finish");

    path
}

/// 造一个 CCM 风格的战役包（metadata.txt 在子目录里 —— 也就是最常见的那种）。
pub(super) fn package(fixture: &Fixture, name: &str, title: &str, maps: &[&str]) -> PathBuf {
    let mut files: Vec<(String, String)> = vec![(
        "Reborn/metadata.txt".to_string(),
        format!("title={title}\nauthor=Creator\ncampaign=WOL\nversion=1.0\n"),
    )];
    for map in maps {
        files.push((format!("Reborn/maps/{map}"), "stub".to_string()));
    }

    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();

    build_zip(fixture.work.path(), name, &borrowed)
}

pub(super) fn slot(fixture: &Fixture, slug: &str) -> crate::library::SlotView {
    fixture
        .library
        .slots(Some(&fixture.installation))
        .into_iter()
        .find(|slot| slot.slug == slug)
        .expect("slot exists")
}
