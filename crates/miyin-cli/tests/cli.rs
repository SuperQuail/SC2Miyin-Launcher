//! CLI 的对外契约测试：**真把二进制跑起来**，验退出码与 stdout 的 JSON 形状。
//!
//! 进程内的单元测试证明不了这两件事 —— 退出码与"stdout 只有 JSON"恰恰是
//! 别的工具依赖的部分（见 docs/cli.md）。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// 造一个测试 zip（自己写，不用真实战役包 —— 那是游戏版权内容）。
fn build_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    use std::io::Write;

    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("create zip");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    for (entry, content) in files {
        writer.start_file(*entry, options).expect("start file");
        writer.write_all(content).expect("write");
    }
    writer.finish().expect("finish");
    path
}

/// 跑一次 CLI。
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_miyin-cli"))
        .args(args)
        .output()
        .expect("跑得起来 miyin-cli")
}

fn stdout_json(output: &Output) -> serde_json::Value {
    let text = String::from_utf8(output.stdout.clone()).expect("stdout 应当是 UTF-8");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("stdout 不是 JSON：{error}\n{text}"))
}

fn code(output: &Output) -> i32 {
    output
        .status
        .code()
        .expect("应当正常退出（不是被信号杀掉）")
}

/// 好包 -> 退出码 0，且 stdout 是一个包对象（不是数组）。
#[test]
fn inspect_good_package_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "reborn.zip",
        &[
            (
                "Reborn/metadata.txt",
                b"title=Reborn\nauthor=Someone\ncampaign=LOTV\nversion=1.0\n".as_slice(),
            ),
            ("Reborn/maps/paiur01.SC2Map", b"stub".as_slice()),
        ],
    );

    let output = cli(&["inspect", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 0, "好包应当退出 0");
    assert_eq!(report["name"], "Reborn", "字段名与界面同源，不许改");
    assert_eq!(report["installable"], true);
    assert_eq!(report["campaign_type"], "lotv");
    assert_eq!(report["suggested_slot"], "lotv");
    assert_eq!(report["format"], "ccm");
    assert_eq!(report["map_count"], 1);
    assert!(report["issues"].is_array());
}

/// 不存在的路径 -> 退出码 2，且 stdout 干净（不留半截 JSON）。
#[test]
fn inspect_missing_path_exits_two() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nope.zip");

    let output = cli(&["inspect", missing.to_str().expect("utf-8 路径")]);
    assert_eq!(code(&output), 2);
    assert!(
        output.stdout.is_empty(),
        "参数错时 stdout 不该有东西：{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("找不到这个文件"),
        "人话应当走 stderr"
    );
}

/// 存在但读不了（不是压缩包）-> 退出码 1（包坏了），不是 2（参数错）。
#[test]
fn inspect_unreadable_package_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let broken = dir.path().join("broken.zip");
    std::fs::write(&broken, b"not an archive at all").expect("write");

    let output = cli(&["inspect", broken.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 1);
    assert_eq!(report["installable"], false);
    assert_eq!(report["issues"][0]["code"], "NOT_AN_ARCHIVE");
}

/// 多个包 -> JSON 数组；只要有一个 broken，整体退出 1。
#[test]
fn inspect_many_packages_returns_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = build_zip(
        dir.path(),
        "good.zip",
        &[
            ("metadata.txt", b"title=Good\ncampaign=WOL\n".as_slice()),
            ("maps/a.SC2Map", b"stub".as_slice()),
        ],
    );
    let empty = build_zip(
        dir.path(),
        "empty.zip",
        &[("readme.txt", b"nothing".as_slice())],
    );

    let output = cli(&[
        "inspect",
        good.to_str().expect("utf-8 路径"),
        empty.to_str().expect("utf-8 路径"),
    ]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 1, "有一个 broken 就整体退出 1");
    assert!(report.is_array(), "多条时必须是数组");
    let items = report.as_array().expect("数组");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["installable"], true);
    assert_eq!(items[1]["installable"], false);
    // 空包的第一条问题是 NO_METADATA（warning），broken 的那条是 NO_CONTENT —— 别按下标写死
    assert!(
        items[1]["issues"]
            .as_array()
            .expect("issues 是数组")
            .iter()
            .any(|issue| issue["level"] == "broken" && issue["code"] == "NO_CONTENT")
    );
}

/// 声明了归属 -> evidence 是 metadata，且 is_exact 为真。
#[test]
fn identify_declared_campaign() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "nova.zip",
        &[
            (
                "metadata.txt",
                b"title=Nova Pack\ncampaign=NCO\n".as_slice(),
            ),
            ("maps/n01.SC2Map", b"stub".as_slice()),
        ],
    );

    let output = cli(&["identify", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 0);
    assert_eq!(report["campaign_type"], "nova");
    assert_eq!(report["slug"], "nova");
    assert_eq!(report["evidence"], "metadata");
    assert_eq!(report["is_exact"], true);
}

/// 认不出归属 -> 如实说不知道（字段为 null），退出码仍是 0。
#[test]
fn identify_unknown_campaign_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "mystery.zip",
        &[("maps/abc.SC2Map", b"stub".as_slice())],
    );

    let output = cli(&["identify", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 0, "认不出归属不是错误，是结论");
    // 包内没声明 -> 核心给出 Other("")，如实带出来；slug 为 null 才是"认不出来"
    assert!(report["campaign_type"].is_object());
    assert!(report["slug"].is_null());
    assert!(report["evidence"].is_null());
    assert!(report["is_exact"].is_null());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("认不出"),
        "人话走 stderr"
    );
}

/// 普通包（根上没有 Maps/ 或 Mods/）：地图按槽位的子目录展开。
#[test]
fn placements_follow_the_slot_subdirectory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "reborn.zip",
        &[
            ("metadata.txt", b"title=Reborn\ncampaign=LOTV\n".as_slice()),
            ("maps/paiur01.SC2Map", b"stub".as_slice()),
        ],
    );

    let output = cli(&["placements", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 0);
    assert_eq!(report["slot"], "lotv");
    assert_eq!(report["custom"], false);
    assert_eq!(report["base"], "Maps/Campaign");
    assert_eq!(
        report["summary"],
        serde_json::json!({ "payloads": 1, "inside_game": 1, "outside_game": 0 })
    );

    // 包里的 maps/ 只是作者的分类目录（不是游戏的 Maps/），所以只取文件名，
    // 子目录由槽位决定：lotv -> void
    let item = &report["payloads"][0];
    assert_eq!(item["kind"], "map");
    assert_eq!(item["target"], "Maps/Campaign/void/paiur01.SC2Map");
    assert_eq!(item["raw_target"], "paiur01.SC2Map");
}

/// 镜像包（根上有 Mods/）：整包按游戏目录镜像落盘，连大小写一起保留。
///
/// 这条是核心的既定规则（package::mirrors_game_root），不是 CLI 的选择 ——
/// 别的 agent 拿 placements 判断落点时看到的就是这个结果。
#[test]
fn placements_keep_mirror_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "mirror.zip",
        &[
            ("metadata.txt", b"title=Mirror\ncampaign=LOTV\n".as_slice()),
            ("maps/paiur01.SC2Map", b"stub".as_slice()),
            ("Mods/X.SC2Mod", b"stub".as_slice()),
        ],
    );

    let output = cli(&["placements", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 0);
    let payloads = report["payloads"].as_array().expect("数组");
    assert_eq!(payloads.len(), 2);
    let targets: Vec<&str> = payloads
        .iter()
        .map(|item| item["target"].as_str().expect("落点应当是字符串"))
        .collect();
    assert!(targets.contains(&"Maps/maps/paiur01.SC2Map"));
    assert!(targets.contains(&"Mods/X.SC2Mod"));
    assert!(
        payloads.iter().all(|item| item["kind"] == "mirror"),
        "镜像载荷的 kind 应当是 mirror"
    );
}

/// --slot 手动指定优先，且认不出来的 slug 是参数错（退出 2）而不是算一条假路径。
#[test]
fn placements_honour_slot_flag_and_reject_bad_slug() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "mystery.zip",
        &[("maps/abc.SC2Map", b"stub".as_slice())],
    );
    let path = archive.to_str().expect("utf-8 路径");

    // 认不出槽位又没指定 -> 参数错
    let output = cli(&["placements", path]);
    assert_eq!(code(&output), 2);

    // 手动指定自制战役 -> 进 Maps/CustomCampaigns/<目录名>/
    let output = cli(&["placements", path, "--slot", "custom", "--pretty"]);
    let report = stdout_json(&output);
    assert_eq!(code(&output), 0);
    assert_eq!(report["slot"], "custom");
    assert_eq!(report["custom"], true);
    assert!(
        report["base"]
            .as_str()
            .expect("base")
            .starts_with("Maps/CustomCampaigns/")
    );
    assert_eq!(
        report["payloads"][0]["target"],
        "Maps/CustomCampaigns/mystery/abc.SC2Map"
    );

    // 不认识的 slug -> 参数错
    let output = cli(&["placements", path, "--slot", "reborn"]);
    assert_eq!(code(&output), 2);
    assert!(output.stdout.is_empty());
}

/// 默认紧凑、--pretty 缩进；两者是同一份 JSON。
#[test]
fn pretty_flag_only_changes_whitespace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "good.zip",
        &[
            ("metadata.txt", b"title=Good\ncampaign=WOL\n".as_slice()),
            ("maps/a.SC2Map", b"stub".as_slice()),
        ],
    );
    let path = archive.to_str().expect("utf-8 路径");

    let compact = cli(&["inspect", path]);
    let pretty = cli(&["inspect", path, "--pretty"]);

    let compact_text = String::from_utf8(compact.stdout.clone()).expect("utf-8");
    let pretty_text = String::from_utf8(pretty.stdout.clone()).expect("utf-8");
    assert!(!compact_text.contains('\n') || compact_text.lines().count() == 1);
    assert!(pretty_text.lines().count() > 1);
    assert_eq!(stdout_json(&compact), stdout_json(&pretty));
}

/// 目录不是「文件找不到」—— 已解开的库版本要给出能照着做的提示，而不是让人以为路径打错了。
#[test]
fn directory_is_reported_as_directory_not_missing() {
    let dir = tempfile::tempdir().expect("tempdir");

    let output = cli(&["inspect", dir.path().to_str().expect("utf-8 路径")]);
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr 应当是 UTF-8");

    assert_eq!(code(&output), 2, "目录算参数错，不是包坏了");
    assert!(stderr.contains("目录"), "要说清这是个目录：{stderr}");
    assert!(output.stdout.is_empty(), "stdout 不许混进非 JSON");
}

/// 官方改版：两套落点不同（我们的进 Maps/Campaign，枢纽的进 CustomCampaigns），
/// 都认得出来 -> 退出 0；模组两边都进 Mods/。
#[test]
fn compat_official_revision_differs_but_both_recognized() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "reborn.zip",
        &[
            ("metadata.txt", b"title=Reborn\ncampaign=LOTV\n".as_slice()),
            // 包根**不能**出现 Mods/ —— 那会让整包按游戏目录镜像落盘
            // （package::mirrors_game_root），maps/ 就变成 Maps/maps/ 了
            ("paiur01.SC2Map", b"stub".as_slice()),
            ("X.SC2Mod", b"stub".as_slice()),
        ],
    );

    let output = cli(&["compat", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(
        code(&output),
        0,
        "两套都全认 -> 0；stderr：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["slot"], "lotv");
    assert_eq!(report["summary"]["total"], 2);
    assert_eq!(report["summary"]["ours"]["missed"], 0);
    assert_eq!(report["summary"]["nexus"]["missed"], 0);

    let files = report["files"].as_array().expect("files 是数组");
    let map = files
        .iter()
        .find(|file| file["source"] == "paiur01.SC2Map")
        .expect("应当有这张地图");
    assert_eq!(map["ours"]["target"], "Maps/Campaign/void/paiur01.SC2Map");
    assert_eq!(
        map["nexus"]["target"], "Maps/CustomCampaigns/Reborn/paiur01.SC2Map",
        "枢纽那套不看声明，一律当自制战役"
    );
    assert_eq!(map["ours"]["recognized"], true);
    assert_eq!(map["nexus"]["recognized"], true);
    assert_eq!(
        map["ours"]["why"],
        "两套约定的落点不同，但都在游戏扫描的目录里"
    );

    let module = files
        .iter()
        .find(|file| file["source"] == "X.SC2Mod")
        .expect("应当有这个模组");
    assert_eq!(module["ours"]["target"], module["nexus"]["target"]);
    assert_eq!(module["ours"]["why"], "");

    assert!(
        report["summary"]["warnings"]
            .as_array()
            .expect("warnings 是数组")
            .iter()
            .any(|warning| warning
                .as_str()
                .expect("warning 是字符串")
                .contains("枢纽那套不看声明")),
        "官方改版被枢纽当自制战役，必须提一句：{}",
        report["summary"]["warnings"]
    );
}

/// 镜像包把地图摆在游戏扫描不到的目录里 -> 两套都不认，退出 1。
#[test]
fn compat_stray_path_is_missed_and_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "stray.zip",
        &[
            ("metadata.txt", b"title=Stray\ncampaign=WOL\n".as_slice()),
            (
                "Maps/Starcraft Mass Recall/Terran01.SC2Map",
                b"stub".as_slice(),
            ),
        ],
    );

    let output = cli(&["compat", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 1, "有文件不会被认 -> 1");
    let file = &report["files"][0];
    assert_eq!(
        file["ours"]["target"], file["nexus"]["target"],
        "镜像载荷两套落点相同"
    );
    assert_eq!(file["ours"]["recognized"], false);
    assert_eq!(file["nexus"]["recognized"], false);
    assert!(
        file["ours"]["why"]
            .as_str()
            .expect("why 是字符串")
            .contains("不在游戏扫描的目录里"),
        "不认时要给一句 why：{}",
        file["ours"]["why"]
    );
    assert_eq!(report["summary"]["ours"]["missed"], 1);
}

/// 认不出归属：slot 为 null，照样给两套落点，不崩、也不报错。
#[test]
fn compat_unknown_campaign_reports_null_slot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "mystery.zip",
        &[("maps/abc.SC2Map", b"stub".as_slice())],
    );

    let output = cli(&["compat", archive.to_str().expect("utf-8 路径")]);
    let report = stdout_json(&output);

    assert_eq!(code(&output), 0, "落点都在扫描目录里 -> 0");
    assert!(report["slot"].is_null(), "认不出就是 null");
    assert_eq!(report["files"].as_array().expect("files").len(), 1);
    assert!(
        report["summary"]["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning.as_str().expect("字符串").contains("认不出")),
        "要如实提醒认不出：{}",
        report["summary"]["warnings"]
    );
}

/// 坏包（没有载荷）走 broken 那条路：退出 1、stdout 干净 ——
/// 不许报一份"两套都不认"的空报告。
#[test]
fn compat_broken_package_exits_one_without_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "empty.zip",
        &[("readme.txt", b"nothing".as_slice())],
    );

    let output = cli(&["compat", archive.to_str().expect("utf-8 路径")]);

    assert_eq!(code(&output), 1);
    assert!(output.stdout.is_empty(), "坏包不该给一份空报告");
    assert!(String::from_utf8_lossy(&output.stderr).contains("无法运行"));
}

/// compat 只要一个包路径，多给是参数错。
#[test]
fn compat_rejects_extra_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let archive = build_zip(
        dir.path(),
        "a.zip",
        &[("metadata.txt", b"title=A\ncampaign=WOL\n".as_slice())],
    );
    let path = archive.to_str().expect("utf-8 路径");

    let output = cli(&["compat", path, path]);
    assert_eq!(code(&output), 2);
    assert!(output.stdout.is_empty());
}
