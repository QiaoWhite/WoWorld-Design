//! RegenSystem — 活体生命体征自然恢复
//!
//! 每帧对有 Vitals + RegenState 的 Entity 恢复 HP 和体力。
//! Cap 上限在 max_hp / max_stamina——不会超过最大值。
//!
//! 注意：使用 query_mut（直接修改），不经过 CommandBuffer。
//! 因此需要 `&mut World`——在 process() 中单独调用。

use crate::components::vitals::{RegenState, Vitals};

/// 每帧执行——自然恢复（直接写入 Vitals）
pub fn regen_system(world: &mut hecs::World) {
    for (_entity, (vitals, regen)) in world.query_mut::<(&mut Vitals, &RegenState)>() {
        vitals.hp = (vitals.hp + regen.hp_regen_rate).min(vitals.max_hp);
        vitals.stamina = (vitals.stamina + regen.stamina_regen_rate).min(100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regen_restores_hp() {
        let mut world = hecs::World::new();

        world.spawn((
            Vitals {
                hp: 50.0,
                max_hp: 100.0,
                stamina: 50.0,
                ..Vitals::default()
            },
            RegenState {
                hp_regen_rate: 5.0,
                stamina_regen_rate: 2.0,
            },
        ));

        regen_system(&mut world);

        // 验证 hp 已恢复
        for (_, vitals) in world.query::<&Vitals>().iter() {
            assert!(vitals.hp > 50.0);
            assert!(vitals.stamina > 50.0);
        }
    }

    #[test]
    fn test_regen_caps_at_max() {
        let mut world = hecs::World::new();

        world.spawn((
            Vitals {
                hp: 99.0,
                max_hp: 100.0,
                stamina: 99.0,
                ..Vitals::default()
            },
            RegenState {
                hp_regen_rate: 10.0,
                stamina_regen_rate: 10.0,
            },
        ));

        regen_system(&mut world);

        for (_, vitals) in world.query::<&Vitals>().iter() {
            assert_eq!(vitals.hp, 100.0);
            assert_eq!(vitals.stamina, 100.0);
        }
    }
}
