//! 战役库的端到端测试：导入 → 启用 → 切换 → 还原。
//!
//! 按域拆成多个文件，共享夹具在 `fixture`。

mod activate;
mod data_root;
mod fixture;
mod import;
mod main_map;
mod merge;
mod mod_import;
mod mods;
mod packages;
mod patch;
mod version;
