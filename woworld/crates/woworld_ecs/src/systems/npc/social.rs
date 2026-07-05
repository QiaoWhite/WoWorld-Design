//! 社交 System — Phase 1 O(n²) 近邻社交恢复
//!
//! 每决策周期：配对检查 NPC 距离，在社交半径内则双方 social 需求下降。
//! Phase 2: SpatialGrid 加速替代 O(n²)。

use crate::components::needs::Needs;
use crate::components::social::SocialPresence;
use crate::components::transform::Position;

/// 社交恢复——近邻 NPC 降低彼此社交需求
///
/// 距离越近效果越强：`recovery × (1 - distance / radius)`。
/// 双方同时恢复（双向社交互动）。
pub fn social_system(world: &mut hecs::World, dt: f32) {
    // 收集位置 + 社交存在
    let entries: Vec<(hecs::Entity, Position, SocialPresence)> = world
        .query::<(&Position, &SocialPresence)>()
        .iter()
        .map(|(e, (pos, sp))| (e, *pos, *sp))
        .collect();

    if entries.len() < 2 {
        return;
    }

    // Phase 1: O(n²) 近邻检测（Phase 2 → SpatialGrid）
    let mut modifications: Vec<(hecs::Entity, f32)> = Vec::new();

    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let (e1, p1, sp1) = &entries[i];
            let (e2, p2, sp2) = &entries[j];

            let dist = ((p1.0.x - p2.0.x).powi(2) + (p1.0.z - p2.0.z).powi(2)).sqrt();

            let effective_radius = sp1.radius.min(sp2.radius);
            if dist >= effective_radius {
                continue;
            }

            let proximity = 1.0 - dist / effective_radius;

            // 双方各按对方的 recovery_rate 恢复
            let recovery1 = sp2.recovery_rate * proximity * dt;
            let recovery2 = sp1.recovery_rate * proximity * dt;

            modifications.push((*e1, recovery1));
            modifications.push((*e2, recovery2));
        }
    }

    // 应用恢复（延迟，避免借用冲突）
    for (entity, recovery) in modifications {
        if let Ok(mut needs) = world.get::<&mut Needs>(entity) {
            needs.social = (needs.social - recovery).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use glam::Vec3;

    #[test]
    fn test_close_npcs_recover_social() {
        let mut world = hecs::World::new();
        let sp1 = SocialPresence::from_bigfive(&BigFive {
            extraversion: 0.5,
            ..BigFive::default()
        });
        let sp2 = SocialPresence::from_bigfive(&BigFive {
            extraversion: 0.5,
            ..BigFive::default()
        });

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp1,
            Needs { social: 0.6, ..Needs::default() },
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)), // 1m apart → well within radius (4.5m)
            sp2,
            Needs { social: 0.6, ..Needs::default() },
        ));

        social_system(&mut world, 1.0);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!(needs.social < 0.6, "social should decrease from interaction");
            assert!(needs.social >= 0.0, "social should not go negative");
        }
    }

    #[test]
    fn test_distant_npcs_no_effect() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default(); // radius ~4.5m

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Needs { social: 0.6, ..Needs::default() },
        ));
        world.spawn((
            Position(Vec3::new(20.0, 0.0, 0.0)), // 20m away → outside radius
            sp,
            Needs { social: 0.6, ..Needs::default() },
        ));

        social_system(&mut world, 1.0);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!((needs.social - 0.6).abs() < f32::EPSILON,
                "distant NPCs should not affect each other");
        }
    }

    #[test]
    fn test_social_never_negative() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Needs { social: 0.001, ..Needs::default() }, // near zero
        ));
        world.spawn((
            Position(Vec3::new(0.5, 0.0, 0.0)),
            sp,
            Needs { social: 0.0, ..Needs::default() },
        ));

        social_system(&mut world, 10.0); // long duration

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!(needs.social >= 0.0, "social should not go negative");
        }
    }

    #[test]
    fn test_social_stronger_when_closer() {
        let mut world_far = hecs::World::new();
        let mut world_near = hecs::World::new();
        let sp = SocialPresence::default();

        // 远距对：4m
        world_far.spawn((Position(Vec3::new(0.0, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));
        world_far.spawn((Position(Vec3::new(4.0, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));

        // 近距对：0.5m
        world_near.spawn((Position(Vec3::new(0.0, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));
        world_near.spawn((Position(Vec3::new(0.5, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));

        social_system(&mut world_far, 1.0);
        social_system(&mut world_near, 1.0);

        let far_social: Vec<f32> = world_far.query::<&Needs>().iter().map(|(_, n)| n.social).collect();
        let near_social: Vec<f32> = world_near.query::<&Needs>().iter().map(|(_, n)| n.social).collect();

        let far_avg = (far_social[0] + far_social[1]) / 2.0;
        let near_avg = (near_social[0] + near_social[1]) / 2.0;
        assert!(near_avg < far_avg, "closer NPCs should recover more social");
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        social_system(&mut world, 1.0);
    }

    #[test]
    fn test_single_npc_no_panic() {
        let mut world = hecs::World::new();
        world.spawn((
            Position(Vec3::ZERO),
            SocialPresence::default(),
            Needs::default(),
        ));
        social_system(&mut world, 1.0);
    }
}
