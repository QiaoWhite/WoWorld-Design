//! EntityKind tag Component — 镜像 `woworld_core::types::EntityKind`
//!
//! ECS 侧作为零数据 tag Component 使用。
//! 值类型定义仍在 `woworld_core::types`——此文件仅提供 ECS Component 实现。

/// 实体种类 tag——值枚举与 `woworld_core::types::EntityKind` 同步。
///
/// 作为 Tag Component 使用时，不携带数据——仅标记 Entity 的种类。
/// 需要完整枚举值的场景请使用 `woworld_core::types::EntityKind`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityKind {
    #[default]
    Creature = 0,
    BuildingComponent = 1,
    TerrainFeature = 2,
    DroppedItem = 3,
    Plant = 4,
}

impl EntityKind {
    /// 从 woworld_core 的 EntityKind 构造 tag Component
    pub fn from_core(kind: woworld_core::types::EntityKind) -> Self {
        match kind {
            woworld_core::types::EntityKind::Creature => Self::Creature,
            woworld_core::types::EntityKind::BuildingComponent => Self::BuildingComponent,
            woworld_core::types::EntityKind::TerrainFeature => Self::TerrainFeature,
            woworld_core::types::EntityKind::DroppedItem => Self::DroppedItem,
            woworld_core::types::EntityKind::Plant => Self::Plant,
        }
    }

    /// 转换回 woworld_core 的值类型
    pub fn to_core(self) -> woworld_core::types::EntityKind {
        match self {
            Self::Creature => woworld_core::types::EntityKind::Creature,
            Self::BuildingComponent => woworld_core::types::EntityKind::BuildingComponent,
            Self::TerrainFeature => woworld_core::types::EntityKind::TerrainFeature,
            Self::DroppedItem => woworld_core::types::EntityKind::DroppedItem,
            Self::Plant => woworld_core::types::EntityKind::Plant,
        }
    }
}
