//! Culture ECS Systems
//!
//! Phase 1: culture_seed_system — 为没有 Culture 的 NPC 从种子生成文化参数。
//! Phase 2+: CultureDriftSystem, CultureTransmissionSystem（等世界生成 P2.5 就位）

use hecs::{CommandBuffer, World};
use woworld_core::culture::CultureCoreParams;

use crate::components::bigfive::BigFive;
use crate::components::culture::Culture;
use crate::resources::culture_registry::CultureRegistry;

/// 文化种子系统: 为 NPC 从实体种子确定性分配文化
///
/// 查询所有有 BigFive 但无 Culture 的实体，从其实体 ID
/// 派生确定性 CultureCoreParams，在 CultureRegistry 中注册，
/// 并附加 Culture Component。
///
/// Phase 1: 每个 NPC 创建唯一 CultureId（在 Phase 2 世界生成 P2.5
/// 就位后，NPC 将按区域共享 CultureId——种子路径保留为备选）。
pub fn culture_seed_system(world: &World, cmd: &mut CommandBuffer, registry: &mut CultureRegistry) {
    for (entity, _bigfive) in world.query::<&BigFive>().iter().filter(|(e, _)| {
        // 仅处理无 Culture 的实体
        world.get::<&Culture>(*e).is_err()
    }) {
        // 从实体 ID 比特派生文化种子
        let seed = entity.to_bits().get();
        // 注意: hecs::Entity::to_bits() 返回 u64，包含 generation 和 index。
        // 这对我们的用途足够独特。
        let params = CultureCoreParams::from_seed(seed);
        let culture_id = registry.register(params);
        cmd.insert_one(entity, Culture { culture_id });
    }
}

/// 文化种子系统（带环境标志）
///
/// 供世界生成管线使用——传入 biome 信息以产生 biome-aware 推导。
#[allow(dead_code, clippy::too_many_arguments)]
pub fn culture_seed_system_with_biome(
    world: &World,
    cmd: &mut CommandBuffer,
    registry: &mut CultureRegistry,
    is_arid: bool,
    is_arctic: bool,
    is_forested: bool,
    is_desert: bool, _is_cold: bool, _is_warm: bool,
    is_grassland: bool,
    is_tropical: bool,
) {
    for (entity, _bigfive) in world.query::<&BigFive>().iter().filter(|(e, _)| {
        world.get::<&Culture>(*e).is_err()
    }) {
        let seed = entity.to_bits().get();
        let params = CultureCoreParams::from_seed(seed);
        let culture_id = registry.register_with_biome(
            params, is_arid, is_arctic, is_forested, is_desert, false, false, is_grassland, is_tropical,
        );
        cmd.insert_one(entity, Culture { culture_id });
    }
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::culture::Culture;
    use woworld_core::culture::CultureId;
    use woworld_core::culture::CultureQuery;
    use woworld_core::culture::CULTURE_ID_NONE;

    #[test]
    fn test_seed_system_inserts_culture() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = CultureRegistry::new();

        let entity = world.spawn((BigFive::from_seed(42),));

        culture_seed_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        let culture = world.get::<&Culture>(entity).unwrap();
        assert_ne!(culture.culture_id, CULTURE_ID_NONE);
        assert_eq!(registry.culture_count(), 1);
    }

    #[test]
    fn test_seed_system_deterministic() {
        // 相同实体 ID → 相同文化
        let mut world1 = World::new();
        let mut cmd1 = CommandBuffer::new();
        let mut reg1 = CultureRegistry::new();

        let mut world2 = World::new();
        let mut cmd2 = CommandBuffer::new();
        let mut reg2 = CultureRegistry::new();

        // hecs Entity 在空 World 中顺序分配 → 两个 World 中第一个实体的 ID 相同
        let _e1 = world1.spawn((BigFive::from_seed(42),));
        let _e2 = world2.spawn((BigFive::from_seed(42),));

        culture_seed_system(&world1, &mut cmd1, &mut reg1);
        cmd1.run_on(&mut world1);
        culture_seed_system(&world2, &mut cmd2, &mut reg2);
        cmd2.run_on(&mut world2);

        // 两个 registry 应有相同的参数
        let p1 = reg1.core_params(CultureId(0)).unwrap();
        let p2 = reg2.core_params(CultureId(0)).unwrap();
        for i in 0..CultureCoreParams::DIM_COUNT {
            assert!((p1.dim(i) - p2.dim(i)).abs() < f32::EPSILON,
                "dim {i}: {} vs {}", p1.dim(i), p2.dim(i));
        }
    }

    #[test]
    fn test_seed_system_skips_existing_culture() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = CultureRegistry::new();

        // Pre-register a culture
        let existing_id = registry.register(CultureCoreParams::from_seed(99));

        // NPC with existing Culture
        let e = world.spawn((BigFive::from_seed(42), Culture { culture_id: existing_id }));

        let initial_count = registry.culture_count();

        culture_seed_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        // Registry should NOT have grown (entity already had Culture)
        assert_eq!(registry.culture_count(), initial_count);
        // Culture should NOT have changed
        let culture = world.get::<&Culture>(e).unwrap();
        assert_eq!(culture.culture_id, existing_id);
    }

    #[test]
    fn test_seed_system_empty_world_no_panic() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = CultureRegistry::new();

        culture_seed_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);
        // 空 World——不 panic
    }

    #[test]
    fn test_seed_system_multiple_npcs() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = CultureRegistry::new();

        world.spawn((BigFive::from_seed(0),));
        world.spawn((BigFive::from_seed(1),));
        world.spawn((BigFive::from_seed(2),));

        culture_seed_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        assert_eq!(registry.culture_count(), 3);
    }

    #[test]
    fn test_seed_system_with_biome() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = CultureRegistry::new();

        world.spawn((BigFive::from_seed(42),));

        culture_seed_system_with_biome(
            &world, &mut cmd, &mut registry,
            true, false, true, false, false, true, true, true,
        );
        cmd.run_on(&mut world);

        assert_eq!(registry.culture_count(), 1);
    }
}
