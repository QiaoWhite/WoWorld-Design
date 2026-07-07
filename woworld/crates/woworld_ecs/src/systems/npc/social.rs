//! 社交 System — Phase 2 情绪传染 + 关系构建 + 社交需求恢复
//!
//! 每决策周期：配对检查 NPC 距离，在社交半径内：
//! 1. 社交需求恢复（Phase 1 遗留）
//! 2. 情绪传染——PAD 向近邻漂移
//! 3. 关系构建——familiarity 累积 + liking 从情绪兼容性调整
//!
//! Phase 3: SpatialGrid 加速替代 O(n²)。

use crate::components::emotion::Emotion;
use crate::components::needs::Needs;
use crate::components::social::{Relationships, SocialPresence};
use crate::components::transform::Position;
use woworld_core::types::EntityId;

/// 情绪传染基础速率——每秒向近邻 PAD 漂移的比例
const CONTAGION_BASE_RATE: f32 = 0.05;
/// 关系构建基础速率——每秒 familiarity 增量
const FAMILIARITY_GAIN_RATE: f32 = 0.02;
/// liking 调整速率——每秒从情绪兼容性调整 liking 的比例
const LIKING_ADJUST_RATE: f32 = 0.01;

/// 社交系统——近邻 NPC 互动
///
/// `current_tick`: 用于标记关系最近互动时间（来自 WorldDriver.frame_count）
pub fn social_system(world: &mut hecs::World, dt: f32, current_tick: u64) {
    // 收集位置 + 社交存在 + Entity ID
    let entries: Vec<(hecs::Entity, Position, SocialPresence)> = world
        .query::<(&Position, &SocialPresence)>()
        .iter()
        .map(|(e, (pos, sp))| (e, *pos, *sp))
        .collect();

    if entries.len() < 2 {
        return;
    }

    // Phase 1: O(n²) 近邻检测（Phase 3 → SpatialGrid）
    // 延迟写入，避免借用冲突
    struct Modification {
        entity: hecs::Entity,
        social_recovery: f32,
        pleasure_delta: f32,
        arousal_delta: f32,
        control_delta: f32,
        relationship_target: Option<(EntityId, f32, f32, f32)>, // (target, fam_gain, affection_delta, trust_gain)
    }

    let mut modifications: Vec<Modification> = Vec::new();

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

            // ── 社交需求恢复（Phase 1 遗留）──
            let social_rec1 = sp2.recovery_rate * proximity * dt;
            let social_rec2 = sp1.recovery_rate * proximity * dt;

            // ── 情绪传染 ──
            // 查双方情绪——没 Emotion 组件的实体跳过传染
            let emot1 = world.get::<&Emotion>(*e1).ok().map(|r| *r);
            let emot2 = world.get::<&Emotion>(*e2).ok().map(|r| *r);

            let (pd1, ad1, cd1, pd2, ad2, cd2) = match (emot1, emot2) {
                (Some(em1), Some(em2)) => {
                    let contagion = CONTAGION_BASE_RATE * proximity * dt;
                    (
                        (em2.pleasure - em1.pleasure) * contagion,
                        (em2.arousal - em1.arousal) * contagion,
                        (em2.control - em1.control) * contagion,
                        (em1.pleasure - em2.pleasure) * contagion,
                        (em1.arousal - em2.arousal) * contagion,
                        (em1.control - em2.control) * contagion,
                    )
                }
                _ => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            };

            // ── 关系构建 ──
            // 查双方关系记忆
            let rel1 = world.get::<&Relationships>(*e1).ok().map(|r| *r);
            let rel2 = world.get::<&Relationships>(*e2).ok().map(|r| *r);

            let id1 = EntityId(e1.to_bits().get());
            let id2 = EntityId(e2.to_bits().get());

            let (rel_target1, rel_target2) = match (rel1, rel2) {
                (Some(_), Some(_)) => {
                    let fam_gain = FAMILIARITY_GAIN_RATE * proximity * dt;

                    // affection 从情绪兼容性调整：相似 → +affection, 相反 → -affection
                    // 文档 §2.4: affection 是短期波动，受最近事件影响
                    let compat = match (emot1, emot2) {
                        (Some(em1), Some(em2)) => {
                            1.0 - (em1.pleasure - em2.pleasure).abs()
                        }
                        _ => 0.5, // 没有情绪数据 → 中性兼容性
                    };
                    let affection_delta = (compat - 0.5) * LIKING_ADJUST_RATE * proximity * dt;

                    // trust 缓慢累积（文档: 长期积累，如果未被负面事件破坏，时间加深信任）
                    let trust_gain = 0.001 * proximity * dt;

                    (
                        Some((id2, fam_gain, affection_delta, trust_gain)),
                        Some((id1, fam_gain, affection_delta, trust_gain)),
                    )
                }
                _ => (None, None),
            };

            modifications.push(Modification {
                entity: *e1,
                social_recovery: social_rec1,
                pleasure_delta: pd1,
                arousal_delta: ad1,
                control_delta: cd1,
                relationship_target: rel_target1,
            });
            modifications.push(Modification {
                entity: *e2,
                social_recovery: social_rec2,
                pleasure_delta: pd2,
                arousal_delta: ad2,
                control_delta: cd2,
                relationship_target: rel_target2,
            });
        }
    }

    // ── 应用修改 ──
    for m in &modifications {
        // 社交需求恢复
        if m.social_recovery > 0.0 {
            if let Ok(mut needs) = world.get::<&mut Needs>(m.entity) {
                needs.social = (needs.social - m.social_recovery).max(0.0);
            }
        }

        // 情绪传染
        if m.pleasure_delta != 0.0 || m.arousal_delta != 0.0 || m.control_delta != 0.0 {
            if let Ok(mut emot) = world.get::<&mut Emotion>(m.entity) {
                emot.pleasure = (emot.pleasure + m.pleasure_delta).clamp(-1.0, 1.0);
                emot.arousal = (emot.arousal + m.arousal_delta).clamp(0.0, 1.0);
                emot.control = (emot.control + m.control_delta).clamp(-1.0, 1.0);
            }
        }

        // 关系构建
        if let Some((target_id, fam_gain, affection_delta, trust_gain)) = m.relationship_target {
            if let Ok(mut rels) = world.get::<&mut Relationships>(m.entity) {
                let entry = rels.upsert(target_id, current_tick);
                entry.familiarity = (entry.familiarity + fam_gain).min(1.0);
                entry.affection = (entry.affection + affection_delta).clamp(-1.0, 1.0);
                entry.trust = (entry.trust + trust_gain).clamp(-1.0, 1.0);
                entry.total_interactions = entry.total_interactions.saturating_add(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use glam::Vec3;

    // ── Phase 1: 社交需求恢复 ──

    #[test]
    fn test_close_npcs_recover_social() {
        let mut world = hecs::World::new();
        let sp1 = SocialPresence::derive_from_bigfive(&BigFive {
            extraversion: 0.5,
            ..BigFive::default()
        });
        let sp2 = SocialPresence::derive_from_bigfive(&BigFive {
            extraversion: 0.5,
            ..BigFive::default()
        });

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp1,
            Needs { social: 0.6, ..Needs::default() },
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp2,
            Needs { social: 0.6, ..Needs::default() },
        ));

        social_system(&mut world, 1.0, 0);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!(needs.social < 0.6, "social should decrease from interaction");
            assert!(needs.social >= 0.0, "social should not go negative");
        }
    }

    #[test]
    fn test_distant_npcs_no_effect() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Needs { social: 0.6, ..Needs::default() },
        ));
        world.spawn((
            Position(Vec3::new(20.0, 0.0, 0.0)),
            sp,
            Needs { social: 0.6, ..Needs::default() },
        ));

        social_system(&mut world, 1.0, 0);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!((needs.social - 0.6).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_social_never_negative() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Needs { social: 0.001, ..Needs::default() },
        ));
        world.spawn((
            Position(Vec3::new(0.5, 0.0, 0.0)),
            sp,
            Needs { social: 0.0, ..Needs::default() },
        ));

        social_system(&mut world, 10.0, 0);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!(needs.social >= 0.0);
        }
    }

    #[test]
    fn test_social_stronger_when_closer() {
        let mut world_far = hecs::World::new();
        let mut world_near = hecs::World::new();
        let sp = SocialPresence::default();

        world_far.spawn((Position(Vec3::new(0.0, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));
        world_far.spawn((Position(Vec3::new(4.0, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));

        world_near.spawn((Position(Vec3::new(0.0, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));
        world_near.spawn((Position(Vec3::new(0.5, 0.0, 0.0)), sp,
            Needs { social: 0.5, ..Needs::default() }));

        social_system(&mut world_far, 1.0, 0);
        social_system(&mut world_near, 1.0, 0);

        let far_social: Vec<f32> = world_far.query::<&Needs>().iter().map(|(_, n)| n.social).collect();
        let near_social: Vec<f32> = world_near.query::<&Needs>().iter().map(|(_, n)| n.social).collect();
        assert!((near_social[0] + near_social[1]) < (far_social[0] + far_social[1]));
    }

    // ── Phase 2: 情绪传染 ──

    #[test]
    fn test_emotional_contagion_pleasure() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        let e1 = world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Emotion { pleasure: -0.5, arousal: 0.5, control: 0.0 },
        ));
        let e2 = world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            Emotion { pleasure: 0.5, arousal: 0.5, control: 0.0 },
        ));

        social_system(&mut world, 10.0, 0);

        // e1（负面）应向 e2（正面）漂移
        let emot1 = world.get::<&Emotion>(e1).unwrap();
        assert!(emot1.pleasure > -0.5, "negative NPC should drift toward positive");

        // e2（正面）应向 e1（负面）漂移
        let emot2 = world.get::<&Emotion>(e2).unwrap();
        assert!(emot2.pleasure < 0.5, "positive NPC should drift toward negative");
    }

    #[test]
    fn test_emotional_contagion_stronger_when_closer() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        let e_far = world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Emotion { pleasure: 0.0, arousal: 0.5, control: 0.0 },
        ));
        world.spawn((
            Position(Vec3::new(4.0, 0.0, 0.0)), // 边缘
            sp,
            Emotion { pleasure: 1.0, arousal: 0.5, control: 0.0 },
        ));

        social_system(&mut world, 5.0, 0);
        let emot_far = world.get::<&Emotion>(e_far).unwrap().pleasure;

        let mut world2 = hecs::World::new();
        let e_near = world2.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Emotion { pleasure: 0.0, arousal: 0.5, control: 0.0 },
        ));
        world2.spawn((
            Position(Vec3::new(0.5, 0.0, 0.0)), // 很近
            sp,
            Emotion { pleasure: 1.0, arousal: 0.5, control: 0.0 },
        ));

        social_system(&mut world2, 5.0, 0);
        let emot_near = world2.get::<&Emotion>(e_near).unwrap().pleasure;

        assert!(emot_near > emot_far, "closer NPC should cause stronger contagion");
    }

    #[test]
    fn test_emotional_contagion_clamped() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        let e1 = world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Emotion { pleasure: 0.99, arousal: 0.99, control: 0.99 },
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            Emotion { pleasure: 1.0, arousal: 1.0, control: 1.0 },
        ));

        social_system(&mut world, 100.0, 0);

        let emot = world.get::<&Emotion>(e1).unwrap();
        assert!(emot.pleasure <= 1.0, "pleasure should be clamped to 1.0");
        assert!(emot.arousal <= 1.0, "arousal should be clamped to 1.0");
        assert!(emot.control <= 1.0, "control should be clamped to 1.0");
        assert!(emot.pleasure >= -1.0, "pleasure should be >= -1.0");
    }

    // ── Phase 2: 关系构建 ──

    #[test]
    fn test_relationship_building_familiarity() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Relationships::default(),
            Emotion::default(),
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            Relationships::default(),
            Emotion::default(),
        ));

        social_system(&mut world, 10.0, 100);

        // 双方应建立关系记录
        for (_, rels) in world.query::<&Relationships>().iter() {
            assert!(rels.count > 0, "should have recorded at least one relationship");
            let first = rels.entries.iter().flatten().next().unwrap();
            assert!(first.familiarity > 0.0, "familiarity should have accumulated");
            assert_eq!(first.last_interaction_tick, 100);
        }
    }

    #[test]
    fn test_relationship_liking_from_compatibility() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        // 两个情绪相似的 NPC
        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Relationships::default(),
            Emotion { pleasure: 0.5, arousal: 0.5, control: 0.0 },
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            Relationships::default(),
            Emotion { pleasure: 0.4, arousal: 0.5, control: 0.0 }, // 相似
        ));

        social_system(&mut world, 10.0, 100);

        for (_, rels) in world.query::<&Relationships>().iter() {
            let first = rels.entries.iter().flatten().next().unwrap();
            assert!(first.affection >= 0.0, "similar emotions should produce positive or neutral affection");
        }
    }

    #[test]
    fn test_relationship_trust_grows_slowly() {
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            Relationships::default(),
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            Relationships::default(),
        ));

        social_system(&mut world, 10.0, 100);

        for (_, rels) in world.query::<&Relationships>().iter() {
            let first = rels.entries.iter().flatten().next().unwrap();
            assert!(first.trust > 0.0, "trust should grow slowly");
        }
    }

    #[test]
    fn test_no_relationship_without_component() {
        // 没有 Relationships 组件的实体不应 panic
        let mut world = hecs::World::new();
        let sp = SocialPresence::default();

        world.spawn((Position(Vec3::new(0.0, 0.0, 0.0)), sp));
        world.spawn((Position(Vec3::new(1.0, 0.0, 0.0)), sp));

        social_system(&mut world, 1.0, 0);
        // 不 panic 即为通过
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        social_system(&mut world, 1.0, 0);
    }

    #[test]
    fn test_single_npc_no_panic() {
        let mut world = hecs::World::new();
        world.spawn((
            Position(Vec3::ZERO),
            SocialPresence::default(),
            Needs::default(),
        ));
        social_system(&mut world, 1.0, 0);
    }
}
