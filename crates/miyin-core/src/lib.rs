//! 弥音启动器核心库。
//!
//! 本 crate 承担与界面无关的全部领域逻辑：
//!
//! - 星际争霸 II 安装的**发现与校验**（注册表 / 手动指定）
//! - 战役（Custom Campaign）的**扫描、解析、安装、卸载**
//! - 所有写操作的**路径安全校验**
//!
//! **约束**：本 crate 不得依赖任何 GUI 框架，也不得直接与用户交互
//! （见 `AGENTS.md` §4 依赖方向）。

pub mod campaign;
pub mod dev;
pub mod error;
pub mod library;
pub mod platform;
pub mod safety;
pub mod sc2;
pub mod tools;
pub mod update;

pub use error::{Error, Result};
