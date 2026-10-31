//! 扫描真机的存档目录（Accounts/<账号>/<地域>/Saves 与 Banks）。

use std::path::Path;

use super::{BankEntry, SaveEntry, SaveInventory};

// ---------------------------------------------------------------------------
// 扫描**真机**的存档目录（Accounts/<账号>/<地域>/Saves 与 Banks）
// ---------------------------------------------------------------------------

/// 递归时跳过的目录名：这些跟存档无关，收进来只会让界面变脏。
const SKIP_DIRS: [&str; 4] = ["Hotkeys", "Cache", "logs", "Logs"];

/// 扫一遍**真机**的存档位置。
///
/// 真机结构（2026-10 核实，scnexus 没做存档管理，没得抄）：
/// ```text
/// <Documents>/StarCraft II/Accounts/<账号>/<地域>/Saves/**/*.SC2Save
/// <Documents>/StarCraft II/Accounts/<账号>/<地域>/Banks/<地图 key>/*.SC2Bank
/// ```
/// 注意：**没有** `<Documents>/StarCraft II/Banks` 这个目录 —— 旧代码读的就是它，
/// 所以真机上永远是 0 个。别再加回去。
///
/// 读不到 / 目录不存在一律当空（不是错误）；`documents` 传 `Installation::documents_root`
/// （即用户文档目录下的 `StarCraft II`）。
pub fn scan(documents: &Path) -> SaveInventory {
    let mut inventory = SaveInventory::default();
    let accounts_root = documents.join("Accounts");
    let Ok(accounts) = std::fs::read_dir(&accounts_root) else {
        return inventory;
    };

    for account in accounts.flatten() {
        if !account.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let account_name = account.file_name().to_string_lossy().into_owned();
        let Ok(regions) = std::fs::read_dir(account.path()) else {
            continue;
        };
        for region in regions.flatten() {
            if !region.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            let region_name = region.file_name().to_string_lossy().into_owned();
            // 只看 Saves 和 Banks：别的目录（Hotkeys 之类）跟存档无关
            let saves_root = region.path().join("Saves");
            let banks_root = region.path().join("Banks");
            if !saves_root.is_dir() && !banks_root.is_dir() {
                continue;
            }
            scan_saves_dir(
                &saves_root,
                &account_name,
                &region_name,
                "Saves",
                &mut inventory.saves,
            );
            scan_banks_dir(
                &banks_root,
                &account_name,
                &region_name,
                "",
                &mut inventory.banks,
            );
            if !inventory.accounts.contains(&account_name) {
                inventory.accounts.push(account_name.clone());
            }
        }
    }

    // 新的在前
    inventory.saves.sort_by(|a, b| b.modified.cmp(&a.modified));
    inventory.banks.sort_by(|a, b| b.modified.cmp(&a.modified));
    // 账号按「它下面最新那份存档」排序：最近玩过的排前面
    let mut scored: Vec<(String, u64)> = inventory
        .accounts
        .iter()
        .map(|account| {
            let newest = inventory
                .saves
                .iter()
                .filter(|item| &item.account == account)
                .map(|item| item.modified)
                .chain(
                    inventory
                        .banks
                        .iter()
                        .filter(|item| &item.account == account)
                        .map(|item| item.modified),
                )
                .max()
                .unwrap_or(0);
            (account.clone(), newest)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    inventory.accounts = scored.into_iter().map(|(account, _)| account).collect();
    inventory
}

/// 递归收 `Saves` 下的 `*.SC2Save`；`folder` 是带前缀的相对路径。
fn scan_saves_dir(dir: &Path, account: &str, region: &str, folder: &str, out: &mut Vec<SaveEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if SKIP_DIRS
                .iter()
                .any(|skip| skip.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            scan_saves_dir(&path, account, region, &format!("{folder}/{name}"), out);
            continue;
        }
        let Some(stem) = save_stem(&path, "SC2Save") else {
            continue;
        };
        out.push(SaveEntry {
            account: account.to_string(),
            region: region.to_string(),
            folder: folder.to_string(),
            name: stem,
            path: path.to_string_lossy().into_owned(),
            bytes: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            modified: modified_secs(&entry),
        });
    }
}

/// 递归收 `Banks` 下的 `*.SC2Bank`；地图 key = **第一层目录名**（父目录）。
fn scan_banks_dir(dir: &Path, account: &str, region: &str, map: &str, out: &mut Vec<BankEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if SKIP_DIRS
                .iter()
                .any(|skip| skip.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            // 第一层子目录就是地图 key；再深也只记这一层
            let next = if map.is_empty() {
                name.clone()
            } else {
                map.to_string()
            };
            scan_banks_dir(&path, account, region, &next, out);
            continue;
        }
        let Some(stem) = save_stem(&path, "SC2Bank") else {
            continue;
        };
        out.push(BankEntry {
            account: account.to_string(),
            region: region.to_string(),
            map: map.to_string(),
            name: stem,
            path: path.to_string_lossy().into_owned(),
            bytes: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            modified: modified_secs(&entry),
        });
    }
}

/// 这个文件是不是 `.<extension>`；是就返回去掉扩展名的文件名。
fn save_stem(path: &Path, extension: &str) -> Option<String> {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
    {
        return None;
    }
    Some(path.file_stem()?.to_string_lossy().into_owned())
}

/// 文件的修改时间（Unix 秒）；取不到就是 0（排序会沉到后面，不影响别的）。
fn modified_secs(entry: &std::fs::DirEntry) -> u64 {
    entry
        .metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
