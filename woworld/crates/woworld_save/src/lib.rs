//! WoWorld 存档系统 — SaveableModule trait + SaveSystem
//!
//! MVP (V6): LMDB 全量快照 → 单 named_db → 重载重建。
//! 不做脏增量/迁移/崩溃恢复/多槽。
//!
//! 参见: `WoWorld-Design/开发阶段/存档系统/` (002 格式 / 003 接口)
//! 参见: `woworld-dev-plan/02-垂直切片/README.md` §4 V6

pub mod header;
pub mod snapshot;
pub mod system;
pub mod trait_def;

// 重导出常用类型
pub use header::SaveHeader;
pub use snapshot::{ClockData, ComponentBag, EntitySnapshot, WorldSnapshot};
pub use system::SaveSystem;
pub use trait_def::{DirtySnapshot, SaveableModule};
