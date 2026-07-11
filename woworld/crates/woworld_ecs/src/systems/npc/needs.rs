//! NPC 需求 System — 衰减（含跨需求耦合）+ 评估
//!
//! 参见: `开发文档/02-NPC核心/02-基本需求.md`

use hecs::CommandBuffer;

use crate::components::circadian::{circadian_factor, Chronotype};
use crate::components::needs::{Desire, DesireKind, NeedSensitivity, Needs};

// ── 调优参数 (设计文档 §9) ──────────

const DECAY_RATE: f32 = 0.01; // 基础衰减率 / 帧
/// 需求紧迫度阈值——最高 urgency ≥ 此值触发 Desire（设计 §9）。
/// 公开供 goal_resolution 的漫游回落复用同一判据。
pub const URGENCY_THRESHOLD: f32 = 0.8;
const SLEEP_DEBT_HUNGER_MOD: f32 = 0.3; // 疲劳放大饥饿
const THIRST_FATIGUE_MOD: f32 = 0.25; // 脱水放大疲惫
const SOCIAL_ALL_URGENCY_MOD: f32 = 1.15; // 社交缺失放大全部 urgency
const SOCIAL_ACCUM_RATE: f32 = 0.0003; // ~0.02/天 @ 60fps
const SAFETY_DECAY_RATE: f32 = 0.002; // 安全感衰减（比生理慢）
const ELEMENT_DECAY_RATE: f32 = 0.005; // 元素平衡衰减
const LIBIDO_DECAY_RATE: f32 = 0.003; // 性欲衰减

// ── NeedsDecaySystem ──────────────────

/// 每帧递增所有需求维度（含跨需求耦合 + 昼夜节律调制）
///
/// `day_progress`: 可选日内时间 0-1（0=午夜），用于昼夜节律调制
pub fn needs_decay_system(world: &mut hecs::World, day_progress: Option<f32>) {
    for (_entity, (needs, chrono)) in world.query_mut::<(&mut Needs, &Chronotype)>() {
        let circ = day_progress
            .map(|dp| circadian_factor(*chrono, dp))
            .unwrap_or(1.0);

        // 跨需求耦合 (设计文档 §3.3):
        let fatigue_mod = if needs.fatigue > 0.3 {
            1.0 + SLEEP_DEBT_HUNGER_MOD
        } else {
            1.0
        };
        let thirst_fatigue = if needs.thirst > 0.6 {
            1.0 + THIRST_FATIGUE_MOD
        } else {
            1.0
        };

        // 生理需求受昼夜节律影响；心理/周期需求不受影响
        needs.hunger = (needs.hunger + DECAY_RATE * fatigue_mod * circ).min(1.0);
        needs.thirst = (needs.thirst + DECAY_RATE * circ).min(1.0);
        needs.fatigue = (needs.fatigue + DECAY_RATE * thirst_fatigue * circ).min(1.0);
        needs.safety = (needs.safety + SAFETY_DECAY_RATE).min(1.0);
        needs.social = (needs.social + SOCIAL_ACCUM_RATE).min(1.0);
        needs.element_balance = (needs.element_balance + ELEMENT_DECAY_RATE).min(1.0);
        needs.libido = (needs.libido + LIBIDO_DECAY_RATE).min(1.0);
    }
}

// ── NeedEvaluationSystem ──────────────

/// 需求基线——低于此值不产生 urgency（设计文档 §3.1）
const BASELINE: f32 = 0.0;

/// 各维度临界值（cap 在 Needs 中为 1.0）
fn urgency_for(value: f32, sensitivity: f32) -> f32 {
    let deviation = ((value - BASELINE) / (1.0 - BASELINE)).clamp(0.0, 1.0);
    deviation * sensitivity
}

/// 计算最高紧迫需求（含 social 耦合放大）——need_evaluation 与 goal_resolution 共用。
///
/// 返回 (最紧迫的 DesireKind, urgency)。`urgency < URGENCY_THRESHOLD` 视为无紧迫需求。
pub fn evaluate_top_urgency(needs: &Needs, sens: &NeedSensitivity) -> (DesireKind, f32) {
    let urgencies = [
        (DesireKind::Eat, urgency_for(needs.hunger, sens.hunger_sens)),
        (
            DesireKind::Drink,
            urgency_for(needs.thirst, sens.thirst_sens),
        ),
        (
            DesireKind::Rest,
            urgency_for(needs.fatigue, sens.fatigue_sens),
        ),
        (
            DesireKind::SeekSafety,
            urgency_for(needs.safety, sens.safety_sens),
        ),
        (
            DesireKind::Socialize,
            urgency_for(needs.social, sens.social_sens),
        ),
        (
            DesireKind::BalanceElements,
            urgency_for(needs.element_balance, sens.element_sens),
        ),
        (
            DesireKind::ExpressLibido,
            urgency_for(needs.libido, sens.libido_sens),
        ),
    ];

    // 选最高 urgency
    let (mut kind, mut urgency) = urgencies[0];
    for &(k, u) in &urgencies[1..] {
        if u > urgency {
            kind = k;
            urgency = u;
        }
    }

    // 跨需求耦合: social > 0.7 使所有 urgency × 1.15
    if needs.social > 0.7 {
        urgency *= SOCIAL_ALL_URGENCY_MOD;
    }

    (kind, urgency)
}

/// 评估需求紧急性——最高 urgency ≥ 阈值则插入 Desire
pub fn need_evaluation_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (entity, (needs, sens)) in world.query::<(&Needs, &NeedSensitivity)>().iter() {
        let (kind, urgency) = evaluate_top_urgency(needs, sens);
        if urgency >= URGENCY_THRESHOLD {
            cmd.remove_one::<Desire>(entity);
            cmd.insert_one(entity, Desire { kind, urgency });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_increments_all_seven() {
        let mut world = hecs::World::new();
        let e = world.spawn((Needs::default(), Chronotype::default()));
        needs_decay_system(&mut world, None);
        let n = world.get::<&Needs>(e).unwrap();
        assert!(n.hunger > 0.0);
        assert!(n.social > 0.0);
        assert!(n.libido > 0.0);
    }

    #[test]
    fn test_fatigue_amplifies_hunger() {
        let mut world = hecs::World::new();
        // 疲劳 > 0.3 → 饥饿衰减 × 1.3
        let e1 = world.spawn((
            Needs {
                fatigue: 0.5,
                ..Needs::default()
            },
            Chronotype::default(),
        ));
        let e2 = world.spawn((
            Needs {
                fatigue: 0.0,
                ..Needs::default()
            },
            Chronotype::default(),
        ));

        needs_decay_system(&mut world, None);

        let n1 = world.get::<&Needs>(e1).unwrap();
        let n2 = world.get::<&Needs>(e2).unwrap();
        assert!(n1.hunger > n2.hunger, "fatigue should amplify hunger decay");
    }

    #[test]
    fn test_thirst_amplifies_fatigue() {
        let mut world = hecs::World::new();
        // 口渴 > 0.6 → 疲劳衰减 × 1.25
        let e1 = world.spawn((
            Needs {
                thirst: 0.8,
                ..Needs::default()
            },
            Chronotype::default(),
        ));
        let e2 = world.spawn((
            Needs {
                thirst: 0.0,
                ..Needs::default()
            },
            Chronotype::default(),
        ));

        needs_decay_system(&mut world, None);

        let n1 = world.get::<&Needs>(e1).unwrap();
        let n2 = world.get::<&Needs>(e2).unwrap();
        assert!(
            n1.fatigue > n2.fatigue,
            "thirst should amplify fatigue decay"
        );
    }

    #[test]
    fn test_social_amplifies_all_urgency() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // 高社交缺失 + 中等饥饿 → 饥饿 urgency 被放大
        let e = world.spawn((
            Needs {
                hunger: 0.75,
                social: 0.8,
                ..Needs::default()
            },
            NeedSensitivity::default(),
        ));

        need_evaluation_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let desire = world.get::<&Desire>(e).expect("should have Desire");
        // baseline hunger urgency = 0.75, social amplification × 1.15 = 0.8625
        assert!(desire.urgency > 0.8);
    }

    #[test]
    fn test_urgency_formula_with_baseline() {
        // urgency = (current - baseline) / (critical - baseline) * sensitivity
        // = (0.5 - 0.0) / (1.0 - 0.0) * 1.0 = 0.5
        assert_eq!(urgency_for(0.5, 1.0), 0.5);
        assert_eq!(urgency_for(0.0, 1.0), 0.0); // baseline → 0
        assert_eq!(urgency_for(1.0, 1.0), 1.0); // critical → 1
        assert_eq!(urgency_for(0.5, 2.0), 1.0); // sensitivity 加倍
    }

    #[test]
    fn test_seven_dimensions_evaluated() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // 所有维度都低——不应触发
        let e = world.spawn((Needs::default(), NeedSensitivity::default()));
        need_evaluation_system(&world, &mut cmd);
        cmd.run_on(&mut world);
        assert!(world.get::<&Desire>(e).is_err());

        // safety 高——应触发 SeekSafety
        let mut cmd2 = CommandBuffer::new();
        world
            .insert_one(
                e,
                Needs {
                    safety: 0.9,
                    ..Needs::default()
                },
            )
            .unwrap();
        need_evaluation_system(&world, &mut cmd2);
        cmd2.run_on(&mut world);
        let desire = world.get::<&Desire>(e).expect("should have Desire");
        assert_eq!(desire.kind, DesireKind::SeekSafety);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        need_evaluation_system(&world, &mut cmd);
        needs_decay_system(&mut hecs::World::new(), None);
    }
}
