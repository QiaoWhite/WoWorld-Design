//! WoWorld ECS — hecs-based Component definitions and Systems
//!
//! 此 crate 定义所有 ECS Component（纯数据）和 System（纯函数）。
//! Component 不包含游戏逻辑方法——铁律 1。大堆数据不进 Component 内联——铁律 2。
//! 纯数据转换方法（from_/to_）允许，不含副作用或状态修改。
//!
//! 参见: `开发文档/00-ECS哲学与架构总纲/` · `woworld-dev-plan/01-核心基础/1J-ECS基础设施.md`

pub mod components;
pub mod entity_id;
pub mod prng;
pub mod resources;
pub mod systems;

/// 常用 Component 和 System 统一导入
pub mod prelude {
    pub use crate::components::entity_kind::EntityKind;
    pub use crate::components::item::Item;
    pub use crate::components::lod::LodLevel;
    pub use crate::components::transform::{Position, Rotation, Velocity};
    pub use crate::components::vitals::{
        Corpse, CorpseLooted, DeathCategory, DeathCause, DecayingRemains, LootResult,
        PendingDespawn, PendingLoot, RegenState, Vitals,
    };
    pub use crate::entity_id::{entity_id_from_hecs, entity_id_to_hecs};
}
