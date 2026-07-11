//! GoalResolutionSystem — Desire → Goal 转换 + 漫游回落
//!
//! 第一遍：查询 Desire → 映射 GoalType → cmd: remove Desire, insert Goal。
//! 第二遍（R4）：无 Desire 且需求已不紧迫的 NPC → 回落 Goal::Idle（漫游），
//! 修复「Goal sticky」（需求满足后仍卡在旧 FindX）。判据读 Needs 紧迫度防振荡。

use hecs::CommandBuffer;

use crate::components::goal::{Goal, GoalType};
use crate::components::needs::{Desire, DesireKind, NeedSensitivity, Needs};
use crate::systems::npc::needs::{evaluate_top_urgency, URGENCY_THRESHOLD};

/// Desire → GoalType 映射
fn goal_for_desire(kind: DesireKind) -> GoalType {
    match kind {
        DesireKind::Eat => GoalType::FindFood,
        DesireKind::Drink => GoalType::FindWater,
        DesireKind::Rest => GoalType::FindRest,
        DesireKind::SeekSafety => GoalType::FindSafePlace,
        DesireKind::Socialize => GoalType::FindSocialContact,
        DesireKind::BalanceElements => GoalType::BalanceElements,
        DesireKind::ExpressLibido => GoalType::ExpressLibido,
    }
}

/// 每帧执行——Desire → Goal + 漫游回落
pub fn goal_resolution_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    // ── 第一遍：有 Desire → 转 Goal（消费 Desire）──
    for (entity, desire) in world.query::<&Desire>().iter() {
        let goal = Goal {
            goal_type: goal_for_desire(desire.kind),
            urgency: desire.urgency,
            target_pos: None, // Phase 2: 知识 System 填充
        };

        cmd.remove_one::<Desire>(entity);
        cmd.insert_one(entity, goal);
    }

    // ── 第二遍：漫游回落（R4）──
    // 无 Desire 且当前 Goal 非 Idle 的 NPC：最高需求已不紧迫 → 回落 Idle（漫游）。
    // ⚠️ 判据必须读 Needs 紧迫度，不能靠「无 Desire」——Desire 被本系统即时消费，
    //    持续紧迫的 NPC 也存在「当帧无 Desire」窗口（needs/goal 同 Block A4 共用 cmd），
    //    若按「无 Desire → Idle」回落会致 Goal 每帧 FindX↔Idle 振荡。
    for (entity, (goal, needs, sens)) in world
        .query::<(&Goal, &Needs, &NeedSensitivity)>()
        .without::<&Desire>()
        .iter()
    {
        if goal.goal_type == GoalType::Idle {
            continue;
        }
        let (_, urgency) = evaluate_top_urgency(needs, sens);
        if urgency < URGENCY_THRESHOLD {
            cmd.insert_one(entity, Goal::default()); // Idle——漫游
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunger_desire_becomes_find_food() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Desire {
            kind: DesireKind::Eat,
            urgency: 0.9,
        },));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
        assert_eq!(goal.urgency, 0.9);
        // Desire 已被移除
        assert!(world.get::<&Desire>(e).is_err());
    }

    #[test]
    fn test_thirst_desire_becomes_find_water() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Desire {
            kind: DesireKind::Drink,
            urgency: 0.85,
        },));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindWater);
    }

    #[test]
    fn test_fatigue_desire_becomes_find_rest() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Desire {
            kind: DesireKind::Rest,
            urgency: 0.88,
        },));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindRest);
    }

    #[test]
    fn test_no_desire_no_goal() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // 空 World——不 panic
        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);
    }

    // ── R4: 漫游回落 ──

    #[test]
    fn test_satisfied_need_falls_back_to_idle() {
        // 旧 Goal{FindFood}、需求全低(不紧迫)、无 Desire → 回落 Idle（漫游）
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let e = world.spawn((
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: None,
            },
            Needs::default(),
            NeedSensitivity::default(),
        ));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::Idle);
    }

    #[test]
    fn test_urgent_need_no_idle_flicker() {
        // 旧 Goal{FindFood}、饥饿仍紧迫、当帧无 Desire → 不回落（防每帧振荡）
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let e = world.spawn((
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: None,
            },
            Needs {
                hunger: 1.0,
                ..Needs::default()
            },
            NeedSensitivity::default(),
        ));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(
            goal.goal_type,
            GoalType::FindFood,
            "urgent need must not flicker to Idle"
        );
    }

    #[test]
    fn test_desire_entity_takes_first_pass_not_idle() {
        // 同时有 Desire 的实体走第一遍(转 Goal)，第二遍 .without::<Desire>() 排除它
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let e = world.spawn((
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
            Goal::default(),
            Needs::default(),
            NeedSensitivity::default(),
        ));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
    }
}
