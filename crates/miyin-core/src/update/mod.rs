//! 自动更新：从 GitHub Release 检查、下载并替换自身。
//!
//! 分工：
//!
//! - [`version`]：版本号比较（认不出来就当没有更新）
//! - [`mirror`]：GitHub 镜像前缀与 URL 改写
//! - [`net`]：代理自动选择 + **并发竞速**下载
//! - [`check`]：查 Releases、挑最新、校验摘要
//! - [`apply`]：解开更新包、生成替换脚本
//!
//! 设计上的两条底线：
//!
//! 1. **网络问题不算错误**：检查失败会包成 `UpdateCheck::error` 原样返回，
//!    界面显示"检查失败"就行，不该弹一堆红字吓人。
//! 2. **不确定就不动**：版本号认不出来、摘要对不上、包里没有 exe、条目越界 ——
//!    一律拒绝，宁可让用户手动下载。

pub mod apply;
pub mod check;
pub mod mirror;
pub mod net;
pub mod version;

/// 当前程序版本，**对外写法**（`0.1.0a2` 这种）。
///
/// Cargo 只接受 semver，所以 `Cargo.toml` 里写 `0.1.0-alpha.2`；
/// 界面、git tag、发行说明统一用紧凑写法，由 [`version::compact`] 转换 ——
/// 单一事实来源仍然是 `Cargo.toml`。
pub fn current_version() -> String {
    version::compact(env!("CARGO_PKG_VERSION"))
}

/// Cargo 里的原始 semver 版本（调试用）。
pub fn semver_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub use apply::{Staged, apply, stage};
pub use check::{ReleaseAsset, ReleaseInfo, UpdateCheck};
pub use net::{NetworkSettings, ProxyMode};
