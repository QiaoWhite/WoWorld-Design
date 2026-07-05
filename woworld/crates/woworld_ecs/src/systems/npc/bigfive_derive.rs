//! BigFive 派生 System — 一次性将 BigFive → NeedSensitivity + Chronotype
//!
//! 幂等: 已有 NeedSensitivity/Chronotype 的实体不会被重复派生。
//! 仅在 NPC 创建或 BigFive 极端冲击后调用。

use hecs::CommandBuffer;

use crate::components::bigfive::BigFive;
use crate::components::circadian::Chronotype;
use crate::components::needs::NeedSensitivity;

/// 为所有 (BigFive, !NeedSensitivity) 或 (BigFive, !Chronotype) 实体
/// 一次性派生并插入对应 Component。
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
pub fn bigfive_derive_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    // 派生 NeedSensitivity（只处理缺失的）
    for (entity, bf) in world.query::<&BigFive>()
        .iter()
        .filter(|(e, _)| world.get::<&NeedSensitivity>(*e).is_err())
    {
        cmd.insert_one(entity, bf.derive_sensitivity());
    }

    // 派生 Chronotype（只处理缺失的）
    for (entity, bf) in world.query::<&BigFive>()
        .iter()
        .filter(|(e, _)| world.get::<&Chronotype>(*e).is_err())
    {
        cmd.insert_one(entity, bf.derive_chronotype());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::circadian::Chronotype;
    use crate::components::needs::{NeedSensitivity, Needs};

    #[test]
    fn test_derive_inserts_both() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // Spawn NPC with BigFive but no NeedSensitivity or Chronotype
        let e = world.spawn((BigFive::from_seed(42), Needs::default()));

        bigfive_derive_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // Both should now exist
        assert!(world.get::<&NeedSensitivity>(e).is_ok(), "NeedSensitivity should be inserted");
        assert!(world.get::<&Chronotype>(e).is_ok(), "Chronotype should be inserted");
    }

    #[test]
    fn test_derive_is_idempotent() {
        let mut world = hecs::World::new();

        // Spawn with pre-existing NeedSensitivity
        let pre_sens = NeedSensitivity {
            hunger_sens: 9.99,
            ..NeedSensitivity::default()
        };
        let e = world.spawn((
            BigFive::from_seed(42),
            pre_sens,
            Chronotype::Evening,
            Needs::default(),
        ));

        let mut cmd = CommandBuffer::new();
        bigfive_derive_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // Should keep original values, not overwrite
        let s = world.get::<&NeedSensitivity>(e).unwrap();
        assert!((s.hunger_sens - 9.99).abs() < 0.01, "pre-existing sensitivity should not be overwritten");
        let c = world.get::<&Chronotype>(e).unwrap();
        assert_eq!(*c, Chronotype::Evening, "pre-existing chronotype should not be overwritten");
    }

    #[test]
    fn test_derive_skips_without_bigfive() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // Spawn NPC with Needs but NO BigFive
        let e = world.spawn((Needs::default(),));

        bigfive_derive_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // Nothing should be inserted
        assert!(world.get::<&NeedSensitivity>(e).is_err());
        assert!(world.get::<&Chronotype>(e).is_err());
    }

    #[test]
    fn test_empty_world_no_panic() {
        let world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        bigfive_derive_system(&world, &mut cmd);
    }

    #[test]
    fn test_multiple_npcs_all_derived() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let seeds = [10, 20, 30, 40, 50];
        for &seed in &seeds {
            world.spawn((BigFive::from_seed(seed), Needs::default()));
        }

        bigfive_derive_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // All 5 should have both derived components
        for (_entity, (sens, chrono)) in world.query::<(&NeedSensitivity, &Chronotype)>().iter() {
            // Verify derived values are in valid ranges
            assert!((0.2..=1.0).contains(&sens.hunger_sens));
            assert!(matches!(*chrono, Chronotype::Morning | Chronotype::Neutral | Chronotype::Evening));
        }
    }
}
