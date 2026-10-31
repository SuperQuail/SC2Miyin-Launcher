//! 真实的存档根：`Accounts/<账号>/<地域>/Banks` 可能有好几个，**整体一起切**。
//!
//! 本机核实（2026-10）：游戏读的是
//! `Documents/StarCraft II/Accounts/<账号>/<地域>/Banks`；
//! `Documents/StarCraft II/Banks` 一般存在但是空的、游戏根本不读 —— 只切那一个目录等于空转。
//!
//! 同一台机器上可能有好几个账号 / 地域，**同一个战役组覆盖所有账号的进度**（不按账号拆组）：
//! 组里按 `Accounts/<账号>/<地域>/Banks/…` 的层级存，切回时按层级原样铺回去。

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// 一个存档文件：相对 `documents` 的路径（`/` 分隔，组里就按它存）、绝对路径、字节数。
pub(super) type FileRef = (String, PathBuf, u64);

/// 一台机器上的全部真实存档根，外加它们的共同上级 `documents`
/// （`Documents/StarCraft II`，组里的相对路径都相对它）。
#[derive(Debug, Clone)]
pub struct SavesRoots {
    documents: PathBuf,
    banks: Vec<PathBuf>,
}

impl SavesRoots {
    /// 老位置：只有 `documents/Banks` 一个目录。组里存的是**相对这个目录**的平铺路径
    /// （旧 API / 单元测试用；既有的组也是这个形状）。
    pub fn single(banks: &Path) -> Self {
        Self {
            documents: banks.to_path_buf(),
            banks: vec![banks.to_path_buf()],
        }
    }

    /// 从用户文档目录（`…/StarCraft II`）扫出所有真实存档根。
    ///
    /// `Accounts/<账号>/<地域>/Banks` **存在的都算**；只有完全没有 `Accounts` 结构
    /// （老机器 / 单元测试）才退回 `<documents>/Banks`。
    ///
    /// 退回的旧目录用 [`SavesRoots::single`]（组里存平铺文件名）—— 和
    /// [`SavesRoots::from_banks`] 落到同一个形状，两个入口写进同一个组的格式才一致。
    ///
    /// `Accounts` 在、但一个 `Banks` 都不存在（还没玩过）时返回空集合 ——
    /// 那是「没地方可动」，不是「退回旧目录」。
    pub fn discover(documents: &Path) -> Self {
        let accounts = documents.join("Accounts");
        let mut banks = Vec::new();
        if accounts.is_dir() {
            for account in std::fs::read_dir(&accounts).into_iter().flatten().flatten() {
                if !account.file_type().is_ok_and(|kind| kind.is_dir()) {
                    continue;
                }
                for region in std::fs::read_dir(account.path())
                    .into_iter()
                    .flatten()
                    .flatten()
                {
                    if !region.file_type().is_ok_and(|kind| kind.is_dir()) {
                        continue;
                    }
                    let dir = region.path().join("Banks");
                    if dir.is_dir() {
                        banks.push(dir);
                    }
                }
            }
        }
        if banks.is_empty() && !accounts.is_dir() {
            return Self::single(&documents.join("Banks"));
        }
        banks.sort();
        Self {
            documents: documents.to_path_buf(),
            banks,
        }
    }

    /// 调用方手上只有老的 `Banks` 路径时用：旁边有 `Accounts/` 就按真实的多根来。
    ///
    /// 旧 API（`snapshot` / `backup` / `switch` …）都走它 —— 包括不能改的
    /// `commands::campaign`，它传的正是 `installation.banks_root`。
    /// 老的平铺组 / 测试目录（旁边没有 `Accounts`）仍是单目录模式，行为不变。
    pub fn from_banks(banks: &Path) -> Self {
        match banks.parent() {
            Some(parent) if parent.join("Accounts").is_dir() => Self::discover(parent),
            _ => Self::single(banks),
        }
    }

    /// 组里的相对路径以它为基准。
    pub fn documents(&self) -> &Path {
        &self.documents
    }

    /// 所有真实 `Banks` 目录（可能一个都不存在：还没玩过）。
    pub fn banks(&self) -> &[PathBuf] {
        &self.banks
    }

    /// 一个根都没有 —— 这台机器上还没有可看 / 可切的地方。
    pub fn is_empty(&self) -> bool {
        self.banks.is_empty()
    }

    /// 所有根下现有的文件；`missing` = 连一个根目录都不存在（不是「空」）。
    ///
    /// 目录不存在不算错误（第一次玩之前它就是不存在的）。
    pub(super) fn files(&self) -> Result<(bool, Vec<FileRef>)> {
        let mut out: Vec<FileRef> = Vec::new();
        let mut found = false;
        for root in &self.banks {
            if !root.is_dir() {
                continue;
            }
            found = true;
            let prefix = self.prefix_of(root);
            for entry in walkdir::WalkDir::new(root)
                .into_iter()
                .map(|item| item.map_err(std::io::Error::from))
            {
                let entry = entry?;
                if !entry.file_type().is_file() {
                    continue;
                }
                let inside = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| Error::PackageRejected("路径越界".to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                let rel = if prefix.is_empty() {
                    inside
                } else {
                    format!("{prefix}/{inside}")
                };
                let bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
                out.push((rel, entry.path().to_path_buf(), bytes));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok((!found, out))
    }

    /// 组里的相对路径落到盘上的哪个位置：按根的前缀还原回原来的账号 / 地域。
    ///
    /// 认不出根（那个账号删掉了 / 组是别的机器带来的）就按 `documents` 的镜像铺；
    /// 老格式的组是**平铺文件名**，放回旧的 `documents/Banks`，别撒进文档根。
    pub(super) fn place(&self, rel: &str) -> PathBuf {
        for root in &self.banks {
            let prefix = self.prefix_of(root);
            if prefix.is_empty() {
                // 老 API：组里就是相对这个目录的路径
                return root.join(rel);
            }
            if let Some(rest) = rel
                .strip_prefix(prefix.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
            {
                return root.join(rest);
            }
        }
        let rel = rel.replace('/', std::path::MAIN_SEPARATOR_STR);
        if rel.contains(std::path::MAIN_SEPARATOR) {
            self.documents.join(rel)
        } else {
            self.documents.join("Banks").join(rel)
        }
    }

    /// 一个根相对 `documents` 的层级（`Accounts/A/CN/Banks`）；单目录模式是空串。
    fn prefix_of(&self, root: &Path) -> String {
        root.strip_prefix(&self.documents)
            .map(|rest| rest.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    }
}
