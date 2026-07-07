//! 社交 System — 8 因子情绪传染 + 关系构建 + 社交需求恢复
//!
//! 参见: `开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0.md` §2.1.5 (情绪感染) + §2.4 (关系系统)
//!
//! Phase 3: SpatialGrid 加速替代 O(n²)。

use crate::components::bigfive::BigFive;
use crate::components::emotion::Emotion;
use crate::components::needs::Needs;
use crate::components::social::{RelationHandle, SocialPresence};
use crate::components::transform::Position;
use crate::resources::relation_storage::RelationStorage;
use woworld_core::types::EntityId;

/// 情绪传染最大距离（文档 §2.1.5: "仅 L1 视野内计算，15m"）
const CONTAGION_MAX_DISTANCE: f32 = 15.0;
/// 关系构建基础速率——每秒 familiarity 增量
const FAMILIARITY_GAIN_RATE: f32 = 0.02;
/// affection 调整速率——每秒从情绪兼容性调整 affection 的比例
const AFFECTION_ADJUST_RATE: f32 = 0.01;

/// 8 重调节因子情绪传染——对齐文档 §2.1.5 `calculate_emotional_contagion()`
///
/// 返回 [0, 1] 的传染强度乘数。调用方将此乘数应用于 PAD 漂移量。
///
/// # 因子
/// 1. 表达强度: `expressed_emotion.arousal × 0.4`
/// 2. 关系亲密度: `0.1 + intimacy × 0.9`
/// 3. 社会地位梯度: Phase 2 — 硬编码 1.0
/// 4. 群体相似性: Phase 2 — 硬编码 1.0
/// 5. 观察者人格: 宜人性>0.7 → 1.4x, 神经质>0.7+负面 → 1.3x
/// 6. 先前情绪一致性: 同向 → 1.2x
/// 7. 距离衰减: `1.0 - distance / 15.0`
/// 8. 最大距离 15m
pub fn calculate_emotional_contagion(
    expressor_emotion: &Emotion,
    observer_emotion: &Emotion,
    observer_bigfive: &BigFive,
    intimacy: f32, // 来自关系存储的 affection
    distance: f32,
) -> f32 {
    if distance > CONTAGION_MAX_DISTANCE {
        return 0.0;
    }

    // 1. 表达强度——唤醒度越高，情绪越"传染"
    let mut strength = expressor_emotion.arousal * 0.4;

    // 2. 关系亲密度——配偶 0.9-1.0，陌生人 0.1
    strength *= 0.1 + intimacy * 0.9;

    // 3. 社会地位梯度 — Phase 2: 权力系统就位后接入
    // strength *= if status_diff > 0.0 { 1.5 } else { 0.5 };

    // 4. 群体相似性 — Phase 2: Faction/Group 系统就位后接入
    // strength *= 0.7 + similarity * 0.6;

    // 5. 观察者人格
    if observer_bigfive.agreeableness > 0.7 {
        strength *= 1.4;
    }
    if observer_bigfive.neuroticism > 0.7 && expressor_emotion.pleasure < 0.0 {
        strength *= 1.3;
    }

    // 6. 先前情绪一致性——同向共振
    if (observer_emotion.pleasure * expressor_emotion.pleasure) > 0.0 {
        strength *= 1.2;
    }

    // 7. 距离衰减
    strength *= 1.0 - (distance / CONTAGION_MAX_DISTANCE).clamp(0.0, 1.0);

    strength.clamp(0.0, 1.0)
}

/// 社交系统——近邻 NPC 互动
///
/// `current_tick`: 用于标记关系最近互动时间（来自 WorldDriver.frame_count）
/// `storage`: 全局关系存储
pub fn social_system(
    world: &mut hecs::World,
    dt: f32,
    current_tick: u64,
    storage: &mut RelationStorage,
) {
    // 收集所有需要参与社交的实体
    struct Entry {
        entity: hecs::Entity,
        pos: Position,
        presence: SocialPresence,
        bigfive: BigFive,
        emotion: Emotion,
    }

    let entries: Vec<Entry> = world
        .query::<(
            &Position,
            &SocialPresence,
            &BigFive,
            &Emotion,
            &RelationHandle,
        )>()
        .iter()
        .map(|(e, (pos, sp, bf, emot, _))| Entry {
            entity: e,
            pos: *pos,
            presence: *sp,
            bigfive: *bf,
            emotion: *emot,
        })
        .collect();

    // 延迟写入避免借用冲突
    struct Modification {
        entity: hecs::Entity,
        social_recovery: f32,
        pleasure_delta: f32,
        arousal_delta: f32,
        control_delta: f32,
        partner_id: EntityId,
        touch_relation: bool,
    }

    let mut modifications: Vec<Modification> = Vec::new();
    // 收集本次触摸的关系对，用于后续衰减未活跃的关系
    let mut touched_pairs: Vec<(EntityId, EntityId)> = Vec::new();

    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let a = &entries[i];
            let b = &entries[j];

            let dx = a.pos.0.x - b.pos.0.x;
            let dz = a.pos.0.z - b.pos.0.z;
            let dist = (dx * dx + dz * dz).sqrt();

            // ── 区分两个半径 ──
            // 社交需求恢复: social radius (2-7m, 从 BigFive 派生)
            // 情绪传染: CONTAGION_MAX_DISTANCE (15m, 文档 §2.1.5)
            let social_radius = a.presence.radius.min(b.presence.radius);
            let in_social_range = dist < social_radius;
            let in_contagion_range = dist < CONTAGION_MAX_DISTANCE;

            if !in_social_range && !in_contagion_range {
                continue;
            }

            let id_a = EntityId(a.entity.to_bits().get());
            let id_b = EntityId(b.entity.to_bits().get());

            // ── 社交需求恢复（仅在 social radius 内）──
            let social_rec_a = if in_social_range {
                let proximity = 1.0 - dist / social_radius;
                b.presence.recovery_rate * proximity * dt
            } else {
                0.0
            };
            let social_rec_b = if in_social_range {
                let proximity = 1.0 - dist / social_radius;
                a.presence.recovery_rate * proximity * dt
            } else {
                0.0
            };

            // ── 情绪传染（8 因子，15m 范围）──
            let (pd_a, ad_a, cd_a, pd_b, ad_b, cd_b) = if in_contagion_range {
                let intimacy_ab = storage
                    .get(id_a, id_b)
                    .map(|r| r.affection)
                    .unwrap_or(0.0);

                let contagion_a = calculate_emotional_contagion(
                    &b.emotion, &a.emotion, &a.bigfive, intimacy_ab, dist,
                );
                let contagion_b = calculate_emotional_contagion(
                    &a.emotion, &b.emotion, &b.bigfive, intimacy_ab, dist,
                );

                let contagion_rate = 0.05 * dt;
                (
                    (b.emotion.pleasure - a.emotion.pleasure) * contagion_a * contagion_rate,
                    (b.emotion.arousal - a.emotion.arousal) * contagion_a * contagion_rate,
                    (b.emotion.control - a.emotion.control) * contagion_a * contagion_rate,
                    (a.emotion.pleasure - b.emotion.pleasure) * contagion_b * contagion_rate,
                    (a.emotion.arousal - b.emotion.arousal) * contagion_b * contagion_rate,
                    (a.emotion.control - b.emotion.control) * contagion_b * contagion_rate,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            };

            // 关系更新仅在 social radius 内（有实际互动）
            let touch = in_social_range;

            modifications.push(Modification {
                entity: a.entity,
                social_recovery: social_rec_a,
                pleasure_delta: pd_a,
                arousal_delta: ad_a,
                control_delta: cd_a,
                partner_id: id_b,
                touch_relation: touch,
            });
            modifications.push(Modification {
                entity: b.entity,
                social_recovery: social_rec_b,
                pleasure_delta: pd_b,
                arousal_delta: ad_b,
                control_delta: cd_b,
                partner_id: id_a,
                touch_relation: touch,
            });

            if touch {
                touched_pairs.push((id_a, id_b));
            }
        }
    }

    // ── 应用修改 ──
    for m in &modifications {
        // 社交需求
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
        if m.touch_relation {
            let own_id = EntityId(m.entity.to_bits().get());
            let rel = storage.get_or_create(own_id, m.partner_id);
            // 熟悉度累积
            let proximity = 1.0; // 已在上面验证过在范围内，此处简化
            let fam_gain = FAMILIARITY_GAIN_RATE * proximity * dt;
            rel.familiarity = (rel.familiarity + fam_gain).min(1.0);
            // affection 从情绪兼容性调整
            // Phase 2: 从双方 Entity 查 Emotion 计算精确兼容性
            // 当前使用中性兼容性 (0.5) —— 情绪兼容性已通过 contagion 间接影响
            let compat = 0.5;
            let affection_delta = (compat - 0.5) * AFFECTION_ADJUST_RATE * dt;
            rel.affection = (rel.affection + affection_delta).clamp(-1.0, 1.0);
            rel.total_interactions = rel.total_interactions.saturating_add(1);
            rel.last_interaction_tick = current_tick;
        }
    }

    // ── 关系维护：衰减 + 信任增长 ──
    // 每 ~30 秒实际时间运行一次（约 900 ticks @30fps），避免每帧全表扫描
    // Phase 2: 接入 WorldClock 做 tick→天 精确换算
    const MAINTENANCE_INTERVAL_TICKS: u64 = 900;
    if current_tick - storage.last_maintenance_tick >= MAINTENANCE_INTERVAL_TICKS {
        // 粗略 tick→天 换算：假设 30fps, 1天=86400秒 → 1tick≈1/2592000 天
        // 实际应通过 WorldClock.seconds_per_day 换算

        for (_key, rel) in storage.relations.iter_mut() {
            // 仅维护有互动历史的活跃关系
            if rel.total_interactions == 0 {
                continue;
            }
            // 计算距上次互动的天数
            let ticks_since_interaction =
                (current_tick.saturating_sub(rel.last_interaction_tick)) as f32;
            let days_since = ticks_since_interaction / 2592000.0;
            rel.decay_affection(days_since);
            rel.grow_trust_over_time(days_since);
        }

        storage.last_maintenance_tick = current_tick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    // ── 8 因子情绪传染测试 ──

    #[test]
    fn test_contagion_beyond_max_distance_is_zero() {
        let em = Emotion::default();
        let bf = BigFive::default();
        let s = calculate_emotional_contagion(&em, &em, &bf, 0.5, 20.0);
        assert_eq!(s, 0.0, "beyond 15m should be zero");
    }

    #[test]
    fn test_contagion_intimacy_boosts() {
        let em = Emotion {
            arousal: 1.0,
            ..Emotion::default()
        };
        let bf = BigFive::default();
        let s_stranger = calculate_emotional_contagion(&em, &em, &bf, 0.0, 1.0);
        let s_spouse = calculate_emotional_contagion(&em, &em, &bf, 1.0, 1.0);
        assert!(
            s_spouse > s_stranger,
            "spouse intimacy should boost contagion"
        );
    }

    #[test]
    fn test_contagion_agreeable_more_affected() {
        let em = Emotion {
            arousal: 1.0,
            ..Emotion::default()
        };
        let bf_agreeable = BigFive {
            agreeableness: 0.9,
            ..BigFive::default()
        };
        let bf_disagreeable = BigFive {
            agreeableness: 0.3,
            ..BigFive::default()
        };
        let s_high = calculate_emotional_contagion(&em, &em, &bf_agreeable, 0.5, 1.0);
        let s_low = calculate_emotional_contagion(&em, &em, &bf_disagreeable, 0.5, 1.0);
        assert!(s_high > s_low, "agreeable people should be more affected");
    }

    #[test]
    fn test_contagion_neurotic_negative_emotion() {
        let em_negative = Emotion {
            pleasure: -0.5,
            arousal: 1.0,
            ..Emotion::default()
        };
        let bf_neurotic = BigFive {
            neuroticism: 0.9,
            ..BigFive::default()
        };
        let bf_stable = BigFive {
            neuroticism: 0.3,
            ..BigFive::default()
        };
        let s_neurotic =
            calculate_emotional_contagion(&em_negative, &em_negative, &bf_neurotic, 0.5, 1.0);
        let s_stable =
            calculate_emotional_contagion(&em_negative, &em_negative, &bf_stable, 0.5, 1.0);
        assert!(
            s_neurotic > s_stable,
            "neurotic people should be more affected by negative emotions"
        );
    }

    #[test]
    fn test_contagion_same_valence_resonance() {
        let em_pos = Emotion {
            pleasure: 0.5,
            arousal: 1.0,
            ..Emotion::default()
        };
        let em_neg = Emotion {
            pleasure: -0.5,
            arousal: 1.0,
            ..Emotion::default()
        };
        let bf = BigFive::default();
        let s_same = calculate_emotional_contagion(&em_pos, &em_pos, &bf, 0.5, 1.0);
        let s_opposite = calculate_emotional_contagion(&em_pos, &em_neg, &bf, 0.5, 1.0);
        assert!(
            s_same > s_opposite,
            "same valence should resonate (1.2x boost)"
        );
    }

    #[test]
    fn test_contagion_distance_decay() {
        let em = Emotion {
            arousal: 1.0,
            ..Emotion::default()
        };
        let bf = BigFive::default();
        let s_near = calculate_emotional_contagion(&em, &em, &bf, 0.5, 1.0);
        let s_far = calculate_emotional_contagion(&em, &em, &bf, 0.5, 10.0);
        assert!(s_near > s_far, "closer should have stronger contagion");
    }

    #[test]
    fn test_contagion_clamped_0_1() {
        let em = Emotion {
            arousal: 1.0,
            ..Emotion::default()
        };
        let bf = BigFive::default();
        let s = calculate_emotional_contagion(&em, &em, &bf, 1.0, 0.1);
        assert!(s >= 0.0 && s <= 1.0, "contagion should be clamped to [0, 1]");
    }

    // ── 社交需求恢复 ──

    #[test]
    fn test_close_npcs_recover_social() {
        let mut world = hecs::World::new();
        let mut storage = RelationStorage::default();
        let sp = SocialPresence::default();
        let bf = BigFive::default();
        let em = Emotion::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs {
                social: 0.6,
                ..Needs::default()
            },
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs {
                social: 0.6,
                ..Needs::default()
            },
        ));

        social_system(&mut world, 1.0, 0, &mut storage);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!(needs.social < 0.6, "social should decrease");
            assert!(needs.social >= 0.0, "should not go negative");
        }
    }

    #[test]
    fn test_distant_npcs_no_effect() {
        let mut world = hecs::World::new();
        let mut storage = RelationStorage::default();
        let sp = SocialPresence::default();
        let bf = BigFive::default();
        let em = Emotion::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs {
                social: 0.6,
                ..Needs::default()
            },
        ));
        world.spawn((
            Position(Vec3::new(20.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs {
                social: 0.6,
                ..Needs::default()
            },
        ));

        social_system(&mut world, 1.0, 0, &mut storage);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!((needs.social - 0.6).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_relationship_building() {
        let mut world = hecs::World::new();
        let mut storage = RelationStorage::default();
        let sp = SocialPresence::default();
        let bf = BigFive::default();
        let em = Emotion::default();

        world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs::default(),
        ));
        world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs::default(),
        ));

        social_system(&mut world, 10.0, 100, &mut storage);

        // 应创建关系记录
        assert!(!storage.relations.is_empty(), "should create relationship");
    }

    #[test]
    fn test_relationship_familiarity_accumulates() {
        let mut world = hecs::World::new();
        let mut storage = RelationStorage::default();
        let sp = SocialPresence::default();
        let bf = BigFive::default();
        let em = Emotion::default();

        let e1 = world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs::default(),
        ));
        let e2 = world.spawn((
            Position(Vec3::new(1.0, 0.0, 0.0)),
            sp,
            bf,
            em,
            RelationHandle,
            Needs::default(),
        ));

        social_system(&mut world, 10.0, 100, &mut storage);

        let id1 = EntityId(e1.to_bits().get());
        let id2 = EntityId(e2.to_bits().get());
        let rel = storage.get(id1, id2).unwrap();
        assert!(rel.familiarity > 0.0, "familiarity should accumulate");
        assert!(rel.total_interactions > 0, "should count interactions");
        assert_eq!(rel.last_interaction_tick, 100);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        let mut storage = RelationStorage::default();
        social_system(&mut world, 1.0, 0, &mut storage);
    }

    #[test]
    fn test_single_npc_no_panic() {
        let mut world = hecs::World::new();
        let mut storage = RelationStorage::default();
        world.spawn((
            Position(Vec3::ZERO),
            SocialPresence::default(),
            BigFive::default(),
            Emotion::default(),
            RelationHandle,
            Needs::default(),
        ));
        social_system(&mut world, 1.0, 0, &mut storage);
    }
}
