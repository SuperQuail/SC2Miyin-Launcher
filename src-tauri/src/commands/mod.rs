//! Tauri 命令层：按领域分文件。
//!
//! 命令名、参数名、返回类型是 IPC 契约（前端 `ui/src/api/`）——改名等于改前端。
//! 所有命令一律 `#[tauri::command(async)]`（见 AGENTS §16），由 `lib.rs` 的
//! `generate_handler!` 注册。

pub(crate) mod campaign;
pub(crate) mod dev;
pub(crate) mod mods;
pub(crate) mod patch;
pub(crate) mod saves;
pub(crate) mod scnexus;
pub(crate) mod settings;
pub(crate) mod ui;
pub(crate) mod window;
