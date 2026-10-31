//! 扫描真机存档目录的测试。

use std::path::Path;

use super::fixture::write;
use crate::saves::*;

/// 改文件的修改时间（排序测试用）。
fn set_mtime(path: &Path, secs: u64) {
    let file = std::fs::File::options()
        .append(true)
        .open(path)
        .expect("打开");
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        .expect("改时间");
}

#[test]
fn scan_reads_the_real_account_layout() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let docs = tmp.path().join("StarCraft II");
    let account = docs.join("Accounts/12345/美服-12345");

    // Saves：顶层 + Campaign + Unsaved/Campaign 都要收到
    write(&account.join("Saves"), "top.SC2Save", "save");
    write(&account.join("Saves/Campaign"), "campaign.SC2Save", "save");
    write(
        &account.join("Saves/Unsaved/Campaign"),
        "unsaved.SC2Save",
        "save",
    );
    // 干扰项：别的扩展名、别的目录（Hotkeys）
    write(&account.join("Saves"), "notes.txt", "x");
    write(&account.join("Hotkeys"), "hot.SC2Save", "x");
    write(&account.join("Saves/Hotkeys"), "hot2.SC2Save", "x");

    // Banks：按地图 key 分子目录；再深一层仍然算第一层那个 key
    write(&account.join("Banks/地图A"), "a.SC2Bank", "bank");
    write(&account.join("Banks/地图A/备份"), "b.SC2Bank", "bank");
    write(&account.join("Banks"), "root.SC2Bank", "bank");
    write(&account.join("Banks/地图A"), "ignore.txt", "x");

    // 第二个账号：有 Saves 目录但一个存档都没有
    std::fs::create_dir_all(docs.join("Accounts/99999/亚服-99999/Saves")).expect("建");

    // 时间：top 最新，campaign 次之，unsaved 最老
    set_mtime(&account.join("Saves/top.SC2Save"), 1_700_000_300);
    set_mtime(
        &account.join("Saves/Campaign/campaign.SC2Save"),
        1_700_000_200,
    );
    set_mtime(
        &account.join("Saves/Unsaved/Campaign/unsaved.SC2Save"),
        1_700_000_100,
    );

    let inventory = scan(&docs);

    let names: Vec<&str> = inventory
        .saves
        .iter()
        .map(|item| item.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["top", "campaign", "unsaved"],
        "只收 .SC2Save，而且按修改时间降序"
    );
    let folders: Vec<&str> = inventory
        .saves
        .iter()
        .map(|item| item.folder.as_str())
        .collect();
    assert_eq!(
        folders,
        vec!["Saves", "Saves/Campaign", "Saves/Unsaved/Campaign"]
    );
    assert!(inventory.saves.iter().all(|item| item.account == "12345"));
    assert!(
        inventory
            .saves
            .iter()
            .all(|item| item.region == "美服-12345")
    );
    assert!(
        inventory.saves.iter().all(|item| item.bytes > 0),
        "字节数要读出来"
    );

    assert_eq!(inventory.banks.len(), 3, "Hotkeys 和非 .SC2Bank 都不收");
    let map_a = inventory
        .banks
        .iter()
        .filter(|item| item.map == "地图A")
        .count();
    assert_eq!(map_a, 2, "父目录名当地图 key（再深的子目录也算同一张图）");
    assert!(
        inventory.banks.iter().any(|item| item.map.is_empty()),
        "直接在 Banks 根下的 map key 是空串"
    );

    // 两个账号都列出来，最近活跃的在前
    assert_eq!(
        inventory.accounts,
        vec!["12345".to_string(), "99999".to_string()]
    );
}

#[test]
fn scan_without_documents_is_empty_not_a_panic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let inventory = scan(&tmp.path().join("根本没有这个目录"));
    assert!(inventory.saves.is_empty());
    assert!(inventory.banks.is_empty());
    assert!(inventory.accounts.is_empty());
}
