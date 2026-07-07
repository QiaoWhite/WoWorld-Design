//! Power ECS Systems — Phase 1: 种子生成基础权力关系
//!
//! Phase 2+: PowerExerciseSystem, LegitimacyUpdateSystem, PolityDetectionSystem

use hecs::{CommandBuffer, World};
use woworld_core::culture::CultureQuery;
use woworld_core::power::{PowerAtom, PowerDomain, PowerEdge, PowerSource, SuccessionRule};
use woworld_core::types::EntityId;

use crate::components::bigfive::BigFive;
use crate::components::culture::Culture;
use crate::resources::culture_registry::CultureRegistry;
use crate::resources::power_registry::PowerRegistry;

/// 权力种子系统：从 NPC 种子 + BigFive 生成初始个人权力关系
///
/// Phase 1 简化：只创建 Self-Constraint (Pledge) 类型的边。
/// 这使每个 NPC 都有"自我约束"的权力关系基础，后续 Phase 2
/// 从世界生成 P8 管线加载完整权力拓扑。
pub fn power_seed_system(
    world: &World,
    _cmd: &mut CommandBuffer,
    power_registry: &mut PowerRegistry,
    culture_registry: &CultureRegistry,
) {
    for (entity, bigfive) in world.query::<&BigFive>().iter() {
        let seed = entity.to_bits().get();

        // 从文化获取 power_distance 作为自我约束强度
        let power_distance = if let Ok(culture) = world.get::<&Culture>(entity) {
            culture_registry.core_params(culture.culture_id)
                .map(|p| p.power_distance)
                .unwrap_or(0.5)
        } else {
            0.5
        };

        // Pledge: 自我约束——尽责性高→自我约束强
        let self_constraint_strength = (bigfive.conscientiousness * 0.6 + power_distance * 0.4)
            .clamp(0.1, 1.0);

        let entity_id = EntityId(seed);

        let edge = PowerEdge {
            holder: entity_id,
            subject: entity_id, // 自我参照——Pledge 的 subject = holder
            atom: PowerAtom::Pledge,
            source: PowerSource::Emergent,
            domain: PowerDomain::Behavior,
            legitimacy: 0.5, // 自我约束合法性中性
            enforcement: self_constraint_strength,
            established_tick: 0,
            valid_until_tick: None,
            last_exercised_tick: 0,
            succession: SuccessionRule::ExtinguishWithHolder,
            active: true,
        };

        power_registry.create_edge(edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::culture::Culture;
    use woworld_core::culture::CultureCoreParams;
    use woworld_core::power::PowerQuery;

    #[test]
    fn test_seed_system_creates_pledge() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut power_reg = PowerRegistry::new();
        let mut culture_reg = CultureRegistry::new();

        let culture_id = culture_reg.register(CultureCoreParams::default());
        world.spawn((
            BigFive::from_seed(42),
            Culture { culture_id },
        ));

        power_seed_system(&world, &mut cmd, &mut power_reg, &culture_reg);

        // 系统应为每个 NPC 创建一条 Pledge 边
        assert!(power_reg.edge_count() > 0);
    }

    #[test]
    fn test_seed_system_deterministic() {
        let mut w1 = World::new();
        let mut c1 = CommandBuffer::new();
        let mut p1 = PowerRegistry::new();
        let mut cr1 = CultureRegistry::new();

        let mut w2 = World::new();
        let mut c2 = CommandBuffer::new();
        let mut p2 = PowerRegistry::new();
        let mut cr2 = CultureRegistry::new();

        let cid1 = cr1.register(CultureCoreParams::default());
        let cid2 = cr2.register(CultureCoreParams::default());

        w1.spawn((BigFive::from_seed(42), Culture { culture_id: cid1 }));
        w2.spawn((BigFive::from_seed(42), Culture { culture_id: cid2 }));

        power_seed_system(&w1, &mut c1, &mut p1, &cr1);
        power_seed_system(&w2, &mut c2, &mut p2, &cr2);

        // 应该产生相同数量的边
        assert_eq!(p1.edge_count(), p2.edge_count());
    }

    #[test]
    fn test_empty_world_no_panic() {
        let world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut power_reg = PowerRegistry::new();
        let culture_reg = CultureRegistry::new();

        power_seed_system(&world, &mut cmd, &mut power_reg, &culture_reg);
    }
}
