//! WoWorld Spatial — 空间索引实现
//!
//! 实现 woworld_core 中定义的三大空间 trait：
//! - `GridEntityIndex`: 稀疏网格实体索引
//! - `DdaVisibility`: DDA 射线可见性查询
//! - `RingEventBus`: 环形缓冲区事件总线

pub mod entity_index;
pub mod event_bus;
pub mod visibility;

pub use entity_index::GridEntityIndex;
pub use event_bus::RingEventBus;
pub use visibility::DdaVisibility;
