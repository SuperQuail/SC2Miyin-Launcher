//! 星际争霸 II 领域模块。
//!
//! 目前包含：
//!
//! - [`build_info`]：`.build.info` 构建元数据解析
//! - [`discovery`]：安装的发现、校验与路径推导

pub mod build_info;
pub mod discovery;

pub use build_info::BuildInfo;
pub use discovery::{DiscoverySource, Installation, MARKER};
