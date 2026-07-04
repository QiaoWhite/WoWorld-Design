//! WoWorld Core — 核心类型与 trait 定义
//!
//! 最少依赖 crate（仅 glam）。所有跨模块共享的类型、ID 注册表、
//! 空间查询 trait 均在此定义。引擎无关——降低未来迁移成本。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2 阶段二

pub mod density;
pub mod id;
pub mod lod;
pub mod material;
pub mod ocean;
pub mod spatial;
pub mod time;
pub mod types;
pub mod vegetation;
pub mod weather_types;

/// 常用类型统一导入
pub mod prelude {
    pub use crate::id::*;
    pub use crate::lod::*;
    pub use crate::material::*;
    pub use crate::time::*;
    pub use crate::types::*;
    pub use crate::vegetation::*;
    pub use crate::weather_types::*;
}
