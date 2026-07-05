//! NPC 需求 System — 衰减 + 评估
//!
//! HungerDecay: 每帧递增 hunger/thirst/fatigue
//! NeedEvaluation: 计算 urgency → 插入 Desire（urgency > 0.8）

use hecs::CommandBuffer;

use crate::components::needs::{Desire, DesireKind, NeedSensitivity, Needs};

/// 需求基础衰减率（每帧）
const DECAY_RATE: f32 = 0.01;

/// Desire 触发阈值
const URGENCY_THRESHOLD: f32 = 0.8;

/// 需求临界值（达到此值 = urgency 1.0）
const CRITICAL_HUNGER: f32 = 1.0;
const CRITICAL_THIRST: f32 = 1.0;
const CRITICAL_FATIGUE: f32 = 1.0;

/// 每帧递增生理需求
pub fn hunger_decay_system(world: &mut hecs::World) {
    for (_entity, needs) in world.query_mut::<&mut Needs>() {
        needs.hunger = (needs.hunger + DECAY_RATE).min(1.0);
        needs.thirst = (needs.thirst + DECAY_RATE).min(1.0);
        needs.fatigue = (needs.fatigue + DECAY_RATE).min(1.0);
    }
}

/// 评估需求紧急性——任意 urgency > 0.8 则插入 Desire
pub fn need_evaluation_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (entity, (needs, sens)) in world.query::<(&Needs, &NeedSensitivity)>().iter() {
        let hunger_u = (needs.hunger / CRITICAL_HUNGER).clamp(0.0, 1.0) * sens.hunger_sens;
        let thirst_u = (needs.thirst / CRITICAL_THIRST).clamp(0.0, 1.0) * sens.thirst_sens;
        let fatigue_u = (needs.fatigue / CRITICAL_FATIGUE).clamp(0.0, 1.0) * sens.fatigue_sens;

        let (kind, urgency) = if hunger_u >= thirst_u && hunger_u >= fatigue_u {
            (DesireKind::Eat, hunger_u)
        } else if thirst_u >= fatigue_u {
            (DesireKind::Drink, thirst_u)
        } else {
            (DesireKind::Rest, fatigue_u)
        };

        if urgency >= URGENCY_THRESHOLD {
            // 移除旧 Desire（如果存在），插入新 Desire
            cmd.remove_one::<Desire>(entity);
            cmd.insert_one(entity, Desire { kind, urgency });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunger_decay_increments() {
        let mut world = hecs::World::new();
        let e = world.spawn((Needs::default(),));

        hunger_decay_system(&mut world);

        let needs = world.get::<&Needs>(e).expect("has Needs");
        assert!(needs.hunger > 0.0);
        assert!(needs.thirst > 0.0);
        assert!(needs.fatigue > 0.0);
    }

    #[test]
    fn test_hunger_decay_caps_at_one() {
        let mut world = hecs::World::new();
        world.spawn((Needs { hunger: 0.999, thirst: 0.999, fatigue: 0.999 },));

        hunger_decay_system(&mut world);

        for (_, needs) in world.query::<&Needs>().iter() {
            assert!(needs.hunger <= 1.0);
            assert!(needs.thirst <= 1.0);
            assert!(needs.fatigue <= 1.0);
        }
    }

    #[test]
    fn test_need_evaluation_inserts_desire_above_threshold() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((
            Needs { hunger: 0.85, thirst: 0.1, fatigue: 0.1 },
            NeedSensitivity::default(),
        ));

        need_evaluation_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let desire = world.get::<&Desire>(e).expect("should have Desire");
        assert_eq!(desire.kind, DesireKind::Eat);
        assert!(desire.urgency >= URGENCY_THRESHOLD);
    }

    #[test]
    fn test_need_evaluation_no_desire_below_threshold() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((
            Needs { hunger: 0.3, thirst: 0.2, fatigue: 0.1 },
            NeedSensitivity::default(),
        ));

        need_evaluation_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // 未达阈值——不应插入 Desire
        assert!(world.get::<&Desire>(e).is_err());
    }

    #[test]
    fn test_need_evaluation_picks_highest_urgency() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((
            Needs { hunger: 0.3, thirst: 0.5, fatigue: 0.9 }, // fatigue highest
            NeedSensitivity::default(),
        ));

        need_evaluation_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let desire = world.get::<&Desire>(e).expect("should have Desire");
        assert_eq!(desire.kind, DesireKind::Rest);
    }

    #[test]
    fn test_need_evaluation_empty_world() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        need_evaluation_system(&world, &mut cmd);
        cmd.run_on(&mut world);
    }
}
