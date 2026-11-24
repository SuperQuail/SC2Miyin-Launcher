//! 文件被占着时：停下或跳过，但安装清单不能与磁盘分叉。
#![cfg(windows)]

use std::os::windows::fs::OpenOptionsExt;

use super::fixture::*;
use crate::library::install::{Manifest, Owner, Plan, SkipOnBusy};

/// 造一份「两个地图都装好、第二个被独占打开」的现场。
fn setup(fixture: &Fixture) -> (Owner, std::path::PathBuf, std::path::PathBuf, std::fs::File) {
    let data = fixture.library.root();
    let work = fixture.work.path();

    let first = work.join("first.SC2Map");
    let second = work.join("second.SC2Map");
    std::fs::write(&first, b"first").expect("写");
    std::fs::write(&second, b"second").expect("写");

    let owner = Owner::Campaign {
        slot: "wol".to_string(),
        variant: "v1".to_string(),
    };
    let mut plan = Plan::new(owner.clone());
    plan.push(&first, "Maps/Campaign/first.SC2Map");
    plan.push(&second, "Maps/Campaign/second.SC2Map");

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    manifest
        .apply(data, &fixture.installation, &plan)
        .expect("装");

    let locked = fixture
        .installation
        .campaign_maps_root
        .join("second.SC2Map");
    // share_mode(0) = 独占：其它句柄删不掉它，效果与游戏占着文件相同
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked)
        .expect("占住");

    (
        owner,
        fixture.installation.campaign_maps_root.join("first.SC2Map"),
        locked,
        held,
    )
}

#[test]
fn abort_stops_at_the_locked_file_and_keeps_the_unfinished_records() {
    let fixture = fixture();
    let data = fixture.library.root();
    let (owner, _first, locked, held) = setup(&fixture);

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    let error = manifest
        .remove(data, &fixture.installation, &owner)
        .expect_err("占着就删不掉，必须报错");
    assert!(
        matches!(error, crate::error::Error::FileBusy { .. }),
        "必须是可询问用户的「文件被占用」：{error}"
    );

    // 撤掉的那条不留账；没撤成的那条必须留在清单里
    let reloaded = Manifest::load_migrating(data, &fixture.installation);
    assert_eq!(
        reloaded.files.len(),
        1,
        "只剩没撤成的那条：{:?}",
        reloaded.files
    );
    assert_eq!(reloaded.files[0].target, "Maps/Campaign/second.SC2Map");

    drop(held);
    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    manifest
        .remove(data, &fixture.installation, &owner)
        .expect("关掉占用之后应当撤得掉");
    assert!(!locked.exists());
    assert!(Manifest::load(data).files.is_empty(), "清单应当空了");
}

#[test]
fn skip_removes_the_rest_and_keeps_the_locked_entry_for_a_later_retry() {
    let fixture = fixture();
    let data = fixture.library.root();
    let (owner, first, locked, held) = setup(&fixture);

    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    let mut decision = SkipOnBusy;
    let warnings = manifest
        .remove_with(data, &fixture.installation, &owner, &mut decision)
        .expect("选择继续时不该报错");
    assert_eq!(
        warnings.len(),
        1,
        "跳过一个文件应当有一条提示：{warnings:?}"
    );
    assert!(warnings[0].contains("second.SC2Map"), "{warnings:?}");

    // 能删的删了；删不掉的还在盘上，也还在清单里
    assert!(!first.exists(), "没被占用的文件应当已经删除");
    assert!(locked.exists(), "被占用的文件不应被删除");
    let reloaded = Manifest::load_migrating(data, &fixture.installation);
    assert_eq!(reloaded.files.len(), 1);
    assert_eq!(reloaded.files[0].target, "Maps/Campaign/second.SC2Map");

    drop(held);
    let mut manifest = Manifest::load_migrating(data, &fixture.installation);
    manifest
        .remove(data, &fixture.installation, &owner)
        .expect("关掉占用之后应当撤得掉");
    assert!(Manifest::load(data).files.is_empty(), "清单应当空了");
}
