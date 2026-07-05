//! EntityId ↔ hecs::Entity 无损往返转换
//!
//! `woworld_core::EntityId` 是一个 `u64` wrapper，与 `hecs::Entity` 的 bit 表示兼容。
//! 转换放在 `woworld_ecs` 中——`woworld_core` 不依赖 hecs。
//!
//! hecs 0.10 中 `Entity::from_bits(u64) -> Option<Entity>` 是安全函数。
//! `EntityId(0)` 映射到 `None`（不是有效 hecs Entity）——调用方需处理此情况。
//!
//! ⚠️ Phase 3 接入：当前仅测试使用——LODCoordinator Phase 3 和 NPC System 集成时将
//! 由 WorldDriver 调用。

use woworld_core::types::EntityId;

/// 从 hecs Entity 构造 EntityId（安全——有效 Entity 的 bit 表示总是合法 u64）
pub fn entity_id_from_hecs(entity: hecs::Entity) -> EntityId {
    EntityId(entity.to_bits().get())
}

/// 从 EntityId 恢复 hecs Entity
///
/// 若 EntityId 不对应任何有效 hecs Entity（如 EntityId(0) 为 Player 占位 ID
/// 但尚未 spawn），返回 None。
pub fn entity_id_to_hecs(id: EntityId) -> Option<hecs::Entity> {
    hecs::Entity::from_bits(id.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_roundtrip_via_hecs() {
        let mut world = hecs::World::new();
        let entity = world.spawn((42_u32,));
        let id = entity_id_from_hecs(entity);
        let back = entity_id_to_hecs(id).expect("valid entity");
        assert_eq!(entity, back);
        assert_eq!(id.0, entity.to_bits().get());
    }

    #[test]
    fn test_entity_id_roundtrip_multiple() {
        let mut world = hecs::World::new();
        let e1 = world.spawn((1_u32,));
        let e2 = world.spawn((2_u32,));
        let e3 = world.spawn((3_u32,));

        let id1 = entity_id_from_hecs(e1);
        let id2 = entity_id_from_hecs(e2);
        let id3 = entity_id_from_hecs(e3);

        assert_ne!(id1.0, id2.0);
        assert_ne!(id2.0, id3.0);
        assert_ne!(id1.0, id3.0);

        let back1 = entity_id_to_hecs(id1).expect("valid");
        let back2 = entity_id_to_hecs(id2).expect("valid");
        let back3 = entity_id_to_hecs(id3).expect("valid");

        assert_eq!(e1, back1);
        assert_eq!(e2, back2);
        assert_eq!(e3, back3);
    }

    #[test]
    fn test_entity_id_zero_returns_none() {
        // EntityId(0) 是 Player 占位——尚未 spawn 时 to_hecs 返回 None
        let result = entity_id_to_hecs(EntityId(0));
        assert!(result.is_none());
    }
}
